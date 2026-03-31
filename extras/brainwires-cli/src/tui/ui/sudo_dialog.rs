//! Sudo password dialog UI rendering.
//!
//! This module renders the sudo password dialog overlay.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::App;

/// Draw the sudo password dialog overlay.
pub fn draw_sudo_dialog(f: &mut Frame, app: &mut App, _area: Rect) {
    let Some(ref state) = app.sudo_dialog_state else {
        return;
    };

    let Some(ref request) = state.current_request else {
        return;
    };

    let screen = f.area();

    // Calculate modal size (centered, 60x12)
    let modal_width = 60.min(screen.width.saturating_sub(4));
    let modal_height = 12.min(screen.height.saturating_sub(4));

    // Center the modal
    let x = (screen.width.saturating_sub(modal_width)) / 2;
    let y = (screen.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background
    f.render_widget(Clear, modal_area);

    // Yellow border for sudo prompt
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Sudo Password Required ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout: Content | Password input | Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Content (command info)
            Constraint::Length(3), // Password input
            Constraint::Length(2), // Footer with shortcuts
        ])
        .split(inner);

    // Render command info
    let command_display = if request.command.len() > 50 {
        format!("{}...", &request.command[..50])
    } else {
        request.command.clone()
    };

    let content_lines = vec![
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&command_display, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter your password to authorize this command.",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let content = Paragraph::new(content_lines).wrap(Wrap { trim: true });
    f.render_widget(content, chunks[0]);

    // Render password input (masked with *)
    let password_len = state.password_len();
    let masked: String = "*".repeat(password_len);
    let cursor_char = if password_len == state.cursor_pos {
        "_"
    } else {
        ""
    };
    let display_text = format!("{}{}", masked, cursor_char);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Password ");

    let input_text = Paragraph::new(Line::from(Span::styled(
        display_text,
        Style::default().fg(Color::White),
    )))
    .block(input_block);

    f.render_widget(input_text, chunks[1]);

    // Render footer with keyboard shortcuts
    let shortcuts = vec![
        Span::styled(
            "[Enter]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Submit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Esc]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
    ];

    let footer = Paragraph::new(Line::from(shortcuts)).alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}
