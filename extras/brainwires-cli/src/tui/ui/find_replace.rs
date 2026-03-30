//! Find/Replace Dialog UI
//!
//! Renders a centered modal dialog for find/replace functionality with
//! clickable buttons and checkboxes.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::{App, DialogFocus, FindReplaceMode};

/// Draw the find/replace dialog as a centered modal overlay
pub fn draw_find_replace_dialog(f: &mut Frame, app: &mut App, _area: Rect) {
    let state = match &mut app.find_replace_state {
        Some(s) => s,
        None => return,
    };

    // Clear click regions before rendering
    state.clear_click_regions();

    let screen = f.area();

    // Calculate modal size based on mode
    let modal_height = match state.mode {
        FindReplaceMode::Find => 8,
        FindReplaceMode::Replace => 10,
    };
    let modal_width = (screen.width * 70 / 100).min(65).max(50);

    // Center the modal
    let x = (screen.width.saturating_sub(modal_width)) / 2;
    let y = (screen.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the area behind the modal
    f.render_widget(Clear, modal_area);

    // Determine title and border color
    let (title, border_color) = match state.mode {
        FindReplaceMode::Find => (" Find ", Color::Cyan),
        FindReplaceMode::Replace => (" Find & Replace ", Color::Yellow),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout sections
    let chunks = if state.mode == FindReplaceMode::Replace {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Find field
                Constraint::Length(1), // Replace field
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Options row (checkboxes)
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Buttons row
                Constraint::Length(1), // Status
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Find field
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Options row (checkboxes)
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Buttons row
                Constraint::Length(1), // Status
            ])
            .split(inner)
    };

    let focus = state.focus;
    let mode = state.mode.clone();

    // Draw find field
    let find_area = chunks[0];
    draw_input_field(
        f,
        "Find:    ",
        &state.find_query,
        state.find_cursor_pos,
        focus == DialogFocus::FindInput,
        find_area,
    );
    state.add_click_region(find_area, DialogFocus::FindInput);

    // Draw replace field (if in Replace mode)
    let (options_chunk, buttons_chunk, status_chunk) = if mode == FindReplaceMode::Replace {
        let replace_area = chunks[1];
        draw_input_field(
            f,
            "Replace: ",
            &state.replace_text,
            state.replace_cursor_pos,
            focus == DialogFocus::ReplaceInput,
            replace_area,
        );
        state.add_click_region(replace_area, DialogFocus::ReplaceInput);
        (chunks[3], chunks[5], chunks[6])
    } else {
        (chunks[2], chunks[4], chunks[5])
    };

    // Draw options row (checkboxes)
    draw_options_row(f, app, options_chunk);

    // Draw buttons row
    draw_buttons_row(f, app, buttons_chunk);

    // Draw status
    draw_status(f, app, status_chunk);
}

/// Draw an input field with label
fn draw_input_field(
    f: &mut Frame,
    label: &str,
    text: &str,
    cursor_pos: usize,
    focused: bool,
    area: Rect,
) {
    let label_style = Style::default().fg(Color::Gray);

    let (text_style, border_char) = if focused {
        (
            Style::default().fg(Color::White).bg(Color::DarkGray),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (Style::default().fg(Color::Gray), Style::default().fg(Color::DarkGray))
    };

    // Split text at cursor position for visual cursor rendering
    let (before_cursor, after_cursor) = if cursor_pos <= text.chars().count() {
        let byte_pos = text
            .char_indices()
            .nth(cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        (&text[..byte_pos], &text[byte_pos..])
    } else {
        (text, "")
    };

    // Calculate available width for input (after label)
    let label_len = label.chars().count() as u16;
    let input_width = area.width.saturating_sub(label_len + 2); // +2 for brackets

    // Truncate text if needed
    let display_before = before_cursor;
    let display_after = after_cursor;

    let cursor_indicator = if focused { "│" } else { "" };

    let line = Line::from(vec![
        Span::styled(label, label_style),
        Span::styled("[", border_char),
        Span::styled(display_before, text_style),
        Span::styled(cursor_indicator, Style::default().fg(Color::Yellow)),
        Span::styled(display_after, text_style),
        // Padding to fill the input area
        Span::styled(
            " ".repeat(input_width.saturating_sub(text.chars().count() as u16 + 1) as usize),
            text_style,
        ),
        Span::styled("]", border_char),
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

/// Draw options row with clickable checkboxes
fn draw_options_row(f: &mut Frame, app: &mut App, area: Rect) {
    let state = match &mut app.find_replace_state {
        Some(s) => s,
        None => return,
    };

    let focus = state.focus;

    // Calculate positions for checkboxes
    let case_start = area.x + 2;
    let case_text = if state.case_sensitive { "[x] Case sensitive" } else { "[ ] Case sensitive" };
    let case_width = case_text.len() as u16;

    let regex_start = case_start + case_width + 4;
    let regex_text = if state.use_regex { "[x] Regex" } else { "[ ] Regex" };
    let regex_width = regex_text.len() as u16;

    // Register click regions
    let case_area = Rect::new(case_start, area.y, case_width, 1);
    let regex_area = Rect::new(regex_start, area.y, regex_width, 1);
    state.add_click_region(case_area, DialogFocus::CaseCheckbox);
    state.add_click_region(regex_area, DialogFocus::RegexCheckbox);

    // Style based on focus
    let case_style = if focus == DialogFocus::CaseCheckbox {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let regex_style = if focus == DialogFocus::RegexCheckbox {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled(case_text, case_style),
        Span::raw("    "),
        Span::styled(regex_text, regex_style),
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

/// Draw buttons row
fn draw_buttons_row(f: &mut Frame, app: &mut App, area: Rect) {
    let state = match &mut app.find_replace_state {
        Some(s) => s,
        None => return,
    };

    let focus = state.focus;
    let mode = state.mode.clone();

    let mut spans = Vec::new();
    let mut x_offset: u16 = 2;

    // Helper to create a button
    let mut add_button = |label: &str, element: DialogFocus, spans: &mut Vec<Span>, x: &mut u16| {
        let is_focused = focus == element;
        let style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        let btn_text = format!(" {} ", label);
        let btn_width = btn_text.len() as u16;

        // Register click region
        let btn_area = Rect::new(area.x + *x, area.y, btn_width, 1);
        state.add_click_region(btn_area, element);

        spans.push(Span::styled(btn_text, style));
        spans.push(Span::raw("  "));
        *x += btn_width + 2;
    };

    // Next button (always shown)
    add_button("Next", DialogFocus::NextButton, &mut spans, &mut x_offset);

    // Replace buttons (only in Replace mode)
    if mode == FindReplaceMode::Replace {
        add_button("Replace", DialogFocus::ReplaceButton, &mut spans, &mut x_offset);
        add_button("Replace All", DialogFocus::ReplaceAllButton, &mut spans, &mut x_offset);
    }

    // Close hint
    spans.push(Span::styled("  Esc: close", Style::default().fg(Color::DarkGray)));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

/// Draw status line
fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let state = match &app.find_replace_state {
        Some(s) => s,
        None => return,
    };

    let is_error = state
        .status_message
        .as_ref()
        .map(|s| s.contains("No matches") || s.contains("Invalid") || s.contains("No match"))
        .unwrap_or(false);

    let status_style = if is_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Green)
    };

    let status_text = state.status_message.as_deref().unwrap_or("");

    let line = Line::from(Span::styled(status_text, status_style));
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
