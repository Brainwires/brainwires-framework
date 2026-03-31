//! Nano-style Editor UI Rendering
//!
//! Renders the nano-style text editor in full-screen mode.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::App;

/// Draw the nano editor full-screen
pub fn draw_nano_editor(f: &mut Frame, app: &mut App, area: Rect) {
    // Clear the background
    f.render_widget(Clear, area);

    let Some(_state) = &mut app.nano_editor_state else {
        return;
    };

    // Main layout: content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Editor content
            Constraint::Length(2), // Footer with key bindings + status
        ])
        .split(area);

    draw_editor_content(f, app, chunks[0]);
    draw_editor_footer(f, app, chunks[1]);
}

/// Draw the editor content with line numbers
fn draw_editor_content(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &mut app.nano_editor_state else {
        return;
    };

    let modified_marker = if state.modified { " -- Modified" } else { "" };
    let read_only_marker = if state.read_only { " [RO]" } else { "" };
    let title = format!(
        " Nano Editor -- {}{}{} ",
        state.file_name(),
        modified_marker,
        read_only_marker
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if state.modified {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        })
        .title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Calculate visible lines
    let visible_rows = inner.height as usize;
    let visible_cols = inner.width.saturating_sub(7) as usize; // Account for line numbers "XXXX | "

    // Ensure cursor is visible
    state.ensure_cursor_visible(visible_rows as u16, visible_cols as u16);

    let start_row = state.scroll_row as usize;
    let end_row = (start_row + visible_rows).min(state.lines.len());

    // Calculate line number width
    let max_line_num = state.lines.len();
    let line_num_width = max_line_num.to_string().len().max(4);

    let mut lines = Vec::new();
    for line_idx in start_row..end_row {
        let actual_line_num = line_idx + 1;
        let is_cursor_line = line_idx == state.cursor_row;

        let line_num_style = if is_cursor_line {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let content_style = if is_cursor_line {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let line_content = state.lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");

        // Handle horizontal scrolling
        let display_content = if state.scroll_col as usize > line_content.len() {
            String::new()
        } else {
            let start = state.scroll_col as usize;
            let end = (start + visible_cols).min(line_content.len());
            line_content[start..end].to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:>width$} | ", actual_line_num, width = line_num_width),
                line_num_style,
            ),
            Span::styled(display_content, content_style),
        ]));
    }

    // Add empty lines if needed
    while lines.len() < visible_rows {
        let line_num = start_row + lines.len() + 1;
        if line_num <= state.lines.len() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>width$} | ", line_num, width = line_num_width),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(""),
            ]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                format!("{:>width$}   ", "~", width = line_num_width),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);

    // Set cursor position
    let cursor_x = inner.x
        + line_num_width as u16
        + 3
        + (state.cursor_col as u16).saturating_sub(state.scroll_col);
    let cursor_y = inner.y + (state.cursor_row as u16).saturating_sub(state.scroll_row);

    if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Draw the editor footer with key bindings and status
fn draw_editor_footer(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.nano_editor_state else {
        return;
    };

    let status = state.status_message.as_deref().unwrap_or("");
    let position = format!("Ln:{} Col:{}", state.cursor_row + 1, state.cursor_col + 1);

    let footer_line = Line::from(vec![
        Span::styled("^S", Style::default().fg(Color::Green)),
        Span::raw(":Save "),
        Span::styled("^X", Style::default().fg(Color::Green)),
        Span::raw(":Exit "),
        Span::styled("^K", Style::default().fg(Color::Green)),
        Span::raw(":Cut "),
        Span::styled("^U", Style::default().fg(Color::Green)),
        Span::raw(":Paste "),
        Span::raw("| "),
        Span::styled(&position, Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(status, Style::default().fg(Color::Magenta)),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(vec![footer_line]).block(block);

    f.render_widget(paragraph, area);
}
