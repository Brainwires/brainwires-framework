//! TUI Rendering
//!
//! Renders the TUI layout using ratatui.

// Height breakpoints for responsive layout
// Below this height, show "terminal too small" message
const MIN_RENDER_HEIGHT: u16 = 12;
// Below this height, status bar is hidden
const HIDE_STATUS_HEIGHT: u16 = 18;

// Width breakpoints for task panel sidebar
// Below this width, task panel is hidden (shown in status bar instead)
const TASK_PANEL_MIN_WIDTH: u16 = 120;
// Width of the task panel sidebar
const TASK_PANEL_WIDTH: u16 = 30;

mod ansi_parser;
mod approval_dialog;
mod sudo_dialog;
mod console_view;
mod conversation_fullscreen;
mod conversation_view;
mod file_explorer;
mod find_replace;
mod git_scm;
mod help_dialog;
mod highlight;
mod hotkey_dialog;
mod input_area;
mod nano_editor;
mod question_panel;
mod session_picker;
mod shell_viewer;
mod suspend_dialog;
mod exit_dialog;
mod task_panel;
mod task_viewer;
mod tool_picker;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::{App, AppMode};

// Re-export for use in submodules
pub(crate) use ansi_parser::render_markdown_to_lines;

/// Draw the TUI
pub fn draw(f: &mut Frame, app: &mut App) {
    // Check if terminal is too small to render
    if f.area().height < MIN_RENDER_HEIGHT {
        draw_too_small(f);
        return;
    }

    // Full-screen modes: ConsoleView, ShellViewer, and ConversationFullscreen take over the entire screen
    if app.mode == AppMode::ConsoleView || app.mode == AppMode::ShellViewer {
        // In full-screen modes, the entire screen is the conversation area
        app.conversation_area = Some(f.area());
        conversation_view::draw_conversation(f, app, f.area());
        return;
    }

    // Full-screen conversation mode
    if app.mode == AppMode::ConversationFullscreen {
        // In full-screen conversation mode, the entire screen is the conversation area
        app.conversation_area = Some(f.area());
        conversation_fullscreen::draw_conversation_fullscreen(f, app, f.area());
        return;
    }

    // Full-screen input mode
    if app.mode == AppMode::InputFullscreen {
        input_area::draw_input_fullscreen(f, app, f.area());
        return;
    }

    // Tool picker mode takes over the entire screen
    if app.mode == AppMode::ToolPicker {
        tool_picker::draw_tool_picker(f, app, f.area());
        return;
    }

    // Task viewer mode is an overlay on top of normal content
    if app.mode == AppMode::TaskViewer {
        task_viewer::draw_task_viewer(f, app);
        return;
    }

    // File explorer mode takes over the entire screen
    if app.mode == AppMode::FileExplorer {
        file_explorer::draw_file_explorer(f, app, f.area());
        return;
    }

    // Nano editor mode takes over the entire screen
    if app.mode == AppMode::NanoEditor {
        nano_editor::draw_nano_editor(f, app, f.area());
        return;
    }

    // Git SCM mode takes over the entire screen
    if app.mode == AppMode::GitScm {
        git_scm::draw_git_scm(f, app, f.area());
        return;
    }

    // Question answer mode renders as an overlay on top of normal content
    if app.mode == AppMode::QuestionAnswer {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the question panel overlay
        if let Some(ref questions) = app.pending_questions {
            question_panel::draw_question_panel(f, questions, &app.question_state, f.area());
        }
        return;
    }

    // Find/Replace dialog modes render as overlay on top of fullscreen content
    if app.mode == AppMode::FindDialog || app.mode == AppMode::FindReplaceDialog {
        use super::app::FindReplaceContext;

        // First draw the fullscreen content underneath based on context
        if let Some(ref state) = app.find_replace_state {
            match state.context {
                FindReplaceContext::ConversationView => {
                    app.conversation_area = Some(f.area());
                    conversation_fullscreen::draw_conversation_fullscreen(f, app, f.area());
                }
                FindReplaceContext::InputView => {
                    input_area::draw_input_fullscreen(f, app, f.area());
                }
            }
        }

        // Then draw the find/replace dialog overlay
        find_replace::draw_find_replace_dialog(f, app, f.area());
        return;
    }

    // Help dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::HelpDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the help dialog overlay
        help_dialog::draw_help_dialog(f, app, f.area());
        return;
    }

    // Suspend dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::SuspendDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the suspend dialog overlay
        suspend_dialog::draw_suspend_dialog(f, app);
        return;
    }

    // Exit dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::ExitDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the exit dialog overlay
        exit_dialog::draw_exit_dialog(f, app);
        return;
    }

    // Hotkey dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::HotkeyDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the hotkey dialog overlay
        hotkey_dialog::draw_hotkey_dialog(f, app, f.area());
        return;
    }

    // Approval dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::ApprovalDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the approval dialog overlay
        approval_dialog::draw_approval_dialog(f, app, f.area());
        return;
    }

    // Sudo password dialog mode renders as overlay on top of normal content
    if app.mode == AppMode::SudoPasswordDialog {
        // First draw the normal content underneath
        draw_normal_layout(f, app);

        // Then draw the sudo password dialog overlay
        sudo_dialog::draw_sudo_dialog(f, app, f.area());
        return;
    }

    // Plan mode uses normal layout with visual distinction (magenta border and [PLAN] indicator)
    if app.mode == AppMode::PlanMode {
        draw_normal_layout(f, app);
        return;
    }

    // Draw normal layout
    draw_normal_layout(f, app);
}

