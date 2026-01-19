use crate::models::JobUpdate;
use color_eyre::Result;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn convert_for_davinci(
    job_id: Uuid,
    input_path: PathBuf,
    output_dir: PathBuf,
    update_tx: mpsc::UnboundedSender<(Uuid, JobUpdate)>,
) -> Result<PathBuf> {
    // Ensure output directory exists
    tokio::fs::create_dir_all(&output_dir).await?;

    // Generate output filename
    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid input filename"))?;

    let output_path = output_dir.join(format!("{}_davinci.mp4", file_stem));

    // FFmpeg command to convert for DaVinci Resolve compatibility
    // Re-encode video to H.264 with PCM audio to ensure compatibility
    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(&input_path)
        .arg("-c:v")
        .arg("libx264") // Re-encode to H.264 for better compatibility
        .arg("-preset")
        .arg("fast") // Faster encoding while maintaining quality
        .arg("-crf")
        .arg("18") // High quality (lower = better quality, 18 is visually lossless)
        .arg("-c:a")
        .arg("pcm_s16le") // Convert audio to PCM 16-bit little-endian
        .arg("-ar")
        .arg("48000") // Sample rate 48kHz (standard for video)
        .arg("-progress")
        .arg("pipe:1") // Output progress to stdout
        .arg("-y") // Overwrite output file if exists
        .arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let stderr_reader = BufReader::new(stderr).lines();

    // Get video duration first for progress calculation
    let duration = get_video_duration(&input_path).await?;

    // Regex to parse progress output
    let time_regex = Regex::new(r"out_time_ms=(\d+)")?;

    // Read progress output
    let update_tx_clone = update_tx.clone();
    let job_id_clone = job_id;
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Some(caps) = time_regex.captures(&line) {
                if let Ok(time_ms) = caps[1].parse::<u64>() {
                    let time_sec = time_ms / 1_000_000;
                    if duration > 0 {
                        let percent = (time_sec as f64 / duration as f64 * 100.0).min(100.0);
                        let _ = update_tx_clone.send((job_id_clone, JobUpdate::Progress(percent)));
                    }
                }
            }
        }
    });

    // Capture stderr for errors
    let mut stderr_output = Vec::new();
    let mut stderr_lines = stderr_reader;
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        stderr_output.push(line);
    }

    // Wait for process to complete
    let status = child.wait().await?;

    if !status.success() {
        let error_msg = stderr_output.join("\n");
        return Err(color_eyre::eyre::eyre!(
            "FFmpeg conversion failed: {}",
            error_msg
        ));
    }

    // Delete the original temp file
    let _ = tokio::fs::remove_file(&input_path).await;

    Ok(output_path)
}

async fn get_video_duration(path: &PathBuf) -> Result<u64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .await?;

    if output.status.success() {
        let duration_str = String::from_utf8_lossy(&output.stdout);
        let duration: f64 = duration_str.trim().parse().unwrap_or(0.0);
        Ok(duration as u64)
    } else {
        Ok(0)
    }
}

// Alternative: Convert to DNxHD for even better DaVinci Resolve compatibility
#[allow(dead_code)]
pub async fn convert_to_dnxhd(
    job_id: Uuid,
    input_path: PathBuf,
    output_dir: PathBuf,
    update_tx: mpsc::UnboundedSender<(Uuid, JobUpdate)>,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(&output_dir).await?;

    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid input filename"))?;

    let output_path = output_dir.join(format!("{}_dnxhd.mov", file_stem));

    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(&input_path)
        .arg("-c:v")
        .arg("dnxhd") // DNxHD codec
        .arg("-profile:v")
        .arg("dnxhr_hq") // High quality profile
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("-ar")
        .arg("48000")
        .arg("-progress")
        .arg("pipe:1")
        .arg("-y")
        .arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let stderr_reader = BufReader::new(stderr).lines();

    let duration = get_video_duration(&input_path).await?;
    let time_regex = Regex::new(r"out_time_ms=(\d+)")?;

    let update_tx_clone = update_tx.clone();
    let job_id_clone = job_id;
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Some(caps) = time_regex.captures(&line) {
                if let Ok(time_ms) = caps[1].parse::<u64>() {
                    let time_sec = time_ms / 1_000_000;
                    if duration > 0 {
                        let percent = (time_sec as f64 / duration as f64 * 100.0).min(100.0);
                        let _ = update_tx_clone.send((job_id_clone, JobUpdate::Progress(percent)));
                    }
                }
            }
        }
    });

    let mut stderr_output = Vec::new();
    let mut stderr_lines = stderr_reader;
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        stderr_output.push(line);
    }

    let status = child.wait().await?;

    if !status.success() {
        let error_msg = stderr_output.join("\n");
        return Err(color_eyre::eyre::eyre!(
            "FFmpeg conversion failed: {}",
            error_msg
        ));
    }

    let _ = tokio::fs::remove_file(&input_path).await;

    Ok(output_path)
}
