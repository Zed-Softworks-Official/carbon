mod app;
mod config;
mod converter;
mod downloader;
mod models;
mod queue;
mod ui;

use app::App;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;

    // Load configuration
    let config = config::load_config()?;

    // Initialize terminal
    let mut terminal = ratatui::init();

    // Create and run app
    let mut app = App::new(config);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
