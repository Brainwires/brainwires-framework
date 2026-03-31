//! Shell Viewer UI
//!
//! Renders the shell history viewer.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::tui::app::App;

/// Draw the shell history viewer
pub fn draw_shell_viewer(f: &mut Frame, app: &App, area: Rect) {
    if app.shell_history.is_empty() {
        // Show empty state
        let empty_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No shell commands executed yet",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Use "),
                Span::styled("/exec <command>", Style::default().fg(Color::Cyan)),
                Span::raw(" to run shell commands."),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(Color::Green)),
                Span::raw(": Exit viewer"),
            ]),
        ];

        let empty_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Shell History ");

        let empty_paragraph = Paragraph::new(Text::from(empty_text))
            .block(empty_block)
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(empty_paragraph, area);
        return;
    }

    // Split into list on left and output on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Draw command list
    let list_items: Vec<ListItem> = app
        .shell_history
        .iter()
        .enumerate()
        .map(|(idx, exec)| {
            let timestamp = chrono::DateTime::from_timestamp(exec.executed_at, 0)
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "??:??:??".to_string());

            let status_color = if exec.exit_code == 0 {
                Color::Green
            } else {
                Color::Red
            };

            let status_symbol = if exec.exit_code == 0 { "✓" } else { "✗" };

            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", status_symbol),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(exec.command.chars().take(30).collect::<String>()),
                if exec.command.len() > 30 {
                    Span::styled("...", Style::default().fg(Color::Gray))
                } else {
                    Span::raw("")
                },
            ]);

            let style = if idx == app.selected_shell_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" Commands ({}) ", app.shell_history.len()));

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(list, chunks[0]);

    // Draw selected command output
    if let Some(exec) = app.shell_history.get(app.selected_shell_index) {
        let mut output_lines = Vec::new();

        // Command header
        output_lines.push(Line::from(vec![
            Span::styled(
                "Command: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&exec.command),
        ]));

        output_lines.push(Line::from(vec![
            Span::styled("Exit code: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                exec.exit_code.to_string(),
                if exec.exit_code == 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]));

        output_lines.push(Line::from(""));
        output_lines.push(Line::from(vec![Span::styled(
            "Output:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        output_lines.push(Line::from(""));

        // Output lines
        for line in exec.output.lines() {
            let style = if line.starts_with("stderr:") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };
            output_lines.push(Line::from(Span::styled(line, style)));
        }

        // Footer
        output_lines.push(Line::from(""));
        output_lines.push(Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Green)),
            Span::raw(": Select command | "),
            Span::styled("PgUp/PgDn", Style::default().fg(Color::Green)),
            Span::raw(": Scroll output | "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(": Exit"),
        ]));

        let output_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(" Output ");

        let output_paragraph = Paragraph::new(Text::from(output_lines))
            .block(output_block)
            .wrap(Wrap { trim: false })
            .scroll((app.shell_viewer_scroll, 0));

        f.render_widget(output_paragraph, chunks[1]);
    }
}
