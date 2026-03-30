//! Tool Picker UI
//!
//! Renders the tool picker for selecting specific tools (explicit mode).
//! Tools are organized by category with collapsible sections.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Draw the tool picker
pub fn draw_tool_picker(f: &mut Frame, app: &App, area: Rect) {
    let picker_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Tool Picker (Space: toggle, Enter: confirm, A: all, N: none, Esc: cancel) ");

    let inner_area = picker_block.inner(area);
    f.render_widget(picker_block, area);

    let Some(picker_state) = &app.tool_picker_state else {
        return;
    };

    // Reserve last 3 lines for footer
    let content_height = inner_area.height.saturating_sub(3);
    let list_area = Rect {
        x: inner_area.x,
        y: inner_area.y,
        width: inner_area.width,
        height: content_height,
    };

    let mut items = Vec::new();

    // Header showing selected count
    let total_tools: usize = picker_state.categories.iter()
        .map(|(_, tools)| tools.len())
        .sum();
    let selected_count: usize = picker_state.categories.iter()
        .flat_map(|(_, tools)| tools.iter())
        .filter(|(_, _, selected)| *selected)
        .count();

    items.push(Line::from(vec![
        Span::styled(
            format!("Selected: {}/{} tools", selected_count, total_tools),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]));
    items.push(Line::from(""));

    // Track which item we're on for cursor positioning
    let mut item_index = 0;
    let cursor_row = calculate_cursor_row(picker_state);

    // Render categories and tools
    for (cat_idx, (category_name, tools)) in picker_state.categories.iter().enumerate() {
        let is_collapsed = picker_state.collapsed.contains(&cat_idx);
        let is_cat_selected = picker_state.selected_category == cat_idx && picker_state.selected_tool.is_none();

        // Count selected in this category
        let cat_selected = tools.iter().filter(|(_, _, s)| *s).count();
        let collapse_icon = if is_collapsed { "▶" } else { "▼" };

        let cat_style = if is_cat_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        };

        let prefix = if is_cat_selected { "> " } else { "  " };

        items.push(Line::from(vec![
            Span::styled(prefix, cat_style),
            Span::styled(collapse_icon, cat_style),
            Span::styled(" ", Style::default()),
            Span::styled(category_name.clone(), cat_style),
            Span::styled(
                format!(" ({}/{})", cat_selected, tools.len()),
                Style::default().fg(Color::Gray),
            ),
        ]));
        item_index += 1;

        // Render tools if not collapsed
        if !is_collapsed {
            for (tool_idx, (name, description, selected)) in tools.iter().enumerate() {
                let is_tool_selected = picker_state.selected_category == cat_idx
                    && picker_state.selected_tool == Some(tool_idx);

                let checkbox = if *selected { "[x]" } else { "[ ]" };
                let prefix = if is_tool_selected { " > " } else { "   " };

                let style = if is_tool_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if *selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                items.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(checkbox, style),
                    Span::raw(" "),
                    Span::styled(name.clone(), style),
                ]));

                // Show description on hover (when selected)
                if is_tool_selected {
                    let desc_preview: String = description.chars().take(60).collect();
                    items.push(Line::from(vec![
                        Span::raw("       "),
                        Span::styled(
                            if description.len() > 60 {
                                format!("{}...", desc_preview)
                            } else {
                                desc_preview
                            },
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                }

                item_index += 1;
            }
        }

        // Add spacing between categories
        items.push(Line::from(""));
    }

    // Calculate scroll position to keep cursor visible
    let visible_height = content_height.saturating_sub(2) as usize; // Account for header
    let scroll = if cursor_row >= picker_state.scroll as usize + visible_height {
        (cursor_row - visible_height + 1) as u16
    } else if (cursor_row as u16) < picker_state.scroll {
        cursor_row as u16
    } else {
        picker_state.scroll
    };

    let picker_text = ratatui::text::Text::from(items);
    let picker_paragraph = Paragraph::new(picker_text)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(picker_paragraph, list_area);

    // Footer with instructions
    let footer_area = Rect {
        x: inner_area.x,
        y: inner_area.y + content_height,
        width: inner_area.width,
        height: 3,
    };

    let footer_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Green)),
            Span::raw(": navigate  "),
            Span::styled("Space", Style::default().fg(Color::Green)),
            Span::raw(": toggle  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(": confirm  "),
            Span::styled("A", Style::default().fg(Color::Green)),
            Span::raw(": all  "),
            Span::styled("N", Style::default().fg(Color::Green)),
            Span::raw(": none  "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(": cancel"),
        ]),
    ];

    let footer_text = ratatui::text::Text::from(footer_lines);
    let footer_paragraph = Paragraph::new(footer_text)
        .alignment(Alignment::Center);

    f.render_widget(footer_paragraph, footer_area);
}

/// Calculate the current cursor row based on picker state
fn calculate_cursor_row(state: &crate::tui::app::ToolPickerState) -> usize {
    let mut row = 2; // Start after header

    for (cat_idx, (_, tools)) in state.categories.iter().enumerate() {
        if cat_idx == state.selected_category {
            if state.selected_tool.is_none() {
                return row;
            }
            row += 1; // Category header

            if !state.collapsed.contains(&cat_idx) {
                if let Some(tool_idx) = state.selected_tool {
                    row += tool_idx;
                    return row;
                }
            }
        } else {
            row += 1; // Category header
            if !state.collapsed.contains(&cat_idx) {
                row += tools.len();
            }
        }
        row += 1; // Spacing
    }

    row
}
