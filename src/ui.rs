use crate::models::{AppState, JobStatus};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

// Color palette
const COLOR_BG: Color = Color::Rgb(0, 0, 0); // Pure black
const COLOR_INPUT_BG: Color = Color::Rgb(28, 28, 32);
const COLOR_ACCENT: Color = Color::Rgb(100, 140, 200); // Muted blue
const COLOR_TEXT: Color = Color::Rgb(200, 200, 200);
const COLOR_DIM: Color = Color::Rgb(100, 100, 100);
const COLOR_PLACEHOLDER: Color = Color::Rgb(70, 70, 70);
const COLOR_YELLOW: Color = Color::Rgb(220, 180, 100); // Muted yellow
const COLOR_GREEN: Color = Color::Rgb(130, 190, 130); // Muted green
const COLOR_RED: Color = Color::Rgb(200, 100, 100); // Muted red
const COLOR_SELECTION: Color = Color::Rgb(35, 35, 45);

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Clear the terminal and fill with pure black
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), area);

    if state.has_jobs() {
        render_jobs_view(frame, area, state);
    } else {
        render_welcome_view(frame, area, state);
    }
}

/// Render the welcome view - shown when there are no jobs
fn render_welcome_view(frame: &mut Frame, area: Rect, state: &AppState) {
    // Center everything vertically
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(3), // Title
            Constraint::Length(2), // Spacing
            Constraint::Length(3), // Input box
            Constraint::Length(2), // Spacing
            Constraint::Length(1), // Shortcuts
            Constraint::Min(0),    // Rest
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        "carbon",
        Style::default()
            .fg(COLOR_ACCENT)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[1]);

    // Input box - centered horizontally with max width
    let input_area = center_horizontally(chunks[3], 60);
    render_input_box(frame, input_area, state, "paste a url...");

    // Shortcuts
    let shortcuts =
        create_shortcuts_line(&[("enter", "submit"), ("ctrl+v", "paste"), ("q", "quit")]);
    let shortcuts_widget = Paragraph::new(shortcuts).alignment(Alignment::Center);
    frame.render_widget(shortcuts_widget, chunks[5]);
}

/// Render the jobs view - shown when there are active jobs
fn render_jobs_view(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Jobs list
            Constraint::Length(3), // Input box
            Constraint::Length(2), // Shortcuts
        ])
        .split(area);

    // Jobs list
    render_jobs_list(frame, chunks[0], state);

    // Input box - with horizontal padding
    let input_area = chunks[1].inner(Margin::new(2, 0));
    render_input_box(frame, input_area, state, "paste another url...");

    // Shortcuts
    let shortcuts = if state.input_buffer.is_empty() {
        create_shortcuts_line(&[
            ("enter", "submit"),
            ("ctrl+v", "paste"),
            ("d", "delete"),
            ("↑↓", "navigate"),
            ("q", "quit"),
        ])
    } else {
        create_shortcuts_line(&[("enter", "submit"), ("ctrl+v", "paste"), ("esc", "clear")])
    };
    let shortcuts_widget = Paragraph::new(shortcuts).alignment(Alignment::Center);
    frame.render_widget(shortcuts_widget, chunks[2]);
}

/// Render the input box with dark grey background
fn render_input_box(frame: &mut Frame, area: Rect, state: &AppState, placeholder: &str) {
    let input_text = if state.input_buffer.is_empty() {
        placeholder.to_string()
    } else {
        format!("{}_", state.input_buffer)
    };

    let text_color = if state.input_buffer.is_empty() {
        COLOR_PLACEHOLDER
    } else {
        COLOR_ACCENT
    };

    let input = Paragraph::new(format!(" {}", input_text))
        .style(Style::default().fg(text_color).bg(COLOR_INPUT_BG))
        .block(
            Block::default()
                .style(Style::default().bg(COLOR_INPUT_BG))
                .padding(Padding::vertical(1)),
        );

    frame.render_widget(input, area);
}

