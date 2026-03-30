//! Task Viewer UI
//!
//! Renders a full-screen view for viewing and managing the task tree.
//! Supports tree navigation, expand/collapse, and status changes.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Draw the task viewer as a full-screen view
pub fn draw_task_viewer(f: &mut Frame, app: &App) {
    // Use full screen
    let area = f.area();

    // Clear the background
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Task Tree (Esc: close, Enter: expand/collapse, Space: toggle status) ");

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split into content and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Task tree content
            Constraint::Length(3), // Footer with instructions
        ])
        .split(inner_area);

    // Render task tree
    render_task_tree(f, app, chunks[0]);

    // Render footer
    render_footer(f, chunks[1]);
}

/// Render the task tree content
fn render_task_tree(f: &mut Frame, app: &App, area: Rect) {
    let state = &app.task_viewer_state;
    let mut lines = Vec::new();

    if state.visible_tasks.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "No tasks yet. Tasks will appear here as they are created.",
                Style::default().fg(Color::Gray),
            ),
        ]));
    } else {
        // We need to get tasks from task_manager - but we can't await here
        // So we use the cached task_tree_cache for now and enhance later
        // For proper tree rendering, we'll build from visible_tasks

        // Header with stats
        let total = state.visible_tasks.len();
        lines.push(Line::from(vec![
            Span::styled(
                format!("Tasks: {}", total),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Render each visible task
        for (idx, (task_id, depth, is_last)) in state.visible_tasks.iter().enumerate() {
            let is_selected = idx == state.selected_index;
            let is_collapsed = state.collapsed.contains(task_id);

            // Build tree prefix with proper connectors
            let mut prefix = String::new();
            for d in 0..*depth {
                if d == depth - 1 {
                    prefix.push_str(if *is_last { "└── " } else { "├── " });
                } else {
                    prefix.push_str("│   ");
                }
            }

            // For now, show task_id as placeholder - will be replaced with actual task data
            let collapse_icon = if is_collapsed { "▶ " } else { "▼ " };
            let status_icon = "○"; // Default pending icon

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let cursor = if is_selected { "> " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(cursor, style),
                Span::styled(prefix.clone(), Style::default().fg(Color::DarkGray)),
                Span::styled(collapse_icon, Style::default().fg(Color::Cyan)),
                Span::styled(status_icon, style),
                Span::styled(" ", Style::default()),
                Span::styled(task_id.clone(), style),
            ]));
        }
    }

    // If no visible_tasks but we have cache, show the cache
    if state.visible_tasks.is_empty() && !app.task_tree_cache.is_empty() && app.task_tree_cache != "No tasks" {
        lines.clear();
        lines.push(Line::from(vec![
            Span::styled(
                format!("Tasks: {}", app.task_count_cache),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Parse the cached tree (simple format for now)
        for (idx, line) in app.task_tree_cache.lines().enumerate() {
            let is_selected = idx == state.selected_index;
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let cursor = if is_selected { "> " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(cursor, style),
                Span::styled(line.to_string(), style),
            ]));
        }
    }

    let text = ratatui::text::Text::from(lines);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));

    f.render_widget(paragraph, area);
}

/// Render footer with key bindings
fn render_footer(f: &mut Frame, area: Rect) {
    let footer_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Green)),
            Span::raw(": navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(": expand/collapse  "),
            Span::styled("Space", Style::default().fg(Color::Green)),
            Span::raw(": toggle status  "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(": close"),
        ]),
    ];

    let footer_text = ratatui::text::Text::from(footer_lines);
    let footer_paragraph = Paragraph::new(footer_text)
        .alignment(Alignment::Center);

    f.render_widget(footer_paragraph, area);
}
