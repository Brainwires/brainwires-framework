//! Task Panel UI
//!
//! Renders a sidebar panel showing the session task list.
//! Displayed on wide screens (>= 120 columns).

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::types::session_task::SessionTaskStatus;

/// Draw the session task list panel (right sidebar)
pub fn draw_task_panel(f: &mut Frame, app: &App, area: Rect) {
    let list = &app.session_task_panel_cache;

    // Create title with progress count
    let completed = list
        .iter()
        .filter(|(_, _, status)| *status == SessionTaskStatus::Completed)
        .count();
    let total = list.len();

    let title = if total > 0 {
        format!(" Tasks ({}/{}) ", completed, total)
    } else {
        " Tasks ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if list.is_empty() {
        let empty_msg = Paragraph::new("No tasks").style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty_msg, inner_area);
        return;
    }

    // Render task list
    let lines: Vec<Line> = list
        .iter()
        .map(|(icon, text, status)| {
            let (icon_style, text_style) = match status {
                SessionTaskStatus::Completed => (
                    Style::default().fg(Color::Green),
                    Style::default().fg(Color::DarkGray),
                ),
                SessionTaskStatus::InProgress => (
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                SessionTaskStatus::Pending => (
                    Style::default().fg(Color::DarkGray),
                    Style::default().fg(Color::White),
                ),
            };

            // Truncate text to fit panel width (leave room for icon and padding)
            let max_text_width = inner_area.width.saturating_sub(4) as usize;
            let display_text = if text.len() > max_text_width {
                format!("{}...", &text[..max_text_width.saturating_sub(3)])
            } else {
                text.clone()
            };

            Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(" "),
                Span::styled(display_text, text_style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });

    f.render_widget(paragraph, inner_area);
}
