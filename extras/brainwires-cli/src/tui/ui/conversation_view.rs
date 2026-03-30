//! Conversation View UI
//!
//! Renders the main conversation panel.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{
    console_view::draw_console_view,
    render_markdown_to_lines,
    session_picker::draw_session_picker,
    shell_viewer::draw_shell_viewer,
};
use crate::tui::app::{App, AppMode, ConversationViewStyle, FocusedPanel, TuiMessage, ToolExecutionEntry};
use crate::tui::app::journal_tree::{JournalNodeId, JournalNodeKind, JournalNodePayload, JournalTreeState, RenderItem};

/// A unified entry for the Journal view that can be either a message or a tool execution
#[derive(Debug, Clone)]
pub enum JournalEntry {
    /// A conversation message
    Message(TuiMessage),
    /// A tool execution record
    ToolExecution(ToolExecutionEntry),
}

impl JournalEntry {
    /// Get the timestamp for sorting
    pub fn timestamp(&self) -> i64 {
        match self {
            JournalEntry::Message(m) => m.created_at,
            JournalEntry::ToolExecution(t) => t.executed_at,
        }
    }
}

/// Merge messages and tool executions into a chronologically sorted list
pub fn merge_journal_entries(messages: &[TuiMessage], tools: &[ToolExecutionEntry]) -> Vec<JournalEntry> {
    let mut entries: Vec<JournalEntry> = Vec::with_capacity(messages.len() + tools.len());

    // Add all messages
    for msg in messages {
        entries.push(JournalEntry::Message(msg.clone()));
    }

    // Add all tool executions
    for tool in tools {
        entries.push(JournalEntry::ToolExecution(tool.clone()));
    }

    // Sort by timestamp
    entries.sort_by_key(|e| e.timestamp());

    entries
}

/// Draw the conversation panel
pub fn draw_conversation(f: &mut Frame, app: &mut App, area: Rect) {
    // Handle session picker mode specially
    if app.mode == AppMode::SessionPicker {
        draw_session_picker(f, app, area);
        return;
    }

    // Handle console view mode specially
    if app.mode == AppMode::ConsoleView {
        draw_console_view(f, app, area);
        return;
    }

    // Handle shell viewer mode specially
    if app.mode == AppMode::ShellViewer {
        draw_shell_viewer(f, app, area);
        return;
    }

    let items = match app.conversation_view_style {
        ConversationViewStyle::Journal => {
            // Lazily rebuild tree if message/tool counts changed
            app.journal_tree.rebuild_if_stale(&app.messages, &app.tool_execution_history);
            let cursor = app.journal_tree.cursor;
            render_journal_tree_mut(&mut app.journal_tree, cursor)
        }
        ConversationViewStyle::Classic => {
            let entries = merge_journal_entries(&app.messages, &app.tool_execution_history);
            render_classic_style_entries(&entries)
        }
    };

    let border_color = if app.focused_panel == FocusedPanel::Conversation {
        Color::Cyan // Bright color when focused
    } else {
        Color::Blue // Dimmer when not focused
    };

    let view_mode = match app.conversation_view_style {
        ConversationViewStyle::Journal => "Journal",
        ConversationViewStyle::Classic => "Classic",
    };
    let title = if app.focused_panel == FocusedPanel::Conversation {
        format!(" {} [focused] (F9: toggle, F10: fullscreen) ", view_mode)
    } else {
        format!(" {} ", view_mode)
    };

    let conversation_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let conversation_text = Text::from(items);
    let conversation_paragraph = Paragraph::new(conversation_text)
        .block(conversation_block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));

    // Calculate and cache the wrapped line count for scroll bounds
    // The width is the inner width (area width minus 2 for borders)
    let inner_width = area.width.saturating_sub(2);
    app.conversation_line_count = conversation_paragraph.line_count(inner_width);

    // Clear the area first to prevent content from bleeding through
    f.render_widget(Clear, area);
    f.render_widget(conversation_paragraph, area);
}