/// Draw the normal TUI layout (conversation, input, status bar)
fn draw_normal_layout(f: &mut Frame, app: &mut App) {
    // Determine which sections to show based on terminal dimensions
    let terminal_width = f.area().width;
    let terminal_height = f.area().height;
    let show_status = terminal_height >= HIDE_STATUS_HEIGHT;

    // Show task panel sidebar on wide screens when there are tasks
    let show_task_panel = terminal_width >= TASK_PANEL_MIN_WIDTH
        && !app.session_task_panel_cache.is_empty();

    // Update app state so event handlers know if status bar is visible
    app.status_bar_visible = show_status;

    // If showing task panel, first split horizontally
    let main_area = if show_task_panel {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(80),                      // Main content
                Constraint::Length(TASK_PANEL_WIDTH),     // Task panel
            ])
            .split(f.area());

        // Draw task panel on the right
        task_panel::draw_task_panel(f, app, h_chunks[1]);

        // Return the main content area
        h_chunks[0]
    } else {
        // No task panel - use full width
        f.area()
    };

    // Hide input section in SessionPicker mode
    let chunks = if app.mode == AppMode::SessionPicker {
        if !show_status {
            // Minimal layout: only conversation
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(main_area)
        } else {
            // Conversation + status bar
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),    // Conversation (session picker)
                    Constraint::Length(3),  // Status bar
                ])
                .split(main_area)
        }
    } else {
        // Calculate dynamic input height based on content
        // Count lines in input (split by newline) + 2 for borders
        // Note: We count newlines instead of using lines() because lines() doesn't count trailing newlines
        let newline_count = app.input_state.line_count().saturating_sub(1);
        let input_lines = (newline_count + 1).max(1) as u16;
        let input_height_needed = input_lines + 2; // +2 for top and bottom borders

        // Cap at 35% of screen height
        let max_input_height = (main_area.height * 35) / 100;
        let input_height = input_height_needed.min(max_input_height).max(3); // Minimum 3 lines

        if !show_status {
            // Minimal layout: conversation + input only
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),               // Conversation
                    Constraint::Length(input_height), // Input (dynamic)
                ])
                .split(main_area)
        } else {
            // Full layout: conversation + input + status bar
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),              // Conversation
                    Constraint::Length(input_height), // Input (dynamic)
                    Constraint::Length(3),            // Status bar
                ])
                .split(main_area)
        }
    };

    // Store conversation area for mouse hit testing
    app.conversation_area = Some(chunks[0]);

    conversation_view::draw_conversation(f, app, chunks[0]);

    if app.mode == AppMode::SessionPicker {
        app.input_area = None; // No input area in session picker mode
        if show_status {
            draw_status_bar(f, app, chunks[1], show_task_panel);
        }
    } else {
        // Store input area for mouse hit testing
        app.input_area = Some(chunks[1]);
        input_area::draw_input(f, app, chunks[1]);
        if show_status {
            draw_status_bar(f, app, chunks[2], show_task_panel);
        }
    }

    // Draw toast notification if active (on top of everything)
    if let Some(toast_msg) = app.get_toast() {
        draw_toast(f, toast_msg, chunks[0]); // Draw over conversation area
    }
}

