//! Session Picker UI
//!
//! Renders the session picker for loading saved conversations.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Draw the session picker
pub fn draw_session_picker(f: &mut Frame, app: &App, area: Rect) {
    let picker_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Session Picker (Ctrl+L) ");

    let inner_area = picker_block.inner(area);
    f.render_widget(picker_block, area);

    // Reserve last line for footer instructions
    let content_height = inner_area.height.saturating_sub(1);

    let list_area = Rect {
        x: inner_area.x,
        y: inner_area.y,
        width: inner_area.width,
        height: content_height,
    };

    let footer_area = Rect {
        x: inner_area.x,
        y: inner_area.y + content_height,
        width: inner_area.width,
        height: 1,
    };

    // Build session list items
    let mut items = Vec::new();

    // Header
    items.push(Line::from(vec![
        Span::styled("Select a session to load:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));
    items.push(Line::from("")); // Empty line

    // Session list
    if app.available_sessions.is_empty() {
        items.push(Line::from(vec![
            Span::styled("No saved sessions found", Style::default().fg(Color::Gray)),
        ]));
    } else {
        for (idx, session) in app.available_sessions.iter().enumerate() {
            let is_selected = idx == app.selected_session_index;

            // Format timestamp
            let timestamp = chrono::DateTime::from_timestamp(session.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            // Selection indicator
            let prefix = if is_selected { "▶ " } else { "  " };

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Session line with title, model, and message count
            let title = session.title.as_deref().unwrap_or("Untitled");
            let model = session.model_id.as_deref().unwrap_or("Unknown model");
            let msg_count = session.message_count;

            items.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{} ", title), style),
                Span::styled(format!("({})", model), Style::default().fg(Color::Cyan)),
                Span::raw(" - "),
                Span::styled(format!("{} messages", msg_count), Style::default().fg(Color::Gray)),
            ]));

            // Timestamp line
            items.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(timestamp, Style::default().fg(Color::Gray)),
            ]));
        }
    }

    // Draw scrollable session list
    let picker_text = Text::from(items);
    let picker_paragraph = Paragraph::new(picker_text)
        .wrap(Wrap { trim: false })
        .scroll((app.session_picker_scroll, 0));

    f.render_widget(picker_paragraph, list_area);

    // Draw fixed footer instructions (pinned to bottom)
    let footer_text = Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Green)),
        Span::raw(": Navigate | "),
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::raw(": Load | "),
        Span::styled("Esc", Style::default().fg(Color::Green)),
        Span::raw(": Cancel"),
    ]);

    let footer_paragraph = Paragraph::new(footer_text)
        .alignment(Alignment::Center);

    f.render_widget(footer_paragraph, footer_area);
}
