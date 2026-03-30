//! Help dialog UI rendering.
//!
//! This module renders the interactive help dialog overlay.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::{
    app::{help_dialog::HelpFocus, App},
    help_content::HelpCategory,
};

/// Draw the help dialog overlay.
pub fn draw_help_dialog(f: &mut Frame, app: &mut App, _area: Rect) {
    let Some(state) = &mut app.help_dialog_state else {
        return;
    };

    let screen = f.area();

    // Calculate modal size (80% of screen, max 100x40, min 60x20)
    let modal_width = (screen.width * 80 / 100).min(100).max(60).min(screen.width.saturating_sub(4));
    let modal_height = (screen.height * 80 / 100).min(40).max(20).min(screen.height.saturating_sub(4));

    // Center the modal
    let x = (screen.width.saturating_sub(modal_width)) / 2;
    let y = (screen.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background
    f.render_widget(Clear, modal_area);

    // Outer border with title
    let border_color = Color::Cyan;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Help (F1) ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Clear click regions before rendering
    state.clear_click_regions();

    // Layout: Search bar (3) | Main content | Footer (2)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search bar
            Constraint::Min(1),     // Main content
            Constraint::Length(2),  // Footer
        ])
        .split(inner);

    // Render search bar
    render_search_bar(f, app, main_chunks[0]);

    // Split main content: Categories (25%) | Content (75%)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(75),
        ])
        .split(main_chunks[1]);

    // Render category list
    render_category_list(f, app, content_chunks[0]);

    // Render content area
    render_content_area(f, app, content_chunks[1]);

    // Render footer
    render_footer(f, app, main_chunks[2]);
}

/// Render the search bar.
fn render_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &mut app.help_dialog_state else {
        return;
    };

    let is_focused = state.focus == HelpFocus::SearchInput;
    let border_color = if is_focused { Color::Yellow } else { Color::DarkGray };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" 🔍 Search ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build search text with cursor
    let text = if state.search_query.is_empty() && !is_focused {
        Line::from(Span::styled(
            "Type to filter...",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        let before = state.text_before_cursor();
        let after = state.text_after_cursor();

        let mut spans = vec![Span::styled(
            before.to_string(),
            Style::default().fg(Color::White),
        )];

        if is_focused {
            spans.push(Span::styled("│", Style::default().fg(Color::Yellow)));
        }

        spans.push(Span::styled(
            after.to_string(),
            Style::default().fg(Color::White),
        ));

        Line::from(spans)
    };

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

/// Render the category list.
fn render_category_list(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &mut app.help_dialog_state else {
        return;
    };

    let is_focused = state.focus == HelpFocus::CategoryList;
    let border_color = if is_focused { Color::Yellow } else { Color::DarkGray };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Categories ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let categories = HelpCategory::all();
    let mut lines = Vec::new();

    for (idx, category) in categories.iter().enumerate() {
        let is_selected = *category == state.selected_category;

        let prefix = if is_selected { "▶ " } else { "  " };
        let icon = category.icon();
        let name = category.display_name();

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let line = Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{} ", icon), style),
            Span::styled(name, style),
        ]);
        lines.push(line);

        // Register click region
        let row_y = inner.y + idx as u16;
        if row_y < inner.y + inner.height {
            state.add_click_region(
                Rect::new(inner.x, row_y, inner.width, 1),
                *category,
            );
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

/// Render the content area.
fn render_content_area(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &mut app.help_dialog_state else {
        return;
    };

    let is_focused = state.focus == HelpFocus::ContentArea;
    let border_color = if is_focused { Color::Yellow } else { Color::DarkGray };

    // Title shows category name or "Search Results"
    let title = if state.is_searching() {
        let count = state.get_search_results().len();
        format!(" Search Results ({}) ", count)
    } else {
        format!(" {} {} ", state.selected_category.icon(), state.selected_category.display_name())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Get entries to display
    let entries = state.get_current_entries();
    let total_entries = entries.len();

    if entries.is_empty() {
        let msg = if state.is_searching() {
            "No matching entries found"
        } else {
            "No entries in this category"
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(paragraph, inner);
        return;
    }

    // Build lines with proper formatting
    let mut lines = Vec::new();
    let max_shortcut_len = entries
        .iter()
        .map(|e| e.shortcut.chars().count())
        .max()
        .unwrap_or(12);

    for entry in &entries {
        // Pad shortcut for alignment
        let shortcut_padded = format!("{:width$}", entry.shortcut, width = max_shortcut_len);

        let line = Line::from(vec![
            Span::styled(shortcut_padded, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(&entry.description, Style::default().fg(Color::White)),
        ]);
        lines.push(line);
    }

    // Apply scroll
    let visible_height = inner.height as usize;
    let scroll = state.content_scroll.min(total_entries.saturating_sub(1));

    let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0));
    f.render_widget(paragraph, inner);

    // Render scrollbar if needed
    if total_entries > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ScrollbarState::new(total_entries)
            .position(scroll)
            .viewport_content_length(visible_height);

        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

/// Render the footer with key hints.
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let Some(state) = &app.help_dialog_state else {
        return;
    };

    let hints = match state.focus {
        HelpFocus::SearchInput => vec![
            ("Esc", "Clear/Close"),
            ("Tab", "Categories"),
            ("Enter", "Search"),
        ],
        HelpFocus::CategoryList => vec![
            ("↑/↓", "Navigate"),
            ("Tab", "Content"),
            ("Enter", "Select"),
            ("Esc/F1", "Close"),
        ],
        HelpFocus::ContentArea => vec![
            ("↑/↓", "Scroll"),
            ("PgUp/Dn", "Page"),
            ("Tab", "Search"),
            ("Esc/F1", "Close"),
        ],
    };

    let mut spans = Vec::new();
    for (idx, (key, desc)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  │  "));
        }
        spans.push(Span::styled(*key, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(": "));
        spans.push(Span::styled(*desc, Style::default().fg(Color::Gray)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}
