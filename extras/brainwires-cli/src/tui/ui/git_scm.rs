//! Git SCM UI Rendering
//!
//! Renders the Git Source Control Management view in full-screen mode.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::{App, GitFileStatus, GitOperationMode, ScmPanel};

/// Draw the Git SCM full-screen view
pub fn draw_git_scm(f: &mut Frame, app: &App, area: Rect) {
    // Clear the background
    f.render_widget(Clear, area);

    let Some(state) = &app.git_scm_state else {
        return;
    };

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with branch info
            Constraint::Min(1),    // File panels
            Constraint::Length(3), // Footer with key bindings
        ])
        .split(area);

    // Draw header
    draw_header(f, app, chunks[0]);

    // Draw file panels (3 columns)
    draw_file_panels(f, app, chunks[1]);

    // Draw footer
    draw_footer(f, app, chunks[2]);

    // Draw commit message overlay if in commit mode
    if let GitOperationMode::CommitMessage = state.mode {
        draw_commit_overlay(f, app, area);
    }
}

/// Draw the header with branch info and status
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.git_scm_state else {
        return;
    };

    let branch_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let mut spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled(&state.current_branch, branch_style),
    ];

    // Add upstream info
    if let Some(ref upstream) = state.upstream_branch {
        spans.push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(upstream, Style::default().fg(Color::Cyan)));
    }

    // Add ahead/behind
    if state.ahead > 0 || state.behind > 0 {
        spans.push(Span::raw(" ("));
        if state.ahead > 0 {
            spans.push(Span::styled(
                format!("↑{}", state.ahead),
                Style::default().fg(Color::Green),
            ));
        }
        if state.ahead > 0 && state.behind > 0 {
            spans.push(Span::raw(" "));
        }
        if state.behind > 0 {
            spans.push(Span::styled(
                format!("↓{}", state.behind),
                Style::default().fg(Color::Red),
            ));
        }
        spans.push(Span::raw(")"));
    }

    // Add status/error message
    let second_line = if let Some(ref err) = state.error_message {
        Line::from(vec![
            Span::styled(" ✗ ", Style::default().fg(Color::Red)),
            Span::styled(err, Style::default().fg(Color::Red)),
        ])
    } else if let Some(ref msg) = state.status_message {
        Line::from(vec![
            Span::styled(" ✓ ", Style::default().fg(Color::Green)),
            Span::styled(msg, Style::default().fg(Color::Green)),
        ])
    } else {
        let total = state.total_changes();
        Line::from(vec![Span::styled(
            format!(" {} file(s) changed", total),
            Style::default().fg(Color::Yellow),
        )])
    };

    let header_lines = vec![Line::from(spans), second_line];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Git Source Control ");

    let paragraph = Paragraph::new(header_lines).block(block);
    f.render_widget(paragraph, area);
}

/// Draw the three file panels (Staged, Changes, Untracked)
fn draw_file_panels(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.git_scm_state else {
        return;
    };

    // Split into 3 columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Draw each panel
    draw_panel(
        f,
        &state.staged_files,
        "Staged Changes",
        state.current_panel == ScmPanel::Staged,
        if state.current_panel == ScmPanel::Staged {
            Some(state.cursor_index)
        } else {
            None
        },
        state.scroll,
        chunks[0],
        Color::Green,
    );

    draw_panel(
        f,
        &state.changed_files,
        "Changes",
        state.current_panel == ScmPanel::Changes,
        if state.current_panel == ScmPanel::Changes {
            Some(state.cursor_index)
        } else {
            None
        },
        state.scroll,
        chunks[1],
        Color::Yellow,
    );

    draw_panel(
        f,
        &state.untracked_files,
        "Untracked",
        state.current_panel == ScmPanel::Untracked,
        if state.current_panel == ScmPanel::Untracked {
            Some(state.cursor_index)
        } else {
            None
        },
        state.scroll,
        chunks[2],
        Color::Cyan,
    );
}

