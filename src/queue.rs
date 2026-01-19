use crate::converter::convert_for_davinci;
use crate::downloader::download_video;
use crate::models::{Config, JobStatus, JobUpdate};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

pub struct JobQueue {
    semaphore: Arc<Semaphore>,
    update_tx: mpsc::UnboundedSender<(Uuid, JobUpdate)>,
    config: Config,
}

impl JobQueue {
    pub fn new(
        max_concurrent: usize,
        update_tx: mpsc::UnboundedSender<(Uuid, JobUpdate)>,
        config: Config,
    ) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            update_tx,
            config,
        }
    }

    pub fn start_job(&self, job_id: Uuid, url: String, quality: String) {
        let semaphore = self.semaphore.clone();
        let update_tx = self.update_tx.clone();
        let output_dir = PathBuf::from(&self.config.output_directory);
        let auto_convert = self.config.auto_convert;

        tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await.unwrap();

            // Update status to Downloading
            let _ = update_tx.send((job_id, JobUpdate::Status(JobStatus::Downloading)));
            let _ = update_tx.send((job_id, JobUpdate::Progress(0.0)));

            // Download video
            let download_result = download_video(
                job_id,
                url.clone(),
                quality,
                output_dir.clone(),
                update_tx.clone(),
            )
            .await;

            match download_result {
                Ok((title, temp_path)) => {
                    // Update title if we got it
                    let _ = update_tx.send((job_id, JobUpdate::Title(title)));

                    if auto_convert {
                        // Update status to Converting
                        let _ = update_tx.send((job_id, JobUpdate::Status(JobStatus::Converting)));
                        let _ = update_tx.send((job_id, JobUpdate::Progress(0.0)));

                        // Convert video
                        let convert_result = convert_for_davinci(
                            job_id,
                            temp_path.clone(),
                            output_dir.clone(),
                            update_tx.clone(),
                        )
                        .await;

                        match convert_result {
                            Ok(output_path) => {
                                // Update status to Complete
                                let _ =
                                    update_tx.send((job_id, JobUpdate::OutputPath(output_path)));
                                let _ = update_tx.send((job_id, JobUpdate::Progress(100.0)));
                                let _ = update_tx
                                    .send((job_id, JobUpdate::Status(JobStatus::Complete)));
                            }
                            Err(e) => {
                                // Conversion failed
                                let _ = update_tx.send((
                                    job_id,
                                    JobUpdate::Error(format!("Conversion failed: {}", e)),
                                ));
                                let _ =
                                    update_tx.send((job_id, JobUpdate::Status(JobStatus::Failed)));
                            }
                        }
                    } else {
                        // No conversion, just mark as complete
                        let _ = update_tx.send((job_id, JobUpdate::OutputPath(temp_path)));
                        let _ = update_tx.send((job_id, JobUpdate::Progress(100.0)));
                        let _ = update_tx.send((job_id, JobUpdate::Status(JobStatus::Complete)));
                    }
                }
                Err(e) => {
                    // Download failed
                    let _ = update_tx
                        .send((job_id, JobUpdate::Error(format!("Download failed: {}", e))));
                    let _ = update_tx.send((job_id, JobUpdate::Status(JobStatus::Failed)));
                }
            }

            // Permit is automatically released when _permit goes out of scope
        });
    }

    pub fn available_slots(&self) -> usize {
        self.semaphore.available_permits()
    }
}
