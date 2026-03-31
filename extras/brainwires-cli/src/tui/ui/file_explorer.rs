//! File Explorer UI Rendering
//!
//! Renders the file explorer popup in full-screen mode.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::{App, EntryType, FileExplorerMode};

/// Draw the file explorer full-screen popup
pub fn draw_file_explorer(f: &mut Frame, app: &App, area: Rect) {
    // Clear the background
    f.render_widget(Clear, area);

    let Some(state) = &app.file_explorer_state else {
        return;
    };

    // Main layout: content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // File list
            Constraint::Length(3), // Footer with key bindings
        ])
        .split(area);

    // Draw file list
    draw_file_list(f, app, chunks[0]);

    // Draw footer
    draw_file_explorer_footer(f, app, chunks[1]);

    // If in search mode, show search bar at the bottom
    if state.mode == FileExplorerMode::Search {
        draw_search_bar(f, state.search_query.as_str(), chunks[1]);
    }
}

/// Draw the file list
fn draw_file_list(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.file_explorer_state else {
        return;
    };

    let title = format!(" File Explorer -- {} ", state.current_dir.display());

    let selected_count = state.selected_files.len();
    let title_with_count = if selected_count > 0 {
        format!("{} ({} selected) ", title, selected_count)
    } else {
        title
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title_with_count);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build entry lines
    let visible_height = inner.height as usize;
    let scroll = state.scroll as usize;

    // Get entries to display (filtered or all)
    let entries_to_show: Vec<(usize, &crate::tui::app::FileEntry)> =
        if let Some(ref indices) = state.filtered_indices {
            indices.iter().map(|&i| (i, &state.entries[i])).collect()
        } else {
            state.entries.iter().enumerate().collect()
        };

    let mut lines = Vec::new();
    for (display_idx, (_entry_idx, entry)) in entries_to_show
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
    {
        let is_cursor = display_idx == state.cursor_index;
        let is_checked = state.selected_files.contains(&entry.path);

        let style = if is_cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let cursor = if is_cursor { ">" } else { " " };
        let checkbox = match &entry.entry_type {
            EntryType::File { .. } => {
                if is_checked {
                    "[x]"
                } else {
                    "[ ]"
                }
            }
            _ => "   ", // No checkbox for directories
        };

        let (icon, name_style) = match &entry.entry_type {
            EntryType::Directory => (
                "[DIR]",
                if is_cursor {
                    style.fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                },
            ),
            EntryType::ParentDir => (
                " .. ",
                if is_cursor {
                    style.fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                },
            ),
            EntryType::File { extension, .. } => {
                let color = match extension.as_deref() {
                    Some("rs") => Color::Yellow,
                    Some("toml" | "json" | "yaml" | "yml") => Color::Green,
                    Some("md" | "txt" | "rst") => Color::White,
                    Some("py") => Color::Cyan,
                    Some("js" | "ts" | "tsx" | "jsx") => Color::Magenta,
                    Some("sh" | "bash" | "zsh") => Color::Red,
                    _ => Color::Gray,
                };
                (
                    "     ",
                    if is_cursor {
                        style.fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(color)
                    },
                )
            }
            EntryType::Symlink { .. } => (
                "[LNK]",
                if is_cursor {
                    style.fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Magenta)
                },
            ),
        };

        let size_str = match &entry.entry_type {
            EntryType::File { size, .. } => format_size(*size),
            _ => String::new(),
        };

        // Build the line with proper padding
        let name_width = inner.width.saturating_sub(22) as usize; // Account for cursor, checkbox, icon, size
        let display_name = if entry.name.len() > name_width {
            format!("{}...", &entry.name[..name_width.saturating_sub(3)])
        } else {
            entry.name.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(cursor, style),
            Span::styled(" ", style),
            Span::styled(checkbox, style),
            Span::styled(" ", style),
            Span::styled(icon, style),
            Span::styled(" ", style),
            Span::styled(
                format!("{:<width$}", display_name, width = name_width),
                name_style,
            ),
            Span::styled(
                format!("{:>10}", size_str),
                if is_cursor {
                    style.fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]));
    }

    // Add empty lines if needed
    while lines.len() < visible_height {
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

/// Draw the footer with key bindings
fn draw_file_explorer_footer(f: &mut Frame, _app: &App, area: Rect) {
    let footer_lines = vec![
        Line::from(vec![
            Span::styled("^/v", Style::default().fg(Color::Green)),
            Span::raw(":Move "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(":Open "),
            Span::styled("Space", Style::default().fg(Color::Green)),
            Span::raw(":Select "),
            Span::styled("i", Style::default().fg(Color::Green)),
            Span::raw(":Insert "),
            Span::styled("e", Style::default().fg(Color::Green)),
            Span::raw(":Edit "),
            Span::styled("/", Style::default().fg(Color::Green)),
            Span::raw(":Search"),
        ]),
        Line::from(vec![
            Span::styled(".", Style::default().fg(Color::Green)),
            Span::raw(":Hidden "),
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(":All "),
            Span::styled("n", Style::default().fg(Color::Green)),
            Span::raw(":None "),
            Span::styled("r", Style::default().fg(Color::Green)),
            Span::raw(":Refresh "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(":Close"),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(footer_lines)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Draw the search bar (overlay on footer when in search mode)
fn draw_search_bar(f: &mut Frame, query: &str, area: Rect) {
    let search_text = Line::from(vec![
        Span::styled(
            "Search: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(query, Style::default().fg(Color::White)),
        Span::styled(
            "_",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(vec![
        search_text,
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(":Confirm "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(":Cancel"),
        ]),
    ])
    .block(block)
    .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

/// Format file size in human-readable form
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
