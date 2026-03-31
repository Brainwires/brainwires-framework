//! Dialog Event Handlers
//!
//! Event handling for help, suspend, exit, hotkey, approval, and find/replace dialogs.

use crate::tui::Event;
use crate::tui::app::state::{App, AppMode};
use anyhow::Result;
use crossterm::event::KeyCode;

impl App {
    /// Handle events in find/replace dialog mode
    pub(in crate::tui::app) async fn handle_find_replace_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use super::super::find_replace::{DialogFocus, FindReplaceContext};
        use crossterm::event::KeyModifiers;

        let state = match &mut self.find_replace_state {
            Some(s) => s,
            None => {
                // No state, return to previous mode
                self.mode = AppMode::Normal;
                return Ok(());
            }
        };

        let context = state.context.clone();
        let focus = state.focus;

        // Handle mouse clicks
        if let Event::Mouse(mouse) = &event {
            use crossterm::event::{MouseButton, MouseEventKind};
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(clicked) = state.handle_click(mouse.column, mouse.row)
            {
                // Handle the click based on what was clicked
                match clicked {
                    DialogFocus::FindInput | DialogFocus::ReplaceInput => {
                        state.set_focus(clicked);
                    }
                    DialogFocus::CaseCheckbox => {
                        state.set_focus(clicked);
                        state.toggle_case_sensitive();
                        // Update matches
                        let text = if context == FindReplaceContext::InputView {
                            self.input_text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        if let Some(ref mut state) = self.find_replace_state {
                            state.update_matches(&text);
                        }
                    }
                    DialogFocus::RegexCheckbox => {
                        state.set_focus(clicked);
                        state.toggle_regex();
                        // Update matches
                        let text = if context == FindReplaceContext::InputView {
                            self.input_text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        if let Some(ref mut state) = self.find_replace_state {
                            state.update_matches(&text);
                        }
                    }
                    DialogFocus::NextButton => {
                        state.set_focus(clicked);
                        self.find_replace_next_match();
                    }
                    DialogFocus::ReplaceButton => {
                        state.set_focus(clicked);
                        self.find_replace_do_replace();
                    }
                    DialogFocus::ReplaceAllButton => {
                        state.set_focus(clicked);
                        self.find_replace_do_replace_all();
                    }
                }
                return Ok(());
            }
        }

        // Escape closes the dialog
        if event.is_escape() {
            self.find_replace_state = None;
            self.mode = match context {
                FindReplaceContext::ConversationView => AppMode::ConversationFullscreen,
                FindReplaceContext::InputView => AppMode::InputFullscreen,
            };
            return Ok(());
        }

        // Tab cycles through focusable elements
        if event.is_tab() {
            if let Some(ref mut state) = self.find_replace_state {
                state.focus_next();
            }
            return Ok(());
        }

        // Shift+Tab cycles backward
        if event.is_shift_tab() {
            if let Some(ref mut state) = self.find_replace_state {
                state.focus_prev();
            }
            return Ok(());
        }

        // Enter activates the focused element
        if event.is_enter() {
            match focus {
                DialogFocus::FindInput => {
                    // In find input, Enter goes to next match
                    self.find_replace_next_match();
                }
                DialogFocus::ReplaceInput => {
                    // In replace input, Enter does replace
                    self.find_replace_do_replace();
                }
                DialogFocus::CaseCheckbox => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.toggle_case_sensitive();
                        let text = if state.context == FindReplaceContext::InputView {
                            self.input_state.text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        state.update_matches(&text);
                    }
                }
                DialogFocus::RegexCheckbox => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.toggle_regex();
                        let text = if state.context == FindReplaceContext::InputView {
                            self.input_state.text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        state.update_matches(&text);
                    }
                }
                DialogFocus::NextButton => {
                    self.find_replace_next_match();
                }
                DialogFocus::ReplaceButton => {
                    self.find_replace_do_replace();
                }
                DialogFocus::ReplaceAllButton => {
                    self.find_replace_do_replace_all();
                }
            }
            return Ok(());
        }