/// Draw a single file panel
#[allow(clippy::too_many_arguments)]
fn draw_panel(
    f: &mut Frame,
    files: &[crate::tui::app::GitFileEntry],
    title: &str,
    is_focused: bool,
    cursor_index: Option<usize>,
    scroll: u16,
    area: Rect,
    accent_color: Color,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_with_count = format!(" {} ({}) ", title, files.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title_with_count);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if files.is_empty() {
        let empty_msg = Paragraph::new("No files")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(empty_msg, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let scroll_offset = scroll as usize;

    let mut lines = Vec::new();
    for (idx, entry) in files
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let is_cursor = cursor_index == Some(idx);

        let base_style = if is_cursor {
            Style::default()
                .fg(Color::Black)
                .bg(accent_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let checkbox = if entry.selected { "[x]" } else { "[ ]" };

        let status_indicator = entry.status.indicator();
        let status_color = match entry.status {
            GitFileStatus::Modified | GitFileStatus::StagedModified => Color::Yellow,
            GitFileStatus::Staged => Color::Green,
            GitFileStatus::Untracked => Color::Cyan,
            GitFileStatus::Deleted | GitFileStatus::StagedDeleted => Color::Red,
            GitFileStatus::Conflict => Color::Magenta,
            GitFileStatus::Renamed | GitFileStatus::Copied => Color::Blue,
            GitFileStatus::Ignored => Color::DarkGray,
        };

        let file_name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| entry.path.to_string_lossy().to_string());

        // Truncate if too long
        let max_name_len = inner.width.saturating_sub(10) as usize;
        let display_name = if file_name.len() > max_name_len {
            format!("{}...", &file_name[..max_name_len.saturating_sub(3)])
        } else {
            file_name
        };

        lines.push(Line::from(vec![
            Span::styled(checkbox, base_style),
            Span::styled(" ", base_style),
            Span::styled(
                status_indicator,
                if is_cursor {
                    base_style
                } else {
                    Style::default().fg(status_color)
                },
            ),
            Span::styled(" ", base_style),
            Span::styled(display_name, base_style),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

/// Draw the footer with key bindings
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.git_scm_state else {
        return;
    };

    let footer_lines = match state.mode {
        GitOperationMode::Browse => vec![
            Line::from(vec![
                Span::styled("Tab", Style::default().fg(Color::Green)),
                Span::raw(":Panel "),
                Span::styled("Space", Style::default().fg(Color::Green)),
                Span::raw(":Select "),
                Span::styled("Enter/s", Style::default().fg(Color::Green)),
                Span::raw(":Stage "),
                Span::styled("u", Style::default().fg(Color::Green)),
                Span::raw(":Unstage "),
                Span::styled("d", Style::default().fg(Color::Green)),
                Span::raw(":Discard "),
            ]),
            Line::from(vec![
                Span::styled("c", Style::default().fg(Color::Green)),
                Span::raw(":Commit "),
                Span::styled("P", Style::default().fg(Color::Green)),
                Span::raw(":Push "),
                Span::styled("p", Style::default().fg(Color::Green)),
                Span::raw(":Pull "),
                Span::styled("f", Style::default().fg(Color::Green)),
                Span::raw(":Fetch "),
                Span::styled("r", Style::default().fg(Color::Green)),
                Span::raw(":Refresh "),
                Span::styled("Esc", Style::default().fg(Color::Green)),
                Span::raw(":Close"),
            ]),
        ],
        GitOperationMode::CommitMessage => vec![
            Line::from(vec![Span::styled(
                "Enter commit message...",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(":Commit "),
                Span::styled("Esc", Style::default().fg(Color::Green)),
                Span::raw(":Cancel"),
            ]),
        ],
        GitOperationMode::Confirm { ref message, .. } => vec![
            Line::from(vec![Span::styled(
                message,
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![
                Span::styled("y", Style::default().fg(Color::Green)),
                Span::raw(":Confirm "),
                Span::styled("n/Esc", Style::default().fg(Color::Green)),
                Span::raw(":Cancel"),
            ]),
        ],
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(footer_lines)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Draw commit message overlay
fn draw_commit_overlay(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.git_scm_state else {
        return;
    };

    // Calculate overlay size and position
    let overlay_width = area.width.saturating_sub(10).min(80);
    let overlay_height = 8;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;

    let overlay_area = Rect {
        x: area.x + overlay_x,
        y: area.y + overlay_y,
        width: overlay_width,
        height: overlay_height,
    };

    // Clear and draw overlay
    f.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Commit Message ");

    let inner = block.inner(overlay_area);
    f.render_widget(block, overlay_area);

    // Show staged file count
    let staged_count = state.staged_files.len();
    let info_line = Line::from(vec![Span::styled(
        format!("{} file(s) to commit", staged_count),
        Style::default().fg(Color::DarkGray),
    )]);

    // Show commit message with cursor
    let message_with_cursor = format!("{}_", state.commit_message);
    let message_line = Line::from(vec![Span::styled(
        &message_with_cursor,
        Style::default().fg(Color::White),
    )]);

    let help_line = Line::from(vec![Span::styled(
        "Enter to commit, Esc to cancel",
        Style::default().fg(Color::DarkGray),
    )]);

    let content = vec![
        info_line,
        Line::from(""),
        message_line,
        Line::from(""),
        help_line,
    ];

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}