/// Render the jobs list with inline progress bars
fn render_jobs_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let list_area = area.inner(Margin::new(2, 1));

    let items: Vec<ListItem> = state
        .jobs
        .iter()
        .enumerate()
        .flat_map(|(idx, job)| {
            let is_selected = idx == state.selected_index;

            let (status_symbol, status_color) = match job.status {
                JobStatus::Queued => ("○", COLOR_DIM),
                JobStatus::Downloading => ("●", COLOR_ACCENT),
                JobStatus::Converting => ("◐", COLOR_YELLOW),
                JobStatus::Complete => ("✓", COLOR_GREEN),
                JobStatus::Failed => ("✗", COLOR_RED),
            };

            let title = job.display_title();
            let title_display = if title.len() > 50 {
                format!("{}...", &title[..47])
            } else {
                title
            };

            let status_text = match job.status {
                JobStatus::Queued => "queued",
                JobStatus::Downloading => "downloading",
                JobStatus::Converting => "converting",
                JobStatus::Complete => "complete",
                JobStatus::Failed => "failed",
            };

            // Build the main job line
            let mut main_line = vec![
                Span::styled(
                    format!(" {} ", status_symbol),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("{:<12}", status_text),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::DIM),
                ),
                Span::styled(title_display, Style::default().fg(COLOR_TEXT)),
            ];

            // Add extra info for certain states
            if job.status.is_complete() {
                if let Some(path) = &job.output_path {
                    let path_str = path.to_string_lossy();
                    let display_path = if path_str.len() > 25 {
                        format!("  ...{}", &path_str[path_str.len() - 22..])
                    } else {
                        format!("  {}", path_str)
                    };
                    main_line.push(Span::styled(
                        display_path,
                        Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM),
                    ));
                }
            } else if job.status.is_failed() {
                if let Some(error) = &job.error {
                    let error_display = if error.len() > 40 {
                        format!("  {}...", &error[..37])
                    } else {
                        format!("  {}", error)
                    };
                    main_line.push(Span::styled(error_display, Style::default().fg(COLOR_RED)));
                }
            }

            let main_style = if is_selected {
                Style::default().bg(COLOR_SELECTION)
            } else {
                Style::default()
            };

            let mut items = vec![ListItem::new(Line::from(main_line)).style(main_style)];

            // Add progress bar for active jobs
            if job.status.is_active() {
                let progress_line =
                    create_progress_line(job.progress, &job.speed, &job.eta, &job.status);
                let progress_style = if is_selected {
                    Style::default().bg(COLOR_SELECTION)
                } else {
                    Style::default()
                };
                items.push(ListItem::new(progress_line).style(progress_style));
                // Add empty line after progress
                items.push(ListItem::new(Line::from("")));
            }

            items
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);
}

/// Create a text-based progress line
fn create_progress_line(
    progress: f64,
    speed: &Option<String>,
    eta: &Option<String>,
    status: &JobStatus,
) -> Line<'static> {
    let bar_width = 30;
    let filled = ((progress / 100.0) * bar_width as f64) as usize;
    let empty = bar_width - filled;

    let progress_color = match status {
        JobStatus::Downloading => COLOR_ACCENT,
        JobStatus::Converting => COLOR_YELLOW,
        _ => COLOR_DIM,
    };

    let mut spans = vec![
        Span::raw("    "),
        Span::styled("█".repeat(filled), Style::default().fg(progress_color)),
        Span::styled("░".repeat(empty), Style::default().fg(COLOR_INPUT_BG)),
        Span::styled(
            format!(" {:5.1}%", progress),
            Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(s) = speed {
        spans.push(Span::styled(
            format!("  {}", s),
            Style::default().fg(COLOR_DIM),
        ));
    }

    if let Some(e) = eta {
        spans.push(Span::styled(
            format!("  eta {}", e),
            Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM),
        ));
    }

    Line::from(spans)
}

/// Create a shortcuts line
fn create_shortcuts_line(shortcuts: &[(&str, &str)]) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();

    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(COLOR_DIM)));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(COLOR_ACCENT),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(COLOR_DIM),
        ));
    }

    Line::from(spans)
}

/// Center a rect horizontally within a container, with a max width
fn center_horizontally(container: Rect, max_width: u16) -> Rect {
    let width = max_width.min(container.width.saturating_sub(4));
    let x = container.x + (container.width.saturating_sub(width)) / 2;
    Rect::new(x, container.y, width, container.height)
}