        // Space toggles checkboxes when focused
        if let Event::Key(key) = &event
            && key.code == KeyCode::Char(' ')
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            match focus {
                DialogFocus::CaseCheckbox => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.toggle_case_sensitive();
                        let text = if state.context == FindReplaceContext::InputView {
                            self.input_state.text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        state.update_matches(&text);
                    }
                    return Ok(());
                }
                DialogFocus::RegexCheckbox => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.toggle_regex();
                        let text = if state.context == FindReplaceContext::InputView {
                            self.input_state.text()
                        } else {
                            self.messages
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        };
                        state.update_matches(&text);
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        // Handle key events for input fields
        if let Event::Key(key) = event {
            // Only handle character input when an input field is focused
            let is_input_focused =
                matches!(focus, DialogFocus::FindInput | DialogFocus::ReplaceInput);

            match key.code {
                // Character input (only for input fields)
                KeyCode::Char(c)
                    if is_input_focused
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.insert_char(c);
                        // Update matches live as user types in find field
                        if state.focus == DialogFocus::FindInput {
                            let text = if state.context == FindReplaceContext::InputView {
                                self.input_state.text()
                            } else {
                                self.messages
                                    .iter()
                                    .map(|m| m.content.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n\n")
                            };
                            state.update_matches(&text);
                        }
                    }
                }
                // Backspace (only for input fields)
                KeyCode::Backspace if is_input_focused => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.delete_char_backward();
                        // Update matches live
                        if state.focus == DialogFocus::FindInput {
                            let text = if state.context == FindReplaceContext::InputView {
                                self.input_state.text()
                            } else {
                                self.messages
                                    .iter()
                                    .map(|m| m.content.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n\n")
                            };
                            state.update_matches(&text);
                        }
                    }
                }
                // Left arrow (only for input fields)
                KeyCode::Left if is_input_focused => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.move_cursor_left();
                    }
                }
                // Right arrow (only for input fields)
                KeyCode::Right if is_input_focused => {
                    if let Some(ref mut state) = self.find_replace_state {
                        state.move_cursor_right();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Helper: Go to next match in find/replace
    fn find_replace_next_match(&mut self) {
        use super::super::find_replace::{FindReplaceContext, byte_to_char_index};
        if let Some(ref mut state) = self.find_replace_state {
            state.next_match();
            if state.context == FindReplaceContext::InputView
                && let Some((start, _end)) = state.current_match_position()
            {
                let text = self.input_text();
                let char_pos = byte_to_char_index(&text, start);
                // Convert flat char position to line/col
                let mut remaining = char_pos;
                for (i, line) in self.input_state.lines.iter().enumerate() {
                    let line_chars = line.chars().count();
                    if remaining <= line_chars {
                        self.input_state.cursor_line = i;
                        self.input_state.cursor_col = remaining;
                        break;
                    }
                    remaining -= line_chars + 1; // +1 for newline
                }
            }
        }
    }

    /// Helper: Replace current match
    fn find_replace_do_replace(&mut self) {
        use super::super::find_replace::{FindReplaceContext, FindReplaceMode};
        if let Some(ref mut state) = self.find_replace_state
            && state.mode == FindReplaceMode::Replace
            && state.context == FindReplaceContext::InputView
        {
            let mut text = self.input_state.text();
            match state.replace_current(&mut text) {
                Ok(()) => {
                    self.input_state.set_text(text);
                }
                Err(e) => {
                    state.status_message = Some(e);
                }
            }
        }
    }

    /// Helper: Replace all matches
    fn find_replace_do_replace_all(&mut self) {
        use super::super::find_replace::{FindReplaceContext, FindReplaceMode};
        if let Some(ref mut state) = self.find_replace_state
            && state.mode == FindReplaceMode::Replace
            && state.context == FindReplaceContext::InputView
        {
            let mut text = self.input_state.text();
            match state.replace_all(&mut text) {
                Ok(_count) => {
                    self.input_state.set_text(text);
                }
                Err(e) => {
                    state.status_message = Some(e);
                }
            }
        }
    }

    /// Handle events in help dialog mode
    pub(in crate::tui::app) async fn handle_help_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use super::super::help_dialog::HelpFocus;

        let Some(state) = &mut self.help_dialog_state else {
            // No state, return to normal mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Handle mouse scroll events
        if let Some((_col, _row)) = event.mouse_scroll_up() {
            state.scroll_content_up(3);
            return Ok(());
        }

        if let Some((_col, _row)) = event.mouse_scroll_down() {
            state.scroll_content_down(3);
            return Ok(());
        }

        // Handle mouse click events
        if let Event::Mouse(mouse) = &event {
            use crossterm::event::{MouseButton, MouseEventKind};
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && state.handle_click(mouse.column, mouse.row)
            {
                return Ok(());
            }
        }

        // Escape or F1/? closes the dialog
        if event.is_escape() || event.is_help() {
            // If in search input with text, first clear search, then close
            if state.focus == HelpFocus::SearchInput && !state.search_query.is_empty() {
                state.clear_search();
                return Ok(());
            }
            self.help_dialog_state = None;
            self.mode = AppMode::Normal;
            return Ok(());
        }

        // Tab cycles through focus areas
        if event.is_tab() {
            state.focus_next();
            return Ok(());
        }

        // Shift+Tab cycles backward
        if event.is_shift_tab() {
            state.focus_prev();
            return Ok(());
        }

        // Handle focus-specific keys
        match state.focus {
            HelpFocus::SearchInput => {
                // Handle search input
                if event.is_backspace() {
                    state.delete_char_backward();
                    return Ok(());
                }

                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(c) => {
                            state.insert_char(c);
                        }
                        KeyCode::Left => {
                            state.move_cursor_left();
                        }
                        KeyCode::Right => {
                            state.move_cursor_right();
                        }
                        KeyCode::Home => {
                            state.move_cursor_home();
                        }
                        KeyCode::End => {
                            state.move_cursor_end();
                        }
                        KeyCode::Delete => {
                            state.delete_char_forward();
                        }
                        KeyCode::Enter => {
                            // Jump to content area when Enter pressed in search
                            state.focus = HelpFocus::ContentArea;
                        }
                        _ => {}
                    }
                }
            }
            HelpFocus::CategoryList => {
                if event.is_up() {
                    state.prev_category();
                    return Ok(());
                }

                if event.is_down() {
                    state.next_category();
                    return Ok(());
                }

                if event.is_enter() {
                    // Select category and move to content
                    state.focus = HelpFocus::ContentArea;
                    return Ok(());
                }
            }
            HelpFocus::ContentArea => {
                if event.is_up() {
                    state.scroll_content_up(1);
                    return Ok(());
                }

                if event.is_down() {
                    state.scroll_content_down(1);
                    return Ok(());
                }

                if event.is_page_up() {
                    state.scroll_content_up(10);
                    return Ok(());
                }

                if event.is_page_down() {
                    state.scroll_content_down(10);
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Handle events in suspend dialog mode
    pub(in crate::tui::app) async fn handle_suspend_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use super::super::suspend_dialog::{SuspendAction, SuspendFocus};

        // Allow switching to exit dialog with Ctrl+C
        if event.is_quit() {
            self.suspend_dialog_state = None;
            use super::super::exit_dialog::ExitDialogState;
            // Use last known preserve_chat setting
            self.exit_dialog_state = Some(ExitDialogState::with_preserve_chat(
                self.last_preserve_chat_setting,
            ));
            self.mode = AppMode::ExitDialog;
            return Ok(());
        }

        let Some(state) = &mut self.suspend_dialog_state else {
            // No state, return to normal mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Handle mouse click events
        if let Event::Mouse(mouse) = &event {
            use crossterm::event::{MouseButton, MouseEventKind};
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(clicked) = state.handle_click(mouse.column, mouse.row)
            {
                state.set_focus(clicked);
                // If checkbox clicked, toggle it; otherwise activate the button
                if clicked == SuspendFocus::ExitWhenDoneCheckbox {
                    state.toggle_exit_when_done();
                } else if let Some(action) = state.selected_action() {
                    return self.execute_suspend_action(action);
                }
            }
        }

        // Escape closes dialog without action
        if event.is_escape() {
            self.suspend_dialog_state = None;
            self.mode = AppMode::Normal;
            return Ok(());
        }

        // Tab/Shift+Tab to navigate between buttons
        if event.is_tab() {
            state.focus_next();
            return Ok(());
        }

        if event.is_shift_tab() {
            state.focus_prev();
            return Ok(());
        }

        // Left/Right arrow keys also navigate
        if let Event::Key(key) = &event {
            match key.code {
                KeyCode::Left => {
                    state.focus_prev();
                    return Ok(());
                }
                KeyCode::Right => {
                    state.focus_next();
                    return Ok(());
                }
                // 'b' or 'B' for quick Background
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    return self.execute_suspend_action(SuspendAction::Background);
                }
                // 's' or 'S' for quick Suspend
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    return self.execute_suspend_action(SuspendAction::Suspend);
                }
                // Space toggles checkbox if focused on it
                KeyCode::Char(' ') => {
                    if state.focus == SuspendFocus::ExitWhenDoneCheckbox {
                        state.toggle_exit_when_done();
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        // Enter activates selected button (not checkbox - checkbox uses Space)
        if event.is_enter()
            && let Some(action) = state.selected_action()
        {
            return self.execute_suspend_action(action);
        }
        // If checkbox is focused, Enter does nothing (use Space to toggle)

        Ok(())
    }

    /// Execute the selected suspend/background action
    fn execute_suspend_action(
        &mut self,
        action: super::super::suspend_dialog::SuspendAction,
    ) -> Result<()> {
        use super::super::suspend_dialog::SuspendAction;

        // Save exit_when_done setting before cleaning up dialog
        let exit_when_done = self
            .suspend_dialog_state
            .as_ref()
            .map(|s| s.exit_when_done())
            .unwrap_or(false);

        // Clean up dialog state
        self.suspend_dialog_state = None;

        match action {
            SuspendAction::Background => {
                // Signal the main loop to background the process
                self.pending_background = true;
                // Store the exit_when_done preference
                self.exit_when_agent_done = exit_when_done;
            }
            SuspendAction::Suspend => {
                // Signal the main loop to suspend the process
                self.pending_suspend = true;
            }
        }

        // Keep mode as SuspendDialog - the main loop will handle the actual
        // terminal restoration and signal sending, then reset mode
        Ok(())
    }

    /// Handle events in exit dialog mode
    pub(in crate::tui::app) async fn handle_exit_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use super::super::exit_dialog::{ExitAction, ExitFocus};

        // Allow switching to suspend dialog with Ctrl+Z
        if event.is_suspend() {
            // Save preserve_chat setting before switching
            let preserve_chat = self
                .exit_dialog_state
                .as_ref()
                .map(|s| s.preserve_chat())
                .unwrap_or(true);
            self.last_preserve_chat_setting = preserve_chat;

            self.exit_dialog_state = None;
            use super::super::suspend_dialog::SuspendDialogState;
            self.suspend_dialog_state = Some(SuspendDialogState::new());
            self.mode = AppMode::SuspendDialog;
            return Ok(());
        }

        let Some(state) = &mut self.exit_dialog_state else {
            // No state, return to normal mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Handle mouse click events
        if let Event::Mouse(mouse) = &event {
            use crossterm::event::{MouseButton, MouseEventKind};
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(clicked) = state.handle_click(mouse.column, mouse.row)
            {
                state.set_focus(clicked);
                // If checkbox clicked, toggle it; otherwise activate the button
                match clicked {
                    ExitFocus::PreserveChatCheckbox => {
                        state.toggle_preserve_chat();
                    }
                    ExitFocus::ExitWhenDoneCheckbox => {
                        state.toggle_exit_when_done();
                    }
                    _ => {
                        if let Some(action) = state.selected_action() {
                            return self.execute_exit_action(action);
                        }
                    }
                }
            }
        }

        // Escape closes dialog without action
        if event.is_escape() {
            self.exit_dialog_state = None;
            self.mode = AppMode::Normal;
            return Ok(());
        }

        // Tab/Shift+Tab to navigate between buttons
        if event.is_tab() {
            state.focus_next();
            return Ok(());
        }

        if event.is_shift_tab() {
            state.focus_prev();
            return Ok(());
        }

        // Left/Right arrow keys also navigate
        if let Event::Key(key) = &event {
            match key.code {
                KeyCode::Left => {
                    state.focus_prev();
                    return Ok(());
                }
                KeyCode::Right => {
                    state.focus_next();
                    return Ok(());
                }
                // 'e' or 'E' for quick Exit
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    return self.execute_exit_action(ExitAction::Exit);
                }
                // 'b' or 'B' for quick Background
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    return self.execute_exit_action(ExitAction::Background);
                }
                // Space toggles checkbox if focused on it
                KeyCode::Char(' ') => match state.focus {
                    ExitFocus::PreserveChatCheckbox => {
                        state.toggle_preserve_chat();
                        return Ok(());
                    }
                    ExitFocus::ExitWhenDoneCheckbox => {
                        state.toggle_exit_when_done();
                        return Ok(());
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // Enter activates selected button (not checkbox - checkbox uses Space)
        if event.is_enter()
            && let Some(action) = state.selected_action()
        {
            return self.execute_exit_action(action);
        }
        // If checkbox is focused, Enter does nothing (use Space to toggle)

        Ok(())
    }

    /// Execute the selected exit/background action
    fn execute_exit_action(&mut self, action: super::super::exit_dialog::ExitAction) -> Result<()> {
        use super::super::exit_dialog::ExitAction;

        // Save settings before cleaning up dialog
        let exit_when_done = self
            .exit_dialog_state
            .as_ref()
            .map(|s| s.exit_when_done())
            .unwrap_or(false);
        let preserve_chat = self
            .exit_dialog_state
            .as_ref()
            .map(|s| s.preserve_chat())
            .unwrap_or(true);

        // Save preserve_chat for next dialog open
        self.last_preserve_chat_setting = preserve_chat;
        self.preserve_chat_on_exit = preserve_chat;

        // Clean up dialog state
        self.exit_dialog_state = None;

        match action {
            ExitAction::Exit => {
                // Signal the main loop to exit
                self.should_quit = true;
                self.mode = AppMode::Normal;
            }
            ExitAction::Background => {
                // Signal the main loop to background the process
                self.pending_background = true;
                // Store the exit_when_done preference
                self.exit_when_agent_done = exit_when_done;
                // Keep mode - the main loop will handle the actual
                // terminal restoration and then reset mode
            }
        }

        Ok(())
    }

    /// Handle events in hotkey configuration dialog mode
    pub(in crate::tui::app) async fn handle_hotkey_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use crate::tui::hotkey_content::BrainwiresHotkeyProvider;
        use ratatui_interact::components::hotkey_dialog::{
            HotkeyDialogAction, handle_hotkey_dialog_key, handle_hotkey_dialog_mouse,
        };

        let Some(state) = &mut self.hotkey_dialog_state else {
            // No state, return to normal mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Handle mouse events
        if let Event::Mouse(mouse) = event {
            let action = handle_hotkey_dialog_mouse(state, mouse);
            // Mouse actions (scroll, click) are handled internally by the state
            // We don't need to do anything special here
            let _ = action;
            return Ok(());
        }

        // Handle keyboard events
        if let Event::Key(key) = event {
            let action = handle_hotkey_dialog_key(state, key);

            match action {
                HotkeyDialogAction::Close => {
                    self.hotkey_dialog_state = None;
                    self.mode = AppMode::Normal;
                }
                HotkeyDialogAction::EntrySelected { .. } => {
                    // Show toast with selected hotkey info
                    let provider = BrainwiresHotkeyProvider;
                    if let Some(entry) = state.get_selected_entry(&provider) {
                        let msg = format!(
                            "{}: {} [{}]",
                            entry.key_combination, entry.action, entry.context
                        );
                        self.show_toast(msg, 3000);
                    }
                }
                HotkeyDialogAction::None
                | HotkeyDialogAction::ScrollUp(_)
                | HotkeyDialogAction::ScrollDown(_) => {
                    // No additional action needed
                }
            }
        }

        Ok(())
    }

    /// Handle events in sudo password dialog mode
    pub(in crate::tui::app) async fn handle_sudo_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        let Some(ref mut state) = self.sudo_dialog_state else {
            // No state, return to previous mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Escape cancels the request
        if event.is_escape() {
            state.cancel();
            self.sudo_dialog_state = None;
            self.mode = AppMode::Normal;
            self.add_console_message("🔒 Sudo password prompt cancelled".to_string());
            return Ok(());
        }

        // Handle key events
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => {
                    if state.password_len() > 0 {
                        state.submit();
                        self.sudo_dialog_state = None;
                        self.mode = AppMode::Normal;
                        self.add_console_message("🔒 Sudo password submitted".to_string());
                    }
                }
                KeyCode::Char(c) => {
                    state.insert_char(c);
                }
                KeyCode::Backspace => {
                    state.delete_char();
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle events in approval dialog mode
    pub(in crate::tui::app) async fn handle_approval_dialog_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use crate::approval::ApprovalResponse;

        let Some(ref mut state) = self.approval_dialog_state else {
            // No state, return to previous mode
            self.mode = AppMode::Normal;
            return Ok(());
        };

        // Escape denies the request
        if event.is_escape() {
            state.respond(ApprovalResponse::Deny);
            self.approval_dialog_state = None;
            self.mode = AppMode::Normal;
            self.add_console_message("❌ Tool approval denied".to_string());
            return Ok(());
        }

        // Handle approval keys
        if let Event::Key(key) = event {
            match key.code {
                // 'y' or 'Y' - Approve once
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let tool_name = state
                        .current_request
                        .as_ref()
                        .map(|r| r.tool_name.clone())
                        .unwrap_or_default();
                    state.respond(ApprovalResponse::Approve);
                    self.approval_dialog_state = None;
                    self.mode = AppMode::Normal;
                    self.add_console_message(format!("✅ Tool '{}' approved", tool_name));
                }
                // 'n' or 'N' - Deny once
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    let tool_name = state
                        .current_request
                        .as_ref()
                        .map(|r| r.tool_name.clone())
                        .unwrap_or_default();
                    state.respond(ApprovalResponse::Deny);
                    self.approval_dialog_state = None;
                    self.mode = AppMode::Normal;
                    self.add_console_message(format!("❌ Tool '{}' denied", tool_name));
                }
                // 'a' or 'A' - Always approve (for session)
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    let tool_name = state
                        .current_request
                        .as_ref()
                        .map(|r| r.tool_name.clone())
                        .unwrap_or_default();
                    state.respond(ApprovalResponse::ApproveForSession);
                    self.approval_dialog_state = None;
                    self.mode = AppMode::Normal;
                    self.add_console_message(format!(
                        "✅ Tool '{}' approved for session",
                        tool_name
                    ));
                }
                // 'd' or 'D' - Always deny (for session)
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    let tool_name = state
                        .current_request
                        .as_ref()
                        .map(|r| r.tool_name.clone())
                        .unwrap_or_default();
                    state.respond(ApprovalResponse::DenyForSession);
                    self.approval_dialog_state = None;
                    self.mode = AppMode::Normal;
                    self.add_console_message(format!("❌ Tool '{}' denied for session", tool_name));
                }
                _ => {}
            }
        }

        Ok(())
    }
}
