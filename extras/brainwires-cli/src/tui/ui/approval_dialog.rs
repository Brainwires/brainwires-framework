//! Approval dialog UI rendering.
//!
//! This module renders the tool approval dialog overlay.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::approval::types::ApprovalSeverity;
use crate::tui::app::App;

/// Draw the approval dialog overlay.
pub fn draw_approval_dialog(f: &mut Frame, app: &mut App, _area: Rect) {
    let Some(ref state) = app.approval_dialog_state else {
        return;
    };

    let Some(info) = state.get_display_info() else {
        return;
    };

    let screen = f.area();

    // Calculate modal size (centered, 60x16)
    let modal_width = 60.min(screen.width.saturating_sub(4));
    let modal_height = 16.min(screen.height.saturating_sub(4));

    // Center the modal
    let x = (screen.width.saturating_sub(modal_width)) / 2;
    let y = (screen.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background
    f.render_widget(Clear, modal_area);

    // Determine border color based on severity
    let border_color = match info.severity {
        ApprovalSeverity::High => Color::Red,
        ApprovalSeverity::Medium => Color::Yellow,
        ApprovalSeverity::Low => Color::Cyan,
    };

    // Outer border with title
    let title = format!(" {} Approval Required ", info.action_category);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout: Content | Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),     // Content
            Constraint::Length(3),  // Footer with buttons
        ])
        .split(inner);

    // Render content
    render_content(f, &info, chunks[0], border_color);

    // Render footer with keyboard shortcuts
    render_footer(f, chunks[1]);
}

/// Render the dialog content
fn render_content(f: &mut Frame, info: &crate::tui::app::approval_dialog::ApprovalDisplayInfo, area: Rect, accent_color: Color) {
    let mut lines = Vec::new();

    // Tool name
    lines.push(Line::from(vec![
        Span::styled("Tool: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&info.tool_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));

    lines.push(Line::from(""));

    // Action
    lines.push(Line::from(vec![
        Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&info.action_description, Style::default().fg(accent_color)),
    ]));

    lines.push(Line::from(""));

    // Tool description (truncated if too long)
    let desc = if info.tool_description.len() > 100 {
        format!("{}...", &info.tool_description[..100])
    } else {
        info.tool_description.clone()
    };

    lines.push(Line::from(vec![
        Span::styled("Description: ", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(desc, Style::default().fg(Color::Gray)),
    ]));

    lines.push(Line::from(""));

    // Show key parameters if present
    if let Some(obj) = info.parameters.as_object() {
        for (key, value) in obj.iter().take(3) {
            let value_str = match value {
                serde_json::Value::String(s) => {
                    if s.len() > 40 {
                        format!("\"{}...\"", &s[..40])
                    } else {
                        format!("\"{}\"", s)
                    }
                }
                other => {
                    let s = other.to_string();
                    if s.len() > 40 {
                        format!("{}...", &s[..40])
                    } else {
                        s
                    }
                }
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", key), Style::default().fg(Color::DarkGray)),
                Span::styled(value_str, Style::default().fg(Color::Gray)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render the footer with keyboard shortcuts
fn render_footer(f: &mut Frame, area: Rect) {
    let shortcuts = vec![
        Span::styled("[Y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("es  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[N]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("o  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[A]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("lways  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[D]", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled("eny always", Style::default().fg(Color::DarkGray)),
    ];

    let footer = Paragraph::new(Line::from(shortcuts))
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    // UI tests would require a terminal backend mock
    // For now, just ensure it compiles
}
