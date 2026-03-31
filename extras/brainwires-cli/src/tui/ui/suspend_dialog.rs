//! Suspend/Background Dialog UI
//!
//! Renders a centered modal dialog for suspend/background options.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use ratatui_interact::components::{CheckBox, CheckBoxStyle};

use crate::tui::app::{App, SuspendFocus};

/// Draw the suspend dialog as a centered modal overlay
pub fn draw_suspend_dialog(f: &mut Frame, app: &mut App) {
    let state = match &mut app.suspend_dialog_state {
        Some(s) => s,
        None => return,
    };

    // Clear click regions before rendering
    state.clear_click_regions();

    let screen = f.area();

    // Calculate modal size - compact dialog
    let modal_width = 52.min(screen.width.saturating_sub(4));
    let modal_height = 11; // Increased for checkbox

    // Center the modal
    let x = (screen.width.saturating_sub(modal_width)) / 2;
    let y = (screen.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear the area behind the modal
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Suspend Process ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Layout: Description | Spacing | Buttons | Spacing | Checkbox | Spacing | Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Description
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Buttons
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Checkbox
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Footer hints
        ])
        .split(inner);

    // Draw description
    let description = Line::from("Choose how to pause the application:");
    let desc_paragraph = Paragraph::new(description)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    f.render_widget(desc_paragraph, chunks[0]);

    // Draw buttons
    draw_buttons(f, app, chunks[2]);

    // Draw checkbox
    draw_checkbox(f, app, chunks[4]);

    // Draw footer hints
    let footer = Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::DarkGray)),
        Span::raw(": switch  "),
        Span::styled("Enter/Space", Style::default().fg(Color::DarkGray)),
        Span::raw(": select  "),
        Span::styled("Esc", Style::default().fg(Color::DarkGray)),
        Span::raw(": cancel"),
    ]);
    let footer_paragraph = Paragraph::new(footer).alignment(Alignment::Center);
    f.render_widget(footer_paragraph, chunks[6]);
}

fn draw_buttons(f: &mut Frame, app: &mut App, area: Rect) {
    let state = match &mut app.suspend_dialog_state {
        Some(s) => s,
        None => return,
    };

    let focus = state.focus;

    // Button labels with keyboard shortcuts
    let bg_label = " [B]ackground ";
    let suspend_label = " [S]uspend ";
    let spacing = 4;

    let bg_width = bg_label.len() as u16;
    let suspend_width = suspend_label.len() as u16;
    let total_width = bg_width + suspend_width + spacing as u16;
    let start_x = area.x + (area.width.saturating_sub(total_width)) / 2;

    // Background button style - green when focused
    let bg_style = if focus == SuspendFocus::BackgroundButton {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };

    // Register click region for Background button
    let bg_area = Rect::new(start_x, area.y, bg_width, 1);
    state.add_click_region(bg_area, SuspendFocus::BackgroundButton);

    // Suspend button style - yellow when focused
    let suspend_start = start_x + bg_width + spacing as u16;
    let suspend_style = if focus == SuspendFocus::SuspendButton {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };

    // Register click region for Suspend button
    let suspend_area = Rect::new(suspend_start, area.y, suspend_width, 1);
    state.add_click_region(suspend_area, SuspendFocus::SuspendButton);

    // Calculate padding to center the buttons
    let left_padding = (start_x.saturating_sub(area.x)) as usize;

    // Render buttons as a single line with proper spacing
    let line = Line::from(vec![
        Span::raw(" ".repeat(left_padding)),
        Span::styled(bg_label, bg_style),
        Span::raw(" ".repeat(spacing)),
        Span::styled(suspend_label, suspend_style),
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

fn draw_checkbox(f: &mut Frame, app: &mut App, area: Rect) {
    let state = match &mut app.suspend_dialog_state {
        Some(s) => s,
        None => return,
    };

    // Update focus state on the checkbox
    state.update_checkbox_focus();

    // Center the checkbox in the area
    let checkbox_label = "Exit when agent is done";

    // Calculate checkbox width manually to avoid borrowing state.exit_when_done
    // "[x] " or "[ ] " = 4 chars, plus label
    let checkbox_width = 4 + checkbox_label.len() as u16;
    let centered_x = area.x + (area.width.saturating_sub(checkbox_width)) / 2;
    let checkbox_area = Rect::new(centered_x, area.y, checkbox_width, 1);

    // Create checkbox and render it
    let checkbox =
        CheckBox::new(checkbox_label, &state.exit_when_done).style(CheckBoxStyle::default());
    checkbox.render(checkbox_area, f.buffer_mut());

    // Register click region (need to re-borrow state as mutable)
    let state = app.suspend_dialog_state.as_mut().unwrap();
    state.add_click_region(checkbox_area, SuspendFocus::ExitWhenDoneCheckbox);
}
