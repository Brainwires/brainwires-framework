//! Input Area UI
//!
//! Renders the input text area and autocomplete popup.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_interact::components::{CursorMode, ScrollMode, TextArea, TextAreaStyle, WrapMode};

use super::highlight::highlight_input_text;
use crate::tui::app::{App, AppMode, FocusedPanel, PromptMode};

/// Draw the input panel
pub fn draw_input(f: &mut Frame, app: &mut App, area: Rect) {
    // Draw autocomplete popup if visible (above input area)
    if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
        draw_autocomplete_popup(f, app, area);
    }

    let input_border_color = match app.mode {
        AppMode::Normal => {
            if app.focused_panel == FocusedPanel::Input {
                match app.prompt_mode {
                    PromptMode::Ask => Color::Blue,
                    PromptMode::Edit => Color::Cyan,
                    PromptMode::Plan => Color::Magenta,
                }
            } else {
                Color::Green
            }
        }
        AppMode::ReverseSearch => Color::Yellow,
        AppMode::SessionPicker => Color::Cyan,
        AppMode::ConsoleView => Color::Magenta,
        AppMode::ShellViewer => Color::Yellow,
        AppMode::Waiting => match app.prompt_mode {
            PromptMode::Ask => Color::Blue,
            _ => Color::Gray,
        },
        AppMode::CancelConfirm => Color::Yellow,
        AppMode::ApprovalDialog => Color::Yellow,
        AppMode::SudoPasswordDialog => Color::Yellow,
        AppMode::PlanMode => Color::Magenta,
        _ => Color::Cyan,
    };

    let input_title: String = match app.mode {
        AppMode::Normal => {
            if app.focused_panel == FocusedPanel::Input {
                match app.prompt_mode {
                    PromptMode::Ask => " Ask [focused] (Enter: send, /edit to switch) ".to_string(),
                    PromptMode::Edit => {
                        " Edit [focused] (Enter: send, Alt+Enter: new line, F10: fullscreen) "
                            .to_string()
                    }
                    PromptMode::Plan => {
                        " Plan [focused] (Enter: send, /edit to switch) ".to_string()
                    }
                }
            } else {
                match app.prompt_mode {
                    PromptMode::Ask => " Ask (Tab to switch) ".to_string(),
                    PromptMode::Edit => " Edit (Tab to switch) ".to_string(),
                    PromptMode::Plan => " Plan (Tab to switch) ".to_string(),
                }
            }
        }
        AppMode::ReverseSearch => " Reverse Search (Esc to cancel) ".to_string(),
        AppMode::SessionPicker => " Session Picker ".to_string(),
        AppMode::ConsoleView => " Console View (Esc to exit) ".to_string(),
        AppMode::ShellViewer => " Shell History (Esc to exit) ".to_string(),
        AppMode::Waiting => {
            let queued = app.queued_message_count();
            let mode_prefix = match app.prompt_mode {
                PromptMode::Ask => "Ask - ",
                PromptMode::Edit => "",
                PromptMode::Plan => "Plan - ",
            };
            if queued > 0 {
                format!(
                    " {}Processing... ({} queued) - type to queue more ",
                    mode_prefix, queued
                )
            } else {
                format!(" {}Processing... (type to queue messages) ", mode_prefix)
            }
        }
        AppMode::CancelConfirm => " Cancel operation? (y: yes, n/Esc: no) ".to_string(),
        AppMode::PlanMode => {
            if app.focused_panel == FocusedPanel::Input {
                " Plan Mode [focused] (Ctrl+P or Esc to exit) ".to_string()
            } else {
                " Plan Mode (Ctrl+P or Esc to exit) ".to_string()
            }
        }
        AppMode::ApprovalDialog => " Tool Approval (y/n/a/d) ".to_string(),
        AppMode::SudoPasswordDialog => " Sudo Password ".to_string(),
        _ => " Input ".to_string(),
    };

    // For reverse search mode, override the input content shown
    if app.mode == AppMode::ReverseSearch {
        let search_text = format!("Search: {}", app.search_query);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(input_border_color))
            .title(input_title);
        f.render_widget(Clear, area);
        let paragraph = Paragraph::new(search_text).block(input_block);
        f.render_widget(paragraph, area);
        return;
    }

    let style = TextAreaStyle::default()
        .cursor_mode(CursorMode::Terminal)
        .scroll_mode(ScrollMode::CenterTracking);

    let textarea = TextArea::new()
        .title(Line::from(input_title))
        .placeholder("Type a message...")
        .style(style)
        .wrap_mode(WrapMode::Soft)
        .border_color(input_border_color);

    app.input_state.focused = app.focused_panel == FocusedPanel::Input;

    let render_result = textarea.render_stateful(f, area, &mut app.input_state);

    // Set terminal cursor if in appropriate mode
    if let Some((cx, cy)) = render_result.cursor_position
        && (app.mode == AppMode::Normal
            || app.mode == AppMode::Waiting
            || app.mode == AppMode::CancelConfirm
            || app.mode == AppMode::PlanMode)
    {
        f.set_cursor_position((cx, cy));
    }
}

