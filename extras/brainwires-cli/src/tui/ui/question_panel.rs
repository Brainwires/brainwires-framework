//! Question Panel UI
//!
//! Renders a modal overlay for answering clarifying questions from the AI.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::types::question::{QuestionAnswerState, QuestionBlock};

/// Draw the question panel as a centered modal overlay
pub fn draw_question_panel(
    f: &mut Frame,
    questions: &QuestionBlock,
    state: &QuestionAnswerState,
    _area: Rect,
) {
    let screen = f.area();

    // Calculate modal size (60% width, 60% height, clamped)
    let modal_width = (screen.width * 60 / 100).min(80).max(50);
    let modal_height = (screen.height * 60 / 100).min(25).max(15);

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

    // Get current question
    let current_q_idx = state.current_question_idx;
    let current_question = match questions.questions.get(current_q_idx) {
        Some(q) => q,
        None => return,
    };

    // Build the title with question counter
    let title = format!(
        " Clarifying Questions ({}/{}) ",
        current_q_idx + 1,
        questions.questions.len()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .title_alignment(Alignment::Center);

    // Calculate inner area
    let inner = block.inner(modal_area);

    // Render the outer block
    f.render_widget(block, modal_area);

    // Layout: question text, options, footer
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Question header
            Constraint::Min(5),     // Options
            Constraint::Length(3),  // Footer
        ])
        .split(inner);

    // Draw question header
    draw_question_header(f, current_question, chunks[0]);

    // Draw options
    draw_options(f, current_question, state, chunks[1]);

    // Draw footer
    draw_footer(f, state, questions, chunks[2]);
}

/// Draw the question text header
fn draw_question_header(
    f: &mut Frame,
    question: &crate::types::question::ClarifyingQuestion,
    area: Rect,
) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(Span::styled(&question.header, header_style)),
        Line::from(""),
        Line::from(Span::raw(&question.question)),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Draw the selectable options
fn draw_options(
    f: &mut Frame,
    question: &crate::types::question::ClarifyingQuestion,
    state: &QuestionAnswerState,
    area: Rect,
) {
    let q_idx = state.current_question_idx;
    let mut lines = Vec::new();

    // Draw each option
    for (opt_idx, option) in question.options.iter().enumerate() {
        let is_selected = state
            .selected_options
            .get(q_idx)
            .and_then(|opts| opts.get(opt_idx))
            .copied()
            .unwrap_or(false);

        let is_cursor = state.cursor_idx == opt_idx;

        // Determine the selection indicator
        let indicator = if question.multi_select {
            // Checkboxes for multi-select
            if is_selected { "☑" } else { "☐" }
        } else {
            // Radio buttons for single-select
            if is_selected { "◉" } else { "○" }
        };

        // Build the option line
        let cursor_indicator = if is_cursor { "▶ " } else { "  " };
        let option_style = if is_cursor {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::styled(cursor_indicator, option_style),
            Span::styled(format!("{} ", indicator), option_style),
            Span::styled(&option.label, option_style),
        ]);
        lines.push(line);

        // Add description if present (indented)
        if let Some(ref desc) = option.description {
            let desc_style = Style::default().fg(Color::DarkGray);
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(desc, desc_style),
            ]));
        }
    }

    // Add "Other" option
    let is_other_selected = state.other_selected.get(q_idx).copied().unwrap_or(false);
    let is_cursor_on_other = state.cursor_idx >= question.options.len();

    let other_indicator = if question.multi_select {
        if is_other_selected { "☑" } else { "☐" }
    } else {
        if is_other_selected { "◉" } else { "○" }
    };

    let cursor_indicator = if is_cursor_on_other { "▶ " } else { "  " };
    let other_style = if is_cursor_on_other {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if is_other_selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    lines.push(Line::from("")); // Spacing

    let other_text = state.other_text.get(q_idx).map(|s| s.as_str()).unwrap_or("");
    let other_display = if state.editing_other && is_cursor_on_other {
        format!("Other: [{}▎]", other_text)
    } else if !other_text.is_empty() {
        format!("Other: [{}]", other_text)
    } else {
        "Other: [type custom answer...]".to_string()
    };

    lines.push(Line::from(vec![
        Span::styled(cursor_indicator, other_style),
        Span::styled(format!("{} ", other_indicator), other_style),
        Span::styled(other_display, other_style),
    ]));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

/// Draw the footer with keybindings and action buttons
fn draw_footer(f: &mut Frame, state: &QuestionAnswerState, questions: &QuestionBlock, area: Rect) {
    let is_last = state.is_last_question(questions);
    let is_first = state.current_question_idx == 0;

    // Build keybinding hints
    let mut hints = vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
    ];

    if !is_first {
        hints.push(Span::styled("Shift+Tab", Style::default().fg(Color::Yellow)));
        hints.push(Span::raw(" Prev  "));
    }

    if !is_last {
        hints.push(Span::styled("Tab", Style::default().fg(Color::Yellow)));
        hints.push(Span::raw(" Next  "));
    }

    hints.push(Span::styled("Esc", Style::default().fg(Color::Yellow)));
    hints.push(Span::raw(" Skip  "));

    if is_last {
        hints.push(Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
        hints.push(Span::raw(" Submit"));
    }

    let hint_line = Line::from(hints);

    // Build action buttons line
    let skip_style = Style::default().fg(Color::DarkGray);
    let submit_style = if state.all_answered(questions) {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let buttons = Line::from(vec![
        Span::raw("                    "),
        Span::styled("[Skip All]", skip_style),
        Span::raw("  "),
        Span::styled(
            if is_last { "[Submit →]" } else { "[Continue →]" },
            submit_style,
        ),
    ]);

    let lines = vec![
        Line::from("─".repeat(area.width as usize)),
        hint_line,
        buttons,
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(paragraph, area);
}
