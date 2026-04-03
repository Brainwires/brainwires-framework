//! Activity Journal UI
//!
//! Renders the full-screen activity journal (formerly "console view").
//! Every status change is journaled here with a timestamp and severity level.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use ratatui_interact::components::ParagraphExt;

use crate::tui::app::App;

/// Draw the activity journal (full-screen)
pub fn draw_console_view(f: &mut Frame, app: &App, area: Rect) {
    let mut items = Vec::new();

    // Header
    items.push(Line::from(vec![Span::styled(
        format!(
            "Activity Journal — {} entries",
            app.console_state.line_count()
        ),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    items.push(Line::from("")); // Empty line

    // Journal entries
    if app.console_state.lines().is_empty() {
        items.push(Line::from(vec![Span::styled(
            "No activity yet — events will appear here as you use the TUI.",
            Style::default().fg(Color::Gray),
        )]));
    } else {
        for msg in app.console_state.lines().iter() {
            items.push(render_journal_line(msg));
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
        let widget = ParagraphExt::new(items)
            .scroll(scroll_offset)
            .width(area.width);
        f.render_widget(widget, area);
    } else {
        let console_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(" Activity Journal (Full Screen) ");

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

/// Parse a journal line and produce a styled `Line`.
///
/// Expected format from `set_status`:  `HH:MM:SS [LEVEL] message text`
/// Lines that don't match this format (e.g. raw `add_console_message` calls) are
/// rendered with the existing heuristic coloring.
fn render_journal_line(msg: &str) -> Line<'static> {
    // Try to parse: "HH:MM:SS [LEVEL] rest…"
    // Timestamp is exactly 8 chars ("00:00:00"), then a space, then "[LEVEL]" (7 chars), then space+rest
    if msg.len() > 17
        && msg.as_bytes()[2] == b':'
        && msg.as_bytes()[5] == b':'
        && msg.as_bytes()[8] == b' '
        && msg.as_bytes()[9] == b'['
    {
        if let Some(bracket_end) = msg[9..].find(']') {
            let level_str = &msg[10..9 + bracket_end];
            let ts = &msg[..8];
            let tag = &msg[9..9 + bracket_end + 1]; // "[LEVEL]"
            let rest = if msg.len() > 9 + bracket_end + 2 {
                &msg[9 + bracket_end + 2..]
            } else {
                ""
            };

            let (tag_style, msg_style) = match level_str.trim() {
                "ERROR" => (
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Red),
                ),
                "WARN" => (
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Yellow),
                ),
                "DEBUG" => (
                    Style::default().fg(Color::Cyan),
                    Style::default().fg(Color::Cyan),
                ),
                "INFO" | _ => (
                    Style::default().fg(Color::Green),
                    Style::default().fg(Color::White),
                ),
            };

            return Line::from(vec![
                Span::styled(ts.to_string(), Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(tag.to_string(), tag_style),
                Span::raw(" "),
                Span::styled(rest.to_string(), msg_style),
            ]);
        }
    }

    // Fallback: legacy heuristic coloring for raw add_console_message entries
    let style = if msg.contains("ERROR") || msg.contains("error") || msg.contains('❌') {
        Style::default().fg(Color::Red)
    } else if msg.contains("WARN") || msg.contains("warning") || msg.contains('⚠') {
        Style::default().fg(Color::Yellow)
    } else if msg.contains("DEBUG") {
        Style::default().fg(Color::Cyan)
    } else if msg.contains("INFO") || msg.contains('✅') {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::White)
    };

    Line::from(vec![Span::styled(msg.to_string(), style)])
}

/// Draw a centered toast notification
fn draw_toast(f: &mut Frame, message: &str, area: Rect) {
    use ratatui_interact::components::Toast;
    Toast::new(message).render_with_clear(area, f.buffer_mut());
}
