//! Console View UI
//!
//! Renders the full-screen debug console.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
    Frame,
};
use ratatui_interact::components::ParagraphExt;

use crate::tui::app::App;

/// Draw the console view (full-screen debug console)
pub fn draw_console_view(f: &mut Frame, app: &App, area: Rect) {
    let mut items = Vec::new();

    // Header
    items.push(Line::from(vec![
        Span::styled(
            format!("Console - {} messages", app.console_state.line_count()),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        ),
    ]));
    items.push(Line::from("")); // Empty line

    // Console messages
    if app.console_state.lines().is_empty() {
        items.push(Line::from(vec![
            Span::styled("No console messages yet", Style::default().fg(Color::Gray)),
        ]));
        items.push(Line::from(""));
        items.push(Line::from(vec![
            Span::raw("Debug messages from "),
            Span::styled("eprintln!", Style::default().fg(Color::Yellow)),
            Span::raw(" will appear here."),
        ]));
    } else {
        for (idx, msg) in app.console_state.lines().iter().enumerate() {
            // Add line number prefix
            let prefix = format!("{:4} | ", idx + 1);

            // Colorize based on content
            let style = if msg.contains("ERROR") || msg.contains("error") {
                Style::default().fg(Color::Red)
            } else if msg.contains("WARN") || msg.contains("warning") {
                Style::default().fg(Color::Yellow)
            } else if msg.contains("DEBUG") {
                Style::default().fg(Color::Cyan)
            } else if msg.contains("INFO") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            items.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Gray)),
                Span::styled(msg.clone(), style),
            ]));
        }
    }

    // Footer instructions (only show when borders are visible)
    if !app.mouse_capture_disabled {
        items.push(Line::from(""));
        items.push(Line::from(vec![
            Span::styled("PgUp/PgDn", Style::default().fg(Color::Green)),
            Span::raw(": Scroll | "),
            Span::styled("c", Style::default().fg(Color::Green)),
            Span::raw(": Copy | "),
            Span::styled("m", Style::default().fg(Color::Green)),
            Span::raw(": Mouse | "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(": Exit"),
        ]));
    }

    let scroll_offset = app.console_state.scroll_offset() as u16;

    // When mouse capture is disabled, use clean widget for copy/paste
    if app.mouse_capture_disabled {
        let widget = ParagraphExt::new(items).scroll(scroll_offset).width(area.width);
        f.render_widget(widget, area);
    } else {
        let console_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(" Console View (Full Screen) ");

        let console_text = Text::from(items);
        let console_paragraph = Paragraph::new(console_text)
            .block(console_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset, 0));

        f.render_widget(console_paragraph, area);
    }

    // Draw toast notification if active
    if let Some(toast_msg) = app.get_toast() {
        draw_toast(f, toast_msg, area);
    }
}

/// Draw a centered toast notification
fn draw_toast(f: &mut Frame, message: &str, area: Rect) {
    use ratatui_interact::components::Toast;
    Toast::new(message).render_with_clear(area, f.buffer_mut());
}
