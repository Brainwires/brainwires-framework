//! Core Event Handlers
//!
//! Main event dispatch and core input handling for normal, waiting, and cancel modes.

use crate::tui::Event;
use crate::tui::app::state::{App, AppMode, FocusedPanel, LogLevel};
use anyhow::Result;
use crossterm::event::KeyCode;

impl App {
    /// Handle an event, returns false if should exit
    pub async fn handle_event(&mut self, event: Event) -> Result<bool> {
        // Check for quit (Ctrl+C) - open exit dialog instead of immediately quitting
        // Only open dialog if not already in ExitDialog mode
        if event.is_quit() && self.mode != AppMode::ExitDialog {
            use super::super::exit_dialog::ExitDialogState;
            // Use last known preserve_chat setting (defaults to true)
            self.exit_dialog_state = Some(ExitDialogState::with_preserve_chat(
                self.last_preserve_chat_setting,
            ));
            self.mode = AppMode::ExitDialog;
            return Ok(true);
        }

        // Handle resize events - trigger scroll-to-bottom if flag is set (PTY mode)
        if let Event::Resize(_, _) = event {
            if self.scroll_to_bottom_on_resize {
                self.scroll_to_bottom_on_resize = false;
                self.pending_scroll_to_bottom = true;
            }
            return Ok(true);
        }

        // Handle IPC events from Session (event-driven, no polling)
        if let Event::Ipc(msg) = event {
            self.handle_ipc_event(msg);
            return Ok(true);
        }

        // Handle IPC disconnection - set flag for respawn
        if let Event::IpcDisconnected = event {
            self.add_console_message("⚠️ Session connection lost".to_string());
            self.set_status(LogLevel::Warn, "Session disconnected - respawning...");
            self.ipc_needs_respawn = true;
            return Ok(true);
        }

        // Check for suspend (Ctrl+Z) - globally available like quit
        // Only open dialog if not already in SuspendDialog mode
        if event.is_suspend() && self.mode != AppMode::SuspendDialog {
            use super::super::suspend_dialog::SuspendDialogState;
            self.suspend_dialog_state = Some(SuspendDialogState::new());
            self.mode = AppMode::SuspendDialog;
            return Ok(true);
        }

        // Check for console view (Ctrl+D) - globally available
        // Toggle: if already in ConsoleView, return to Normal; otherwise enter ConsoleView
        if event.is_console_view() {
            if self.mode == AppMode::ConsoleView {
                self.mode = AppMode::Normal;
            } else {
                self.mode = AppMode::ConsoleView;
                self.clear_unread_errors();
                // Scroll to bottom so the most recent journal entries are visible
                self.console_state.scroll_down(usize::MAX / 2, 100);
            }
            return Ok(true);
        }

        // Check for plan mode toggle (Ctrl+P) - globally available
        if event.is_plan_mode_toggle() {
            if self.mode == AppMode::PlanMode {
                self.exit_plan_mode().await?;
            } else {
                self.enter_plan_mode(None).await?;
            }
            return Ok(true);
        }

        match self.mode {
            AppMode::Normal => self.handle_normal_event(event).await?,
            AppMode::ReverseSearch => self.handle_search_event(event).await?,
            AppMode::SessionPicker => self.handle_picker_event(event).await?,
            AppMode::ConsoleView => self.handle_console_event(event).await?,
            AppMode::ShellViewer => self.handle_shell_viewer_event(event).await?,
            AppMode::Waiting => self.handle_waiting_event(event).await?,
            AppMode::ConversationFullscreen => {
                self.handle_conversation_fullscreen_event(event).await?
            }
            AppMode::InputFullscreen => self.handle_input_fullscreen_event(event).await?,
            AppMode::ToolPicker => self.handle_tool_picker_event(event).await?,
            AppMode::TaskViewer => self.handle_task_viewer_event(event).await?,
            AppMode::FileExplorer => self.handle_file_explorer_event(event).await?,
            AppMode::NanoEditor => self.handle_nano_editor_event(event).await?,
            AppMode::GitScm => self.handle_git_scm_event(event).await?,
            AppMode::CancelConfirm => self.handle_cancel_confirm_event(event).await?,
            AppMode::QuestionAnswer => self.handle_question_event(event).await?,
            AppMode::UserQuestion => self.handle_user_question_event(event).await?,
            AppMode::FindDialog | AppMode::FindReplaceDialog => {
                self.handle_find_replace_event(event).await?
            }
            AppMode::HelpDialog => self.handle_help_dialog_event(event).await?,
            AppMode::SuspendDialog => self.handle_suspend_dialog_event(event).await?,
            AppMode::ExitDialog => self.handle_exit_dialog_event(event).await?,
            AppMode::HotkeyDialog => self.handle_hotkey_dialog_event(event).await?,
            AppMode::ApprovalDialog => self.handle_approval_dialog_event(event).await?,
            AppMode::SudoPasswordDialog => self.handle_sudo_dialog_event(event).await?,
            AppMode::PlanMode => self.handle_plan_mode_event(event).await?,
            AppMode::SubAgentViewer => self.handle_sub_agent_viewer_event(event).await?,
        }

        Ok(!self.should_quit)
    }

