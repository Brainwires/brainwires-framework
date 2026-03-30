//! Full-Screen Conversation View UI
//!
//! Renders the conversation panel in full-screen mode with mouse toggle support.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_interact::components::ParagraphExt;

use super::conversation_view::{render_classic_style_entries, render_journal_style_entries, merge_journal_entries};
use super::highlight::apply_highlights_to_lines;
use crate::tui::app::{App, AppMode, ConversationViewStyle};

/// Draw the full-screen conversation view
pub fn draw_conversation_fullscreen(f: &mut Frame, app: &mut App, area: Rect) {
    // Generate items based on view style
    let items = if app.messages.is_empty() && app.tool_execution_history.is_empty() {
        vec![
            Line::from(vec![Span::styled(
                "No messages yet",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Start a conversation by typing in the input field."),
            ]),
        ]
    } else {
        // Merge messages and tool executions chronologically
        let entries = merge_journal_entries(&app.messages, &app.tool_execution_history);
        match app.conversation_view_style {
            ConversationViewStyle::Journal => render_journal_style_entries(&entries),
            ConversationViewStyle::Classic => render_classic_style_entries(&entries),
        }
    };

    // Apply search highlighting if in find dialog mode
    let items = if (app.mode == AppMode::FindDialog || app.mode == AppMode::FindReplaceDialog)
        && app.find_replace_state.is_some()
    {
        let state = app.find_replace_state.as_ref().unwrap();
        if !state.find_query.is_empty() {
            let (highlighted, _total, _line_nums) = apply_highlights_to_lines(
                &state.find_query,
                state.case_sensitive,
                state.current_match_index,
                items,
            );
            highlighted
        } else {
            items
        }
    } else {
        items
    };

    // When mouse capture is disabled, use clean widget for copy/paste
    if app.mouse_capture_disabled {
        let widget = ParagraphExt::new(items).scroll(app.scroll).width(area.width);
        app.conversation_line_count = widget.line_count(area.width);
        f.render_widget(widget, area);
    } else {
        // Normal rendering with borders
        let mouse_indicator = "m:Mouse";
        let view_mode = match app.conversation_view_style {
            ConversationViewStyle::Journal => "Journal",
            ConversationViewStyle::Classic => "Classic",
        };
        let title = Line::from(vec![
            Span::raw(format!(" Conversation ({}) ── ", view_mode)),
            Span::styled("F9", Style::default().fg(Color::Green)),
            Span::raw(":View "),
            Span::styled("F10/Esc", Style::default().fg(Color::Green)),
            Span::raw(":Exit "),
            Span::styled("c", Style::default().fg(Color::Green)),
            Span::raw(":Copy "),
            Span::styled(mouse_indicator, Style::default().fg(Color::Green)),
            Span::raw(" "),
        ]);

        let conversation_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title);

        let conversation_text = Text::from(items);
        let conversation_paragraph = Paragraph::new(conversation_text)
            .block(conversation_block)
            .wrap(Wrap { trim: false })
            .scroll((app.scroll, 0));

        // Calculate and cache the wrapped line count for scroll bounds
        let inner_width = area.width.saturating_sub(2);
        app.conversation_line_count = conversation_paragraph.line_count(inner_width);

        f.render_widget(conversation_paragraph, area);
    }

    // Draw toast notification if active
    if let Some(toast_msg) = app.get_toast() {
        draw_toast(f, toast_msg, area);
    }
}

/// Draw a centered toast notification
fn draw_toast(f: &mut Frame, message: &str, area: Rect) {
    use ratatui_interact::components::Toast;
    Toast::new(message).render_with_clear(area, f.buffer_mut());
}