/// Draw the status bar
///
/// `task_panel_visible` indicates whether the task panel sidebar is shown,
/// so we know whether to show task summary in the status bar.
fn draw_status_bar(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect, task_panel_visible: bool) {
    use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };
    use super::app::FocusedPanel;

    // Store the status bar area for mouse hit testing
    app.status_bar_area = Some(area);

    // Clear the area first to prevent content from bleeding through
    f.render_widget(Clear, area);

    // Create the outer block first
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Status ");

    let inner = status_block.inner(area);
    f.render_widget(status_block, area);

    // Split inner area: left side for status text, right side for exit button
    let exit_button_label = " Exit ";
    let exit_button_width = exit_button_label.len() as u16 + 2; // +2 for padding

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),                    // Status text (takes remaining space)
            Constraint::Length(exit_button_width), // Exit button
        ])
        .split(inner);

    // Draw the status text on the left
    let queued_count = app.queued_message_count();
    let mut status_spans = vec![
        Span::styled("brainwires-cli", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ];

    // Add [PLAN] indicator when in plan mode
    if app.mode == AppMode::PlanMode {
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled(
            "[PLAN]",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        ));
    }

    status_spans.extend([
        Span::raw(" | "),
        Span::styled(&app.session_id, Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::raw(app.get_status()),
    ]);

    // Add queued message indicator if there are queued messages
    if queued_count > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("📬 {} queued", queued_count),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        ));
    }

    // Add task summary when task panel sidebar is not visible
    if !task_panel_visible && !app.session_task_summary.is_empty() {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            &app.session_task_summary,
            Style::default().fg(Color::Cyan)
        ));
    }

    let status_text = vec![
        Line::from(status_spans),
        Line::from(vec![
            Span::styled("Ctrl+C", Style::default().fg(Color::Gray)),
            Span::raw(": quit | "),
            Span::styled("Ctrl+R", Style::default().fg(Color::Gray)),
            Span::raw(": history | "),
            Span::styled("Ctrl+L", Style::default().fg(Color::Gray)),
            Span::raw(": sessions | "),
            Span::styled("Ctrl+D", Style::default().fg(Color::Gray)),
            Span::raw(": console"),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Gray)),
            Span::raw(": switch focus | "),
            Span::styled("F10", Style::default().fg(Color::Gray)),
            Span::raw(": fullscreen"),
        ]),
    ];

    let status_paragraph = Paragraph::new(status_text)
        .alignment(Alignment::Left);

    f.render_widget(status_paragraph, chunks[0]);

    // Draw the exit button on the right (vertically centered)
    let button_area = chunks[1];
    let is_focused = app.focused_panel == FocusedPanel::StatusBar;

    // Calculate vertical center for the button (single line)
    let button_y = button_area.y + (button_area.height.saturating_sub(1)) / 2;
    let button_rect = Rect::new(button_area.x, button_y, button_area.width, 1);

    // Store the button area for mouse hit testing
    app.exit_button_area = Some(button_rect);

    // Style: highlight when focused
    let button_style = if is_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
    };

    let button_text = Line::from(Span::styled(exit_button_label, button_style));
    let button_paragraph = Paragraph::new(button_text)
        .alignment(Alignment::Center);

    f.render_widget(button_paragraph, button_rect);
}

/// Draw a centered toast notification
fn draw_toast(f: &mut Frame, message: &str, area: ratatui::layout::Rect) {
    use ratatui_interact::components::Toast;
    Toast::new(message).render_with_clear(area, f.buffer_mut());
}

/// Draw a "terminal too small" message when height is below MIN_RENDER_HEIGHT
fn draw_too_small(f: &mut Frame) {
    use ratatui::{
        layout::Alignment,
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Clear, Paragraph},
    };

    let area = f.area();

    // Clear the screen
    f.render_widget(Clear, area);

    // Create centered message
    let message = vec![
        Line::from(Span::styled(
            "Terminal too small",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("Need {} lines, have {}", MIN_RENDER_HEIGHT, area.height),
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(message).alignment(Alignment::Center);

    // Center vertically if possible
    let vertical_offset = if area.height >= 2 {
        (area.height - 2) / 2
    } else {
        0
    };

    let centered_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + vertical_offset,
        width: area.width,
        height: area.height.saturating_sub(vertical_offset),
    };

    f.render_widget(paragraph, centered_area);
}