/// Render journal entries (messages + tool executions) in journal style
pub fn render_journal_style_entries(entries: &[JournalEntry]) -> Vec<Line<'static>> {
    let mut items = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        match entry {
            JournalEntry::Message(msg) => {
                render_journal_message(&mut items, msg);
            }
            JournalEntry::ToolExecution(tool) => {
                render_journal_tool_execution(&mut items, tool);
            }
        }

        // Add spacing between entries
        if idx < entries.len() - 1 {
            items.push(Line::from(""));
        }
    }

    items
}

/// Render a single message in journal style
fn render_journal_message(items: &mut Vec<Line<'static>>, msg: &TuiMessage) {
    match msg.role.as_str() {
        "user" => {
            // User messages: green ">" prefix, content with green left margin
            let rendered_lines = render_markdown_to_lines(&msg.content);
            for (i, line) in rendered_lines.into_iter().enumerate() {
                let mut spans = vec![
                    if i == 0 {
                        Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("│ ", Style::default().fg(Color::Green))
                    },
                ];
                spans.extend(line.spans);
                items.push(Line::from(spans));
            }
        }
        "system" => {
            // System messages: dim yellow with [sys] prefix on first line
            let rendered_lines = render_markdown_to_lines(&msg.content);
            for (i, line) in rendered_lines.into_iter().enumerate() {
                let mut spans = vec![
                    if i == 0 {
                        Span::styled("[sys] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM))
                    } else {
                        Span::styled("      ", Style::default()) // Indent continuation
                    },
                ];
                // Apply dim yellow to system content
                for span in line.spans {
                    spans.push(Span::styled(
                        span.content.to_string(),
                        span.style.fg(Color::Yellow).add_modifier(Modifier::DIM),
                    ));
                }
                items.push(Line::from(spans));
            }
        }
        _ => {
            // Assistant messages (default): content flows naturally, no prefix
            let rendered_lines = render_markdown_to_lines(&msg.content);
            for line in rendered_lines {
                items.push(line);
            }
        }
    }
}

/// Render a tool execution in journal style
fn render_journal_tool_execution(items: &mut Vec<Line<'static>>, tool: &ToolExecutionEntry) {
    let icon = if tool.success { "✓" } else { "✗" };
    let icon_color = if tool.success { Color::Cyan } else { Color::Red };

    // Tool header line: icon + tool name + duration
    let duration_str = tool.duration_ms
        .map(|d| format!(" ({:.1}s)", d as f64 / 1000.0))
        .unwrap_or_default();

    items.push(Line::from(vec![
        Span::styled(
            format!("{} ", icon),
            Style::default().fg(icon_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            tool.tool_name.clone(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            duration_str,
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Parameters line (if not empty)
    if !tool.parameters_summary.is_empty() && tool.parameters_summary != "(no params)" {
        items.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                tool.parameters_summary.clone(),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    // Result line (collapsed, dimmed)
    if !tool.result_summary.is_empty() && tool.result_summary != "(no output)" {
        let result_preview = if tool.result_summary.len() > 100 {
            format!("{}...", &tool.result_summary[..97])
        } else {
            tool.result_summary.clone()
        };

        items.push(Line::from(vec![
            Span::styled("  → ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                result_preview,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
            ),
        ]));
    }
}

// ── Tree Journal Renderer ─────────────────────────────────────────────────────

/// Render the Journal tree (mutable to allow lazy recompute of render list).
pub fn render_journal_tree_mut(tree: &mut JournalTreeState, cursor: Option<JournalNodeId>) -> Vec<Line<'static>> {
    // Ensure render list is up to date
    let render_items: Vec<RenderItem> = tree.render_list().to_vec();

    if render_items.is_empty() {
        return vec![Line::from(Span::styled(
            "  No messages yet. Type something to start the conversation.",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    let mut lines: Vec<Line<'static>> = Vec::new();

    for item in &render_items {
        let node = match tree.nodes.get(&item.node_id) {
            Some(n) => n.clone(),
            None => continue,
        };

        let is_cursor = cursor == Some(item.node_id);

        // Build the indent + connector prefix string
        let prefix = build_tree_prefix(item);

        // Collapse/expand icon
        let expand_icon: &str = if item.has_children {
            if item.is_collapsed { "▶ " } else { "▼ " }
        } else {
            "  "  // two spaces to align with icon width
        };

        // Determine prefix style (tree lines are dark gray)
        let prefix_style = Style::default().fg(Color::DarkGray);

        // Cursor highlight for the whole line
        let cursor_bg = if is_cursor { Some(Color::DarkGray) } else { None };

        match &node.kind {
            JournalNodeKind::Turn => {
                let turn_label = JournalTreeState::summary_text(&node);
                let mut spans = vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(
                        expand_icon,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        turn_label,
                        apply_cursor(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), cursor_bg),
                    ),
                ];
                if let JournalNodePayload::Turn { turn_number } = &node.payload {
                    // Already in label
                    let _ = turn_number;
                }
                let line = Line::from(spans);
                lines.push(line);
            }

            JournalNodeKind::UserMessage => {
                let content = match &node.payload {
                    JournalNodePayload::Message { content, .. } => content.clone(),
                    _ => String::new(),
                };
                let rendered = render_markdown_to_lines(&content);
                for (i, rendered_line) in rendered.into_iter().enumerate() {
                    let connector = if i == 0 {
                        format!("{}{}> ", prefix, expand_icon)
                    } else {
                        format!("{}  │ ", prefix)
                    };
                    let mut spans = vec![Span::styled(
                        connector,
                        apply_cursor(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD), cursor_bg),
                    )];
                    for span in rendered_line.spans {
                        spans.push(Span::styled(
                            span.content.to_string(),
                            apply_cursor(span.style, cursor_bg),
                        ));
                    }
                    lines.push(Line::from(spans));
                }
            }

            JournalNodeKind::AssistantMessage => {
                let (content, collapsed) = match &node.payload {
                    JournalNodePayload::Message { content, .. } => (content.clone(), item.is_collapsed),
                    _ => (String::new(), false),
                };

                if collapsed {
                    // Show one-line summary
                    let summary = JournalTreeState::summary_text(&node);
                    let connector = format!("{}{}", prefix, expand_icon);
                    lines.push(Line::from(vec![
                        Span::styled(connector, Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            summary,
                            apply_cursor(Style::default().fg(Color::White), cursor_bg),
                        ),
                    ]));
                } else {
                    let rendered = render_markdown_to_lines(&content);
                    for (i, rendered_line) in rendered.into_iter().enumerate() {
                        if i == 0 {
                            let mut spans = vec![
                                Span::styled(prefix.clone(), prefix_style),
                                Span::styled(
                                    expand_icon,
                                    apply_cursor(Style::default().fg(Color::DarkGray), cursor_bg),
                                ),
                            ];
                            for span in rendered_line.spans {
                                spans.push(Span::styled(
                                    span.content.to_string(),
                                    apply_cursor(span.style, cursor_bg),
                                ));
                            }
                            lines.push(Line::from(spans));
                        } else {
                            let indent = format!("{}  ", prefix);
                            let mut spans = vec![Span::styled(indent, prefix_style)];
                            for span in rendered_line.spans {
                                spans.push(Span::styled(span.content.to_string(), apply_cursor(span.style, cursor_bg)));
                            }
                            lines.push(Line::from(spans));
                        }
                    }
                }
            }

            JournalNodeKind::ToolCall => {
                let (tool_name, params, result, success, duration_ms) = match &node.payload {
                    JournalNodePayload::Tool { tool_name, params_summary, result_summary, success, duration_ms } => {
                        (tool_name.clone(), params_summary.clone(), result_summary.clone(), *success, *duration_ms)
                    }
                    _ => (String::new(), String::new(), String::new(), true, None),
                };

                let icon = if success { "✓" } else { "✗" };
                let icon_color = if success { Color::Cyan } else { Color::Red };
                let duration_str = duration_ms.map(|d| format!(" ({:.1}s)", d as f64 / 1000.0)).unwrap_or_default();

                let mut spans = vec![
                    Span::styled(prefix.clone(), prefix_style),
                    Span::styled(expand_icon, prefix_style),
                    Span::styled(
                        format!("{} ", icon),
                        apply_cursor(Style::default().fg(icon_color).add_modifier(Modifier::BOLD), cursor_bg),
                    ),
                    Span::styled(
                        tool_name,
                        apply_cursor(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD), cursor_bg),
                    ),
                    Span::styled(
                        duration_str,
                        apply_cursor(Style::default().fg(Color::DarkGray), cursor_bg),
                    ),
                ];
                lines.push(Line::from(spans));

                // Parameters sub-line (only when not collapsed)
                if !item.is_collapsed && !params.is_empty() && params != "(no params)" {
                    let param_indent = format!("{}    ", prefix);
                    lines.push(Line::from(vec![
                        Span::styled(param_indent, prefix_style),
                        Span::styled(params, Style::default().fg(Color::Gray)),
                    ]));
                }

                // Result sub-line (only when not collapsed)
                if !item.is_collapsed && !result.is_empty() && result != "(no output)" {
                    let result_preview = if result.len() > 100 {
                        format!("{}…", &result[..97])
                    } else {
                        result
                    };
                    let result_indent = format!("{}  → ", prefix);
                    lines.push(Line::from(vec![
                        Span::styled(result_indent, Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            result_preview,
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                        ),
                    ]));
                }
            }

            JournalNodeKind::SubAgentSpawn => {
                let (agent_id, task_desc) = match &node.payload {
                    JournalNodePayload::SubAgentSpawn { agent_id, task_desc } => {
                        (agent_id.clone(), task_desc.clone())
                    }
                    _ => (String::new(), String::new()),
                };

                let summary = if item.is_collapsed {
                    format!("⚡ {} [{}]", task_desc, agent_id)
                } else {
                    format!("⚡ {}", task_desc)
                };

                lines.push(Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(
                        expand_icon,
                        apply_cursor(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), cursor_bg),
                    ),
                    Span::styled(
                        summary,
                        apply_cursor(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), cursor_bg),
                    ),
                ]));
            }

            JournalNodeKind::SystemEvent => {
                let description = match &node.payload {
                    JournalNodePayload::SystemEvent { description } => description.clone(),
                    _ => String::new(),
                };
                let rendered = render_markdown_to_lines(&description);
                for (i, rendered_line) in rendered.into_iter().enumerate() {
                    let lead = if i == 0 {
                        format!("{}{}[sys] ", prefix, expand_icon)
                    } else {
                        format!("{}       ", prefix)
                    };
                    let mut spans = vec![Span::styled(
                        lead,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM),
                    )];
                    for span in rendered_line.spans {
                        spans.push(Span::styled(
                            span.content.to_string(),
                            apply_cursor(
                                span.style.fg(Color::Yellow).add_modifier(Modifier::DIM),
                                cursor_bg,
                            ),
                        ));
                    }
                    lines.push(Line::from(spans));
                }
            }
        }

        // Blank line separator between top-level Turn nodes
        if node.kind == JournalNodeKind::Turn && !item.is_collapsed {
            // blank line after the Turn header (before children are listed)
        } else if node.parent.is_none() {
            lines.push(Line::from(""));
        }
    }

    lines
}

/// Build the indent + connector string for a render item
fn build_tree_prefix(item: &RenderItem) -> String {
    if item.depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    // For each ancestor level, draw vertical line or space
    for &has_more in &item.ancestor_has_more {
        if has_more {
            prefix.push_str("│   ");
        } else {
            prefix.push_str("    ");
        }
    }
    // Connector at this node's level
    if item.is_last_child {
        prefix.push_str("└── ");
    } else {
        prefix.push_str("├── ");
    }
    prefix
}

/// Apply optional cursor background highlight to a style
fn apply_cursor(style: Style, cursor_bg: Option<Color>) -> Style {
    if let Some(bg) = cursor_bg {
        style.bg(bg)
    } else {
        style
    }
}

/// Render messages in journal style (assistant is default speaker, no timestamps)
/// (Legacy function - wraps new implementation for backwards compatibility)
pub fn render_journal_style(messages: &[TuiMessage]) -> Vec<Line<'static>> {
    let entries: Vec<JournalEntry> = messages.iter()
        .map(|m| JournalEntry::Message(m.clone()))
        .collect();
    render_journal_style_entries(&entries)
}

/// Render journal entries in classic style (with role badges)
pub fn render_classic_style_entries(entries: &[JournalEntry]) -> Vec<Line<'static>> {
    let mut items = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        match entry {
            JournalEntry::Message(msg) => {
                render_classic_message(&mut items, msg);
            }
            JournalEntry::ToolExecution(tool) => {
                render_classic_tool_execution(&mut items, tool);
            }
        }

        // Add spacing
        if idx < entries.len() - 1 {
            items.push(Line::from(""));
        }
    }

    items
}

