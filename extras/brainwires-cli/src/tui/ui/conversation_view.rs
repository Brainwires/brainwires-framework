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

    // Merge messages and tool executions chronologically
    let entries = merge_journal_entries(&app.messages, &app.tool_execution_history);

    let items = match app.conversation_view_style {
        ConversationViewStyle::Journal => render_journal_style_entries(&entries),
        ConversationViewStyle::Classic => render_classic_style_entries(&entries),
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
