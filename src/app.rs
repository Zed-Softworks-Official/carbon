use crate::models::{AppEvent, AppState, Config, Job, JobStatus, JobUpdate};
use crate::queue::JobQueue;
use crate::ui;
use arboard::Clipboard;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

pub struct App {
    state: Arc<Mutex<AppState>>,
    queue: JobQueue,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    job_update_rx: mpsc::UnboundedReceiver<(uuid::Uuid, JobUpdate)>,
    event_task: Option<tokio::task::JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (job_update_tx, job_update_rx) = mpsc::unbounded_channel();

        let state = Arc::new(Mutex::new(AppState::new(config.clone())));
        let queue = JobQueue::new(config.max_concurrent_downloads, job_update_tx, config);

        Self {
            state,
            queue,
            event_tx,
            event_rx,
            job_update_rx,
            event_task: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // Spawn event handler task
        let event_tx = self.event_tx.clone();
        let state = self.state.clone();
        let shutdown = self.shutdown.clone();

        let event_task = tokio::spawn(async move {
            let mut clipboard = Clipboard::new().ok();

            loop {
                // Check for shutdown signal
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(event) = event::read() {
                        if let Event::Key(key) = event {
                            if key.kind == KeyEventKind::Press {
                                // Get state info for key mapping
                                let (input_empty, has_jobs) = {
                                    let state = state.lock().await;
                                    (state.input_buffer.is_empty(), state.has_jobs())
                                };

                                let app_event =
                                    Self::map_key_event(key, input_empty, has_jobs, &mut clipboard);
                                if let Some(evt) = app_event {
                                    if event_tx.send(evt).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        self.event_task = Some(event_task);

        // Main loop
        loop {
            // Handle job updates
            while let Ok((job_id, update)) = self.job_update_rx.try_recv() {
                self.apply_job_update(job_id, update).await;
            }

            // Handle events
            while let Ok(event) = self.event_rx.try_recv() {
                if !self.handle_event(event).await? {
                    // Signal shutdown and wait for the event task to finish
                    self.shutdown.store(true, Ordering::Relaxed);
                    if let Some(task) = self.event_task.take() {
                        let _ = task.await;
                    }
                    return Ok(());
                }
            }

            // Render UI
            let state = self.state.lock().await;
            terminal.draw(|frame| ui::render(frame, &state))?;
            drop(state);

            // Process queued jobs
            self.process_queue().await;

            // Small delay to prevent CPU spinning
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn map_key_event(
        key: KeyEvent,
        input_empty: bool,
        has_jobs: bool,
        clipboard: &mut Option<Clipboard>,
    ) -> Option<AppEvent> {
        // Handle Ctrl+V for paste (always available)
        if key.code == KeyCode::Char('v') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(clipboard) = clipboard {
                if let Ok(text) = clipboard.get_text() {
                    return Some(AppEvent::InputPaste(text));
                }
            }
            return None;
        }

        match key.code {
            // Quit only works when input is empty
            KeyCode::Char('q') => {
                if input_empty {
                    Some(AppEvent::Quit)
                } else {
                    Some(AppEvent::InputChar('q'))
                }
            }
            // Delete job only works when input is empty and has jobs
            KeyCode::Char('d') => {
                if input_empty && has_jobs {
                    Some(AppEvent::DeleteJob)
                } else {
                    Some(AppEvent::InputChar('d'))
                }
            }
            // 'c' is just a regular character for input
            KeyCode::Char('c') => Some(AppEvent::InputChar('c')),
            // Navigation only works when input is empty and has jobs
            KeyCode::Up => {
                if input_empty && has_jobs {
                    Some(AppEvent::MoveUp)
                } else {
                    None
                }
            }
            KeyCode::Down => {
                if input_empty && has_jobs {
                    Some(AppEvent::MoveDown)
                } else {
                    None
                }
            }
            // Enter submits URL
            KeyCode::Enter => Some(AppEvent::SubmitUrl),
            // Escape clears input
            KeyCode::Esc => Some(AppEvent::ClearInput),
            // Backspace removes character
            KeyCode::Backspace => Some(AppEvent::InputBackspace),
            // All other characters go to input
            KeyCode::Char(c) => Some(AppEvent::InputChar(c)),
            _ => None,
        }
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<bool> {
        let mut state = self.state.lock().await;

        match event {
            AppEvent::Quit => {
                return Ok(false);
            }
            AppEvent::InputChar(c) => {
                state.input_buffer.push(c);
            }
            AppEvent::InputBackspace => {
                state.input_buffer.pop();
            }
            AppEvent::InputPaste(text) => {
                // Clean up the text (remove newlines, trim)
                let clean_text = text.trim().replace('\n', "").replace('\r', "");
                state.input_buffer.push_str(&clean_text);
            }
            AppEvent::ClearInput => {
                state.input_buffer.clear();
            }
            AppEvent::SubmitUrl => {
                if !state.input_buffer.is_empty() {
                    let url = state.input_buffer.clone();
                    let job = Job::new(url);
                    state.jobs.push(job);
                    state.input_buffer.clear();
                }
            }
            AppEvent::DeleteJob => {
                if !state.jobs.is_empty() {
                    let index = state.selected_index;
                    let job = &state.jobs[index];
                    // Only allow deleting non-active jobs
                    if !job.status.is_active() {
                        state.remove_job(index);
                    }
                }
            }

            AppEvent::MoveUp => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
            }
            AppEvent::MoveDown => {
                if state.selected_index < state.jobs.len().saturating_sub(1) {
                    state.selected_index += 1;
                }
            }
        }

        Ok(true)
    }

    async fn apply_job_update(&mut self, job_id: uuid::Uuid, update: JobUpdate) {
        let mut state = self.state.lock().await;

        if let Some(job) = state.get_job_by_id_mut(job_id) {
            match update {
                JobUpdate::Status(status) => {
                    job.status = status;
                }
                JobUpdate::Progress(progress) => {
                    job.progress = progress;
                }
                JobUpdate::Speed(speed) => {
                    job.speed = Some(speed);
                }
                JobUpdate::Eta(eta) => {
                    job.eta = Some(eta);
                }
                JobUpdate::Title(title) => {
                    job.title = Some(title);
                }
                JobUpdate::Error(error) => {
                    job.error = Some(error);
                }
                JobUpdate::TempPath(path) => {
                    job.temp_path = Some(path);
                }
                JobUpdate::OutputPath(path) => {
                    job.output_path = Some(path);
                }
            }
        }
    }

    async fn process_queue(&mut self) {
        let state = self.state.lock().await;

        // Find queued jobs
        let queued_jobs: Vec<_> = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Queued)
            .map(|j| (j.id, j.url.clone()))
            .collect();

        let quality = state.selected_quality.clone();
        drop(state);

        // Start queued jobs
        for (job_id, url) in queued_jobs {
            self.queue.start_job(job_id, url, quality.clone());
        }
    }
}