/// Render a single message in classic style
fn render_classic_message(items: &mut Vec<Line<'static>>, msg: &TuiMessage) {
    let role_style = match msg.role.as_str() {
        "user" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        "assistant" => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        "system" => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        _ => Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    };

    let role_text = msg.role.to_uppercase();
    let timestamp = chrono::DateTime::from_timestamp(msg.created_at, 0)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "??:??:??".to_string());

    // Message header
    items.push(Line::from(vec![
        Span::styled(role_text, role_style),
        Span::raw(" "),
        Span::styled(timestamp, Style::default().fg(Color::Gray)),
    ]));

    // Render message content as markdown
    let rendered_lines = render_markdown_to_lines(&msg.content);
    for line in rendered_lines {
        // Add indent to content lines
        let mut spans = vec![Span::raw("  ")];
        spans.extend(line.spans);
        items.push(Line::from(spans));
    }
}

/// Render a tool execution in classic style
fn render_classic_tool_execution(items: &mut Vec<Line<'static>>, tool: &ToolExecutionEntry) {
    let icon = if tool.success { "✓" } else { "✗" };
    let status_style = if tool.success {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let timestamp = chrono::DateTime::from_timestamp(tool.executed_at, 0)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "??:??:??".to_string());

    let duration_str = tool.duration_ms
        .map(|d| format!(" ({:.1}s)", d as f64 / 1000.0))
        .unwrap_or_default();

    // Tool header
    items.push(Line::from(vec![
        Span::styled(format!("TOOL {} ", icon), status_style),
        Span::styled(timestamp, Style::default().fg(Color::Gray)),
        Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
    ]));

    // Tool name
    items.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            tool.tool_name.clone(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Parameters
    if !tool.parameters_summary.is_empty() && tool.parameters_summary != "(no params)" {
        items.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Args: ", Style::default().fg(Color::Gray)),
            Span::styled(
                tool.parameters_summary.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Result
    if !tool.result_summary.is_empty() && tool.result_summary != "(no output)" {
        let result_preview = if tool.result_summary.len() > 80 {
            format!("{}...", &tool.result_summary[..77])
        } else {
            tool.result_summary.clone()
        };

        items.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Out:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                result_preview,
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
}

/// Render messages in classic style (with role badges)
/// (Legacy function - wraps new implementation for backwards compatibility)
pub fn render_classic_style(messages: &[TuiMessage]) -> Vec<Line<'static>> {
    let entries: Vec<JournalEntry> = messages.iter()
        .map(|m| JournalEntry::Message(m.clone()))
        .collect();
    render_classic_style_entries(&entries)
}
