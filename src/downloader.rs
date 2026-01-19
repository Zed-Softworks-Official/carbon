use crate::models::JobUpdate;
use color_eyre::Result;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct DownloadProgress {
    pub percent: f64,
    pub speed: Option<String>,
    pub eta: Option<String>,
}

pub async fn download_video(
    job_id: Uuid,
    url: String,
    quality: String,
    output_dir: PathBuf,
    update_tx: mpsc::UnboundedSender<(Uuid, JobUpdate)>,
) -> Result<(String, PathBuf)> {
    // Create temp directory for downloads
    let temp_dir = output_dir.join(".temp");
    tokio::fs::create_dir_all(&temp_dir).await?;

    // Build output template
    let output_template = temp_dir.join("%(title)s.%(ext)s");

    // Build quality format string
    // Use merge-output-format to ensure video and audio are properly merged
    let format = match quality.as_str() {
        "best" => "bestvideo+bestaudio/best",
        "1080p" => "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
        "720p" => "bestvideo[height<=720]+bestaudio/best[height<=720]",
        "480p" => "bestvideo[height<=480]+bestaudio/best[height<=480]",
        _ => "bestvideo+bestaudio/best",
    };

    // Spawn yt-dlp process
    let mut child = Command::new("yt-dlp")
        .arg("-f")
        .arg(format)
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--newline")
        .arg("--no-playlist")
        .arg("-o")
        .arg(output_template.to_string_lossy().to_string())
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Regex patterns for parsing progress
    let progress_regex = Regex::new(r"\[download\]\s+(\d+\.?\d*)%")?;
    let speed_regex = Regex::new(r"at\s+(\S+/s)")?;
    let eta_regex = Regex::new(r"ETA\s+(\S+)")?;
    let destination_regex = Regex::new(r"\[download\] Destination: (.+)")?;

    let mut title: Option<String> = None;
    let mut output_path: Option<PathBuf> = None;

    // Read output in background
    let update_tx_clone = update_tx.clone();
    let job_id_clone = job_id;
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Some(caps) = progress_regex.captures(&line) {
                if let Ok(percent) = caps[1].parse::<f64>() {
                    let _ = update_tx_clone.send((job_id_clone, JobUpdate::Progress(percent)));
                }
            }

            if let Some(caps) = speed_regex.captures(&line) {
                let speed = caps[1].to_string();
                let _ = update_tx_clone.send((job_id_clone, JobUpdate::Speed(speed)));
            }

            if let Some(caps) = eta_regex.captures(&line) {
                let eta = caps[1].to_string();
                let _ = update_tx_clone.send((job_id_clone, JobUpdate::Eta(eta)));
            }

            if let Some(caps) = destination_regex.captures(&line) {
                let path = PathBuf::from(&caps[1]);
                let _ = update_tx_clone.send((job_id_clone, JobUpdate::TempPath(path)));
            }
        }
    });

    // Capture stderr for errors and title
    let mut stderr_output = Vec::new();
    while let Ok(Some(line)) = stderr_reader.next_line().await {
        stderr_output.push(line.clone());

        // Try to extract title from stderr
        if title.is_none() && line.contains("[info]") {
            // yt-dlp sometimes outputs title in stderr
            continue;
        }
    }

    // Wait for process to complete
    let status = child.wait().await?;

    if !status.success() {
        let error_msg = stderr_output.join("\n");
        return Err(color_eyre::eyre::eyre!("yt-dlp failed: {}", error_msg));
    }

    // Find the downloaded file
    let mut entries = tokio::fs::read_dir(&temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video")
                .to_string();

            if title.is_none() {
                title = Some(file_name.clone());
                let _ = update_tx.send((job_id, JobUpdate::Title(file_name)));
            }

            output_path = Some(path);
            break;
        }
    }

    let output_path =
        output_path.ok_or_else(|| color_eyre::eyre::eyre!("Downloaded file not found"))?;
    let title = title.unwrap_or_else(|| "Unknown".to_string());

    Ok((title, output_path))
}

// Function to get video info without downloading
pub async fn get_video_info(url: &str) -> Result<String> {
    let output = Command::new("yt-dlp")
        .arg("--get-title")
        .arg("--no-playlist")
        .arg(url)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(color_eyre::eyre::eyre!("Failed to get video info"))
    }
}
