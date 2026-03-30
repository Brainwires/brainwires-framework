//! Sub-Agent Viewer UI
//!
//! Split-pane view for inspecting and messaging sub-agents.
//! Left panel: scrollable agent list with status icons.
//! Right panel: selected agent's conversation/tool activity tree.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::agents::TaskAgentStatus;
use crate::tui::app::{App, SubAgentPanelFocus};
use crate::tui::app::journal_tree::JournalNodeKind;

use super::render_markdown_to_lines;
use super::conversation_view::render_journal_tree_mut;

/// Draw the Sub-Agent Viewer over the full terminal area.
pub fn draw_sub_agent_viewer(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let focused = app
        .sub_agent_viewer_state
        .as_ref()
        .map(|s| s.panel_focus.clone())
        .unwrap_or(SubAgentPanelFocus::Left);

    // Outer border
    let border_color = Color::Magenta;
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Sub-Agent Viewer  [Tab: switch panel  Esc: close  Ctrl+B: toggle] ");
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Split 30% list / 70% detail
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(inner);

    render_agent_list(f, app, chunks[0], &focused);
    render_agent_detail(f, app, chunks[1], &focused);
}

/// Render the left panel: scrollable list of agents
fn render_agent_list(f: &mut Frame, app: &App, area: Rect, focused: &SubAgentPanelFocus) {
    let state = match app.sub_agent_viewer_state.as_ref() {
        Some(s) => s,
        None => {
            f.render_widget(
                Paragraph::new("Loading…").block(Block::default().borders(Borders::RIGHT)),
                area,
            );
            return;
        }
    };

    let border_color = if *focused == SubAgentPanelFocus::Left {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Agents ({}) ", state.agent_list.len()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.agent_list.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No sub-agents running.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .agent_list
        .iter()
        .enumerate()
        .map(|(idx, agent)| {
            let is_selected = idx == state.selected_index;

            let (icon, icon_color) = status_icon(&agent.status);
            let session_badge = if agent.session_id.is_some() { " [S]" } else { "" };
            let ipc_badge = if agent.has_ipc_socket { " ●" } else { "" };

            let label = format!("{} {}{}{}", icon, agent.task_desc, session_badge, ipc_badge);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(icon_color)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    // Simple scroll offset
    let scroll = state.list_scroll as usize;
    let visible_items: Vec<ListItem> = items.into_iter().skip(scroll).collect();

    f.render_widget(List::new(visible_items), inner);
}

/// Render the right panel: selected agent's activity + optional input bar
fn render_agent_detail(f: &mut Frame, app: &mut App, area: Rect, focused: &SubAgentPanelFocus) {
    let (selected_index, panel_focus, detail_scroll, message_input) = match app.sub_agent_viewer_state.as_ref() {
        Some(s) => (
            s.selected_index,
            s.panel_focus.clone(),
            s.detail_scroll,
            s.message_input.clone(),
        ),
        None => {
            f.render_widget(
                Paragraph::new("No state").block(Block::default().borders(Borders::NONE)),
                area,
            );
            return;
        }
    };

    let agent = app
        .sub_agent_viewer_state
        .as_ref()
        .and_then(|s| s.agent_list.get(selected_index))
        .cloned();

    let border_color = if *focused == SubAgentPanelFocus::Right {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let has_ipc = agent.as_ref().map(|a| a.has_ipc_socket).unwrap_or(false);
    let right_focused = *focused == SubAgentPanelFocus::Right;

    // If agent has IPC socket and right panel is focused, reserve space for input bar
    let (detail_area, input_area) = if has_ipc && right_focused {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    // Header block
    let title = match &agent {
        Some(a) => {
            let (icon, _) = status_icon(&a.status);
            format!(" {} {} — {} iters ", icon, a.agent_id, a.iterations)
        }
        None => " (no agent selected) ".to_string(),
    };

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);
    let detail_inner = detail_block.inner(detail_area);
    f.render_widget(detail_block, detail_area);

    if agent.is_none() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Select an agent from the left panel (Tab to focus).",
                Style::default().fg(Color::DarkGray),
            )),
            detail_inner,
        );
        if let Some(ia) = input_area {
            render_input_bar(f, &message_input, ia, border_color);
        }
        return;
    }

    let agent = agent.unwrap();

    // Find the SubAgentSpawn node for this agent in the journal tree
    let spawn_node_id = app.journal_tree.nodes.values()
        .find(|n| {
            matches!(
                &n.payload,
                crate::tui::app::journal_tree::JournalNodePayload::SubAgentSpawn { agent_id, .. }
                    if agent_id == &agent.agent_id
            )
        })
        .map(|n| n.id);

    let content_lines: Vec<Line> = if let Some(spawn_id) = spawn_node_id {
        // Temporarily expand spawn node and render its subtree
        let was_collapsed = app.journal_tree.collapsed.contains(&spawn_id);
        if was_collapsed {
            app.journal_tree.collapsed.remove(&spawn_id);
            app.journal_tree.mark_dirty();
        }
        // Render the full tree but we only want the subtree under spawn_id
        // For simplicity, render only children of the spawn node
        let children: Vec<_> = app.journal_tree.nodes.get(&spawn_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        if children.is_empty() {
            vec![Line::from(Span::styled(
                "  No activity recorded yet for this agent.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            let mut lines = Vec::new();
            for child_id in &children {
                if let Some(child_node) = app.journal_tree.nodes.get(child_id).cloned() {
                    match &child_node.payload {
                        crate::tui::app::journal_tree::JournalNodePayload::Message { role, content } => {
                            let prefix = match role.as_str() {
                                "user" => Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                                _ => Span::styled("  ", Style::default()),
                            };
                            let rendered = render_markdown_to_lines(content);
                            for (i, line) in rendered.into_iter().enumerate() {
                                let p = if i == 0 { prefix.clone() } else { Span::raw("  ") };
                                let mut spans = vec![p];
                                spans.extend(line.spans);
                                lines.push(Line::from(spans));
                            }
                        }
                        crate::tui::app::journal_tree::JournalNodePayload::Tool {
                            tool_name, params_summary, result_summary, success, duration_ms,
                        } => {
                            let icon = if *success { "✓" } else { "✗" };
                            let icon_color = if *success { Color::Cyan } else { Color::Red };
                            let dur = duration_ms.map(|d| format!(" ({:.1}s)", d as f64 / 1000.0)).unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::styled(format!("{} ", icon), Style::default().fg(icon_color).add_modifier(Modifier::BOLD)),
                                Span::styled(tool_name.clone(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                                Span::styled(dur, Style::default().fg(Color::DarkGray)),
                            ]));
                            if !params_summary.is_empty() {
                                lines.push(Line::from(vec![
                                    Span::raw("  "),
                                    Span::styled(params_summary.clone(), Style::default().fg(Color::Gray)),
                                ]));
                            }
                        }
                        crate::tui::app::journal_tree::JournalNodePayload::SystemEvent { description } => {
                            lines.push(Line::from(vec![
                                Span::styled("[sys] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM)),
                                Span::styled(description.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM)),
                            ]));
                        }
                        _ => {}
                    }
                    lines.push(Line::from(""));
                }
            }
            lines
        }
    } else {
        vec![
            Line::from(Span::styled(
                format!("  Agent: {}", agent.agent_id),
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                format!("  Status: {}", agent.status),
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  No activity recorded in journal yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  (Activity will appear here once the agent starts working)",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let content_para = Paragraph::new(content_lines)
        .wrap(Wrap { trim: false })
        .scroll((detail_scroll, 0));
    f.render_widget(content_para, detail_inner);

    if let Some(ia) = input_area {
        render_input_bar(f, &message_input, ia, border_color);
    }
}

/// Render the message input bar at the bottom of the right panel
fn render_input_bar(f: &mut Frame, input: &str, area: Rect, border_color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Send message to agent (Enter to send) ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let display = format!("{}_", input); // cursor blink simulation
    f.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(Color::White))),
        inner,
    );
}

/// Returns (icon_str, color) for a TaskAgentStatus
fn status_icon(status: &TaskAgentStatus) -> (&'static str, Color) {
    match status {
        TaskAgentStatus::Idle => ("·", Color::Gray),
        TaskAgentStatus::Working(_) => ("⟳", Color::Yellow),
        TaskAgentStatus::WaitingForLock(_) => ("⏳", Color::Yellow),
        TaskAgentStatus::Paused(_) => ("⏸", Color::DarkGray),
        TaskAgentStatus::Completed(_) => ("✓", Color::Green),
        TaskAgentStatus::Failed(_) => ("✗", Color::Red),
    }
}
