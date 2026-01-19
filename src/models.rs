use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Downloading,
    Converting,
    Complete,
    Failed,
}

impl JobStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, JobStatus::Downloading | JobStatus::Converting)
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, JobStatus::Complete)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, JobStatus::Failed)
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub url: String,
    pub title: Option<String>,
    pub status: JobStatus,
    pub progress: f64,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub error: Option<String>,
    pub output_path: Option<PathBuf>,
    pub temp_path: Option<PathBuf>,
}

impl Job {
    pub fn new(url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            title: None,
            status: JobStatus::Queued,
            progress: 0.0,
            speed: None,
            eta: None,
            error: None,
            output_path: None,
            temp_path: None,
        }
    }

    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| {
            // Truncate URL for display
            if self.url.len() > 40 {
                format!("{}...", &self.url[..40])
            } else {
                self.url.clone()
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_directory: String,
    pub max_concurrent_downloads: usize,
    pub default_quality: String,
    pub auto_convert: bool,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_output = home.join("Videos").join("carbon");

        Self {
            output_directory: default_output.to_string_lossy().to_string(),
            max_concurrent_downloads: 3,
            default_quality: "best".to_string(),
            auto_convert: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub jobs: Vec<Job>,
    pub config: Config,
    pub input_buffer: String,
    pub selected_quality: String,
    pub selected_index: usize,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            jobs: Vec::new(),
            selected_quality: config.default_quality.clone(),
            config,
            input_buffer: String::new(),
            selected_index: 0,
        }
    }

    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    pub fn active_jobs_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.is_active()).count()
    }

    pub fn queued_jobs_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == JobStatus::Queued)
            .count()
    }

    pub fn completed_jobs_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.is_complete()).count()
    }

    pub fn failed_jobs_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.is_failed()).count()
    }

    pub fn clear_completed(&mut self) {
        self.jobs.retain(|j| !j.status.is_complete());
    }

    pub fn remove_job(&mut self, index: usize) {
        if index < self.jobs.len() {
            self.jobs.remove(index);
            if self.selected_index >= self.jobs.len() && self.selected_index > 0 {
                self.selected_index = self.jobs.len() - 1;
            }
        }
    }

    pub fn get_job_by_id(&self, id: Uuid) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn get_job_by_id_mut(&mut self, id: Uuid) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Quit,
    DeleteJob,
    MoveUp,
    MoveDown,
    InputChar(char),
    InputBackspace,
    InputPaste(String),
    ClearInput,
    SubmitUrl,
}

#[derive(Debug, Clone)]
pub enum JobUpdate {
    Status(JobStatus),
    Progress(f64),
    Speed(String),
    Eta(String),
    Title(String),
    Error(String),
    TempPath(PathBuf),
    OutputPath(PathBuf),
}