    /// Handle events in normal mode
    pub(in crate::tui::app) async fn handle_normal_event(&mut self, event: Event) -> Result<()> {
        use super::super::autocomplete::AutocompleteOps;
        // Handle paste events (bracketed paste mode)
        if let Event::Paste(text) = event {
            self.handle_paste(&text);
            return Ok(());
        }

        // Handle mouse scroll events - only scroll if mouse is over the conversation panel
        if let Some((col, row)) = event.mouse_scroll_up() {
            if self.is_point_in_conversation_area(col, row) {
                self.scroll_up(3);
            }
            return Ok(());
        }

        if let Some((col, row)) = event.mouse_scroll_down() {
            if self.is_point_in_conversation_area(col, row) {
                self.scroll_down(3);
            }
            return Ok(());
        }

        // Handle mouse click on Exit button (single click opens exit dialog)
        // Handle mouse double-click to enter fullscreen (single-click focus switching disabled for panels)
        if let Some((col, row)) = event.mouse_click() {
            // Check for Exit button click first (single click)
            if self.is_point_in_exit_button(col, row) {
                use super::super::exit_dialog::ExitDialogState;
                self.exit_dialog_state = Some(ExitDialogState::with_preserve_chat(
                    self.last_preserve_chat_setting,
                ));
                self.mode = AppMode::ExitDialog;
                return Ok(());
            }

            // Double-click for fullscreen on panels
            let is_double_click = self.check_double_click(col, row);

            if is_double_click {
                if self.is_point_in_conversation_area(col, row) {
                    self.mode = AppMode::ConversationFullscreen;
                } else if self.is_point_in_input_area(col, row) {
                    self.mode = AppMode::InputFullscreen;
                }
            }
            return Ok(());
        }

        // Ctrl+B: open Sub-Agent Viewer (guard: not during waiting/streaming)
        if event.is_sub_agent_viewer() && self.mode != AppMode::Waiting {
            self.refresh_sub_agent_list().await;
            self.mode = AppMode::SubAgentViewer;
            return Ok(());
        }

        if event.is_reverse_search() {
            // Enter reverse search mode
            self.mode = AppMode::ReverseSearch;
            self.search_query.clear();
            return Ok(());
        }

        if event.is_session_picker() {
            // Enter session picker mode
            // Load available sessions from storage
            match self.conversation_store.list(Some(50)).await {
                Ok(sessions) => {
                    // list() already returns sorted by updated_at descending (newest first)
                    self.available_sessions = sessions;
                    self.selected_session_index = 0;
                    self.session_picker_scroll = 0;
                    self.mode = AppMode::SessionPicker;
                }
                Err(e) => {
                    self.set_status(LogLevel::Error, format!("Failed to load sessions: {}", e));
                }
            }
            return Ok(());
        }

        // Ctrl+D is now handled globally in handle_event()

        if event.is_task_viewer() {
            // Enter task viewer mode and refresh task tree
            self.refresh_task_tree_cache().await;
            self.task_viewer_state.selected_index = 0;
            self.task_viewer_state.scroll = 0;
            self.mode = AppMode::TaskViewer;
            return Ok(());
        }

        if event.is_help() {
            // Enter help dialog mode
            use super::super::help_dialog::HelpDialogState;
            self.help_dialog_state = Some(HelpDialogState::new());
            self.mode = AppMode::HelpDialog;
            return Ok(());
        }

        if event.is_tab() && !self.show_autocomplete {
            // Cycle focus forward (only when autocomplete not showing)
            // Skip StatusBar if it's hidden on small terminals
            match self.focused_panel {
                FocusedPanel::Input => {
                    self.focused_panel = FocusedPanel::Conversation;
                }
                FocusedPanel::Conversation => {
                    if self.status_bar_visible {
                        self.focused_panel = FocusedPanel::StatusBar;
                    } else {
                        self.focused_panel = FocusedPanel::Input;
                    }
                }
                FocusedPanel::StatusBar => {
                    self.focused_panel = FocusedPanel::Input;
                }
            }
            return Ok(());
        }

        if event.is_shift_tab() && !self.show_autocomplete {
            // Cycle focus backward (only when autocomplete not showing)
            // Skip StatusBar if it's hidden on small terminals
            match self.focused_panel {
                FocusedPanel::Input => {
                    if self.status_bar_visible {
                        self.focused_panel = FocusedPanel::StatusBar;
                    } else {
                        self.focused_panel = FocusedPanel::Conversation;
                    }
                }
                FocusedPanel::Conversation => {
                    self.focused_panel = FocusedPanel::Input;
                }
                FocusedPanel::StatusBar => {
                    self.focused_panel = FocusedPanel::Conversation;
                }
            }
            return Ok(());
        }

        // Handle Enter when StatusBar (Exit button) is focused
        if self.focused_panel == FocusedPanel::StatusBar
            && let Event::Key(key) = &event
            && (key.code == KeyCode::Enter || key.code == KeyCode::Char(' '))
        {
            // Open exit dialog
            use super::super::exit_dialog::ExitDialogState;
            self.exit_dialog_state = Some(ExitDialogState::with_preserve_chat(
                self.last_preserve_chat_setting,
            ));
            self.mode = AppMode::ExitDialog;
            return Ok(());
        }

        if event.is_fullscreen_toggle() {
            // Alt+F: Enter fullscreen mode for the currently focused panel
            match self.focused_panel {
                FocusedPanel::Input => {
                    self.mode = AppMode::InputFullscreen;
                }
                FocusedPanel::Conversation => {
                    self.mode = AppMode::ConversationFullscreen;
                }
                FocusedPanel::StatusBar => {
                    // No fullscreen for status bar, do nothing
                }
            }
            return Ok(());
        }

        if event.is_view_style_toggle() {
            // F9: Toggle conversation view style
            use super::super::ConversationViewStyle;
            self.conversation_view_style = match self.conversation_view_style {
                ConversationViewStyle::Journal => ConversationViewStyle::Classic,
                ConversationViewStyle::Classic => ConversationViewStyle::Journal,
            };
            let style_name = match self.conversation_view_style {
                ConversationViewStyle::Journal => "Journal",
                ConversationViewStyle::Classic => "Classic",
            };
            self.show_toast(format!("View: {}", style_name), 1500);
            return Ok(());
        }

        if event.is_shift_enter() {
            // Shift+Enter: Insert newline for multi-line input
            self.input_state.insert_newline();
            return Ok(());
        }

        // Journal tree navigation — active when Conversation is focused in Journal mode
        // Must be checked BEFORE the message-submission Enter handler
        use super::super::ConversationViewStyle;
        let in_journal_tree = self.focused_panel == FocusedPanel::Conversation
            && self.conversation_view_style == ConversationViewStyle::Journal;

        if in_journal_tree {
            if event.is_journal_cursor_down() {
                self.journal_tree.cursor_next();
                // Auto-scroll to keep cursor visible
                if let Some(idx) = self.journal_tree.cursor_render_index() {
                    let idx = idx as u16;
                    if idx < self.scroll {
                        self.scroll = idx;
                    } else if idx >= self.scroll + self.conversation_visible_height() {
                        self.scroll = idx
                            .saturating_sub(self.conversation_visible_height().saturating_sub(1));
                    }
                }
                return Ok(());
            }
            if event.is_journal_cursor_up() {
                self.journal_tree.cursor_prev();
                if let Some(idx) = self.journal_tree.cursor_render_index() {
                    let idx = idx as u16;
                    if idx < self.scroll {
                        self.scroll = idx;
                    }
                }
                return Ok(());
            }
            if event.is_journal_expand() {
                if let Some(cursor) = self.journal_tree.cursor {
                    self.journal_tree.expand(cursor);
                }
                return Ok(());
            }
            if event.is_journal_collapse() {
                // cursor_collapse_or_parent() does the right thing regardless of cursor state
                self.journal_tree.cursor_collapse_or_parent();
                return Ok(());
            }
            if event.is_enter() {
                if let Some(cursor) = self.journal_tree.cursor {
                    self.journal_tree.toggle_collapse(cursor);
                } else {
                    // No cursor yet — init at first item
                    self.journal_tree.cursor_first();
                }
                return Ok(());
            }
        }

        if event.is_enter() {
            use super::super::message_processing::MessageProcessing;
            // If autocomplete is showing, accept suggestion and submit immediately
            if self.show_autocomplete {
                self.autocomplete_accept(false); // Don't add space, complete the command
                // Now submit the completed command
                if !self.input_text().trim().is_empty() {
                    self.submit_message().await?;
                }
            } else if !self.input_text().trim().is_empty() {
                // Submit input
                self.submit_message().await?;
            }
            return Ok(());
        }

        if event.is_up() {
            use super::super::history::HistoryOps;
            // If autocomplete is showing, navigate autocomplete instead of history
            if self.show_autocomplete {
                self.autocomplete_prev();
            } else if self.focused_panel == FocusedPanel::Input {
                // In multiline input: move cursor up within text if not on first line
                // On first line (or single-line): navigate history
                if self.is_multiline_input() && !self.cursor_on_first_line() {
                    self.input_state.move_up();
                } else {
                    self.navigate_history_up();
                }
            } else {
                // Scroll conversation up when conversation is focused
                self.scroll_up(1);
            }
            return Ok(());
        }

        if event.is_down() {
            use super::super::history::HistoryOps;
            // If autocomplete is showing, navigate autocomplete instead of history
            if self.show_autocomplete {
                self.autocomplete_next();
            } else if self.focused_panel == FocusedPanel::Input {
                // In multiline input: move cursor down within text if not on last line
                // On last line (or single-line): navigate history
                if self.is_multiline_input() && !self.cursor_on_last_line() {
                    self.input_state.move_down();
                } else {
                    self.navigate_history_down();
                }
            } else {
                // Scroll conversation down when conversation is focused
                self.scroll_down(1);
            }
            return Ok(());
        }

        if event.is_page_up() {
            if self.focused_panel == FocusedPanel::Input && self.is_multiline_input() {
                // In multiline input: move cursor up by ~10 lines
                self.input_state.move_page_up();
            } else {
                // Scroll conversation up
                self.scroll_up(10);
            }
            return Ok(());
        }

        if event.is_page_down() {
            if self.focused_panel == FocusedPanel::Input && self.is_multiline_input() {
                // In multiline input: move cursor down by ~10 lines
                self.input_state.move_page_down();
            } else {
                // Scroll conversation down
                self.scroll_down(10);
            }
            return Ok(());
        }

        // Advanced text editing shortcuts
        if event.is_delete_word_backward() {
            self.input_state.delete_word_backward();
            self.update_autocomplete();
            return Ok(());
        }

        if event.is_delete_word_forward() {
            self.input_state.delete_word_forward();
            self.update_autocomplete();
            return Ok(());
        }

        if event.is_delete_to_start() {
            self.input_state.delete_to_line_start();
            self.update_autocomplete();
            return Ok(());
        }

        if event.is_delete_to_end() {
            self.input_state.delete_to_line_end();
            self.update_autocomplete();
            return Ok(());
        }

        if event.is_move_to_line_start() {
            self.input_state.move_line_start();
            return Ok(());
        }

        if event.is_move_to_line_end() {
            self.input_state.move_line_end();
            return Ok(());
        }

        // Ctrl+Home/End for document start/end (must check before plain Home/End)
        if event.is_move_to_document_start() {
            self.input_state.move_to_start();
            return Ok(());
        }

        if event.is_move_to_document_end() {
            self.input_state.move_to_end();
            return Ok(());
        }

        if event.is_move_word_backward() {
            self.input_state.move_word_left();
            return Ok(());
        }

        if event.is_move_word_forward() {
            self.input_state.move_word_right();
            return Ok(());
        }

        if event.is_backspace() {
            // Handle backspace
            if self.input_state.delete_char_backward() {
                self.update_autocomplete();
            }
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(c) => {
                    // Filter out control characters and escape sequence fragments
                    // These can appear when terminal sends resize/window reports in PTY mode
                    if c.is_control() || c == '\x1b' {
                        return Ok(());
                    }
                    // Insert character at cursor
                    self.input_state.insert_char(c);
                    self.update_autocomplete();
                }
                KeyCode::Left => {
                    // Move cursor left
                    self.input_state.move_left();
                }
                KeyCode::Right => {
                    // Move cursor right
                    self.input_state.move_right();
                }
                KeyCode::Home => {
                    // Move to start of current line (Mac default)
                    self.input_state.move_line_start();
                }
                KeyCode::End => {
                    // Move to end of current line (Mac default)
                    self.input_state.move_line_end();
                }
                KeyCode::Tab => {
                    // Accept autocomplete suggestion with space for adding arguments
                    self.autocomplete_accept(true);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle events while waiting for response
    pub(in crate::tui::app) async fn handle_waiting_event(&mut self, event: Event) -> Result<()> {
        // Handle paste events (bracketed paste mode)
        if let Event::Paste(text) = event {
            self.handle_paste(&text);
            return Ok(());
        }

        // Handle mouse scroll events - scroll conversation while waiting
        if let Some((col, row)) = event.mouse_scroll_up() {
            if self.is_point_in_conversation_area(col, row) {
                self.scroll_up(3);
            }
            return Ok(());
        }

        if let Some((col, row)) = event.mouse_scroll_down() {
            if self.is_point_in_conversation_area(col, row) {
                self.scroll_down(3);
            }
            return Ok(());
        }

        // Handle mouse click on Exit button (single click opens exit dialog)
        // Handle mouse double-click to enter fullscreen (single-click focus switching disabled for panels)
        if let Some((col, row)) = event.mouse_click() {
            // Check for Exit button click first (single click)
            if self.is_point_in_exit_button(col, row) {
                use super::super::exit_dialog::ExitDialogState;
                self.exit_dialog_state = Some(ExitDialogState::with_preserve_chat(
                    self.last_preserve_chat_setting,
                ));
                self.mode = AppMode::ExitDialog;
                return Ok(());
            }

            // Double-click for fullscreen on panels
            let is_double_click = self.check_double_click(col, row);

            if is_double_click {
                if self.is_point_in_conversation_area(col, row) {
                    self.mode = AppMode::ConversationFullscreen;
                } else if self.is_point_in_input_area(col, row) {
                    self.mode = AppMode::InputFullscreen;
                }
            }
            return Ok(());
        }

        // Cancel operation on Escape - show confirmation prompt first
        if event.is_escape() {
            self.mode = AppMode::CancelConfirm;
            self.set_status(LogLevel::Warn, "Cancel operation? (y/n)");
            return Ok(());
        }

        if event.is_enter() {
            // Queue the message to be sent after the agent finishes
            if !self.input_text().trim().is_empty() {
                self.queued_messages.push(self.input_text());
                self.set_status(
                    LogLevel::Info,
                    format!("Message queued ({} pending)", self.queued_messages.len()),
                );
                self.clear_input();
            }
            return Ok(());
        }

        if event.is_backspace() {
            self.input_state.delete_char_backward();
            return Ok(());
        }

        // Ctrl+Home/End for document start/end
        if event.is_move_to_document_start() {
            self.input_state.move_to_start();
            return Ok(());
        }

        if event.is_move_to_document_end() {
            self.input_state.move_to_end();
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char(c) => {
                    self.input_state.insert_char(c);
                }
                KeyCode::Left => {
                    self.input_state.move_left();
                }
                KeyCode::Right => {
                    self.input_state.move_right();
                }
                KeyCode::Home => {
                    // Move to start of current line (Mac default)
                    self.input_state.move_line_start();
                }
                KeyCode::End => {
                    // Move to end of current line (Mac default)
                    self.input_state.move_line_end();
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle events in cancel confirmation mode
    pub(in crate::tui::app) async fn handle_cancel_confirm_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    // User confirmed cancellation - perform the actual cancel
                    if let Some(token) = self.cancellation_token.take() {
                        token.cancel();
                    }

                    // Abort the streaming task if running
                    if let Some(handle) = self.stream_task_handle.take() {
                        handle.abort();
                    }

                    // Abort the tool execution task if running
                    if let Some(handle) = self.tool_task_handle.take() {
                        handle.abort();
                    }

                    // Clean up all streaming/tool state
                    self.stream_rx = None;
                    self.tool_rx = None;
                    self.tool_tx = None;
                    self.pending_tool_data = None;
                    self.streaming_content.clear();
                    self.streaming_msg_idx = None;
                    self.streaming_conversation = None;
                    self.streaming_user_content = None;

                    // Add cancellation message to console
                    self.add_console_message("Operation cancelled by user".to_string());
                    self.set_status(LogLevel::Info, "Operation cancelled");

                    // Return to normal mode
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    // User declined - return to waiting mode
                    self.mode = AppMode::Waiting;
                    self.set_status(LogLevel::Info, "Continuing...");
                }
                _ => {
                    // Ignore other keys, keep showing confirmation
                }
            }
        }

        Ok(())
    }
}