/// Draw autocomplete popup above input area
fn draw_autocomplete_popup(f: &mut Frame, app: &App, input_area: Rect) {
    // Determine if we're showing models or commands
    let is_model_mode = app.autocomplete_title == "Models";

    // Calculate popup size - wider for model names
    let max_suggestions = 8;
    let total_suggestions = app.autocomplete_suggestions.len();
    let visible_suggestions = total_suggestions.min(max_suggestions);
    let popup_height = (visible_suggestions as u16 + 2).min(10); // +2 for borders
    let popup_width = if is_model_mode { 50 } else { 40 }.min(input_area.width);

    // Position popup directly above input area
    let popup_area = Rect {
        x: input_area.x,
        y: input_area.y.saturating_sub(popup_height),
        width: popup_width,
        height: popup_height,
    };

    // Clear the area first to mask underlying content
    f.render_widget(Clear, popup_area);

    // Create popup block with dynamic title
    let title = format!(" {} ", app.autocomplete_title);
    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner_area = popup_block.inner(popup_area);
    f.render_widget(popup_block, popup_area);

    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if app.autocomplete_index >= max_suggestions {
        app.autocomplete_index - max_suggestions + 1
    } else {
        0
    };

    // Render suggestions with scrolling
    let mut lines = Vec::new();
    for (i, item) in app
        .autocomplete_suggestions
        .iter()
        .skip(scroll_offset)
        .take(max_suggestions)
        .enumerate()
    {
        let actual_index = i + scroll_offset;
        let is_selected = actual_index == app.autocomplete_index;
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if is_selected { "► " } else { "  " };
        // For commands, prefix with '/'; for models, show as-is
        let display_text = if is_model_mode {
            item.clone()
        } else {
            format!("/{}", item)
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(display_text, style),
        ]));
    }

    let suggestions_paragraph = Paragraph::new(lines).alignment(Alignment::Left);

    f.render_widget(suggestions_paragraph, inner_area);
}

/// Draw the full-screen input view
pub fn draw_input_fullscreen(f: &mut Frame, app: &mut App, area: Rect) {
    // Clear the area
    f.render_widget(Clear, area);

    // Reserve space for help bar at bottom (2 lines + border)
    let help_bar_height = 3;
    let main_area_height = area.height.saturating_sub(help_bar_height);

    // Main input area
    let main_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: main_area_height,
    };

    // Help bar area at bottom
    let help_area = Rect {
        x: area.x,
        y: area.y + main_area_height,
        width: area.width,
        height: help_bar_height,
    };

    // Build title with line info
    let total_lines = app.input_state.line_count();
    let cursor_line = app.input_state.cursor_line + 1;
    let line_info = format!("Line {}/{}", cursor_line, total_lines);
    let mode_label = match app.prompt_mode {
        PromptMode::Ask => "Ask",
        PromptMode::Edit => "Input",
        PromptMode::Plan => "Plan",
    };
    let title = format!(" {} (Fullscreen) | {} ", mode_label, line_info);

    let style = TextAreaStyle::default()
        .cursor_mode(CursorMode::Terminal)
        .scroll_mode(ScrollMode::CenterTracking);

    let mut textarea = TextArea::new()
        .title(Line::from(title))
        .style(style)
        .wrap_mode(WrapMode::Soft)
        .border_color(Color::Cyan);

    // Apply search highlighting if in find/replace dialog mode
    if (app.mode == AppMode::FindDialog || app.mode == AppMode::FindReplaceDialog)
        && app.find_replace_state.is_some()
    {
        let state = app.find_replace_state.as_ref().unwrap();
        if !state.find_query.is_empty() {
            let input_text = app.input_text();
            let (highlighted_lines, _total, _line_nums) = highlight_input_text(
                &input_text,
                &state.find_query,
                state.case_sensitive,
                state.current_match_index,
            );
            textarea = textarea.content_lines(highlighted_lines);
        }
    }

    app.input_state.focused = true;

    let render_result = textarea.render_stateful(f, main_area, &mut app.input_state);

    // Set cursor position
    if let Some((cx, cy)) = render_result.cursor_position {
        f.set_cursor_position((cx, cy));
    }

    // Draw help bar
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let help_text = vec![Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::raw(": send | "),
        Span::styled("Alt+Enter", Style::default().fg(Color::Green)),
        Span::raw(": new line | "),
        Span::styled("Esc", Style::default().fg(Color::Green)),
        Span::raw(" or "),
        Span::styled("F10", Style::default().fg(Color::Green)),
        Span::raw(": exit fullscreen"),
    ])];

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .alignment(Alignment::Center);

    f.render_widget(help_paragraph, help_area);
}
