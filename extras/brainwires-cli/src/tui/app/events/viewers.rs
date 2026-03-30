//! Viewer Event Handlers
//!
//! Event handling for console view, shell viewer, and fullscreen modes.

use crate::tui::app::state::{App, AppMode};
use crate::tui::Event;
use anyhow::Result;
use crossterm::event::KeyCode;

impl App {
    /// Handle events in console view mode
    pub(in crate::tui::app) async fn handle_console_event(&mut self, event: Event) -> Result<()> {
        // Handle mouse scroll events for console scrolling
        if let Some((_col, _row)) = event.mouse_scroll_up() {
            self.console_state.scroll_up(3);
            return Ok(());
        }

        if let Some((_col, _row)) = event.mouse_scroll_down() {
            // Use a reasonable visible height estimate for scroll bounds
            self.console_state.scroll_down(3, 20);
            return Ok(());
        }

        if event.is_escape() {
            // Exit console view (re-enable mouse capture if it was disabled)
            self.mouse_capture_disabled = false;
            self.mode = AppMode::Normal;
            return Ok(());
        }

        if event.is_copy() {
            // Copy console contents to clipboard
            self.copy_console_to_clipboard();
            return Ok(());
        }

        if event.is_toggle_mouse() {
            // Toggle mouse capture for text selection
            self.mouse_capture_disabled = !self.mouse_capture_disabled;
            if self.mouse_capture_disabled {
                self.show_toast("Mouse disabled - select text with terminal".to_string(), 2000);
            } else {
                self.show_toast("Mouse enabled".to_string(), 1500);
            }
            return Ok(());
        }

        if event.is_page_up() {
            // Scroll console up
            self.console_state.scroll_up(10);
            return Ok(());
        }

        if event.is_page_down() {
            // Scroll console down
            self.console_state.scroll_down(10, 20);
            return Ok(());
        }

        Ok(())
    }

    /// Copy console contents to clipboard
    fn copy_console_to_clipboard(&mut self) {
        use ratatui_interact::utils::{copy_to_clipboard, ClipboardResult};

        let content = self.console_state.content_as_string();
        match copy_to_clipboard(&content) {
            ClipboardResult::Success => {
                self.show_toast(
                    format!("Copied {} lines to clipboard", self.console_state.line_count()),
                    2000,
                );
            }
            ClipboardResult::Error(e) => {
                self.show_toast(format!("Failed to copy: {}", e), 3000);
            }
            ClipboardResult::NotAvailable => {
                self.show_toast("Clipboard not available".to_string(), 3000);
            }
        }
    }

    /// Handle events in shell viewer mode
    pub(in crate::tui::app) async fn handle_shell_viewer_event(&mut self, event: Event) -> Result<()> {
        // Handle mouse scroll events for shell output scrolling
        if let Some((_col, _row)) = event.mouse_scroll_up() {
            self.shell_viewer_scroll = self.shell_viewer_scroll.saturating_sub(3);
            return Ok(());
        }

        if let Some((_col, _row)) = event.mouse_scroll_down() {
            self.shell_viewer_scroll = self.shell_viewer_scroll.saturating_add(3);
            return Ok(());
        }

        if event.is_escape() {
            // Exit shell viewer
            self.mode = AppMode::Normal;
            return Ok(());
        }

        if event.is_up() {
            // Select previous shell command
            self.selected_shell_index = self.selected_shell_index.saturating_sub(1);
            self.shell_viewer_scroll = 0; // Reset scroll when selecting new command
            return Ok(());
        }

        if event.is_down() {
            // Select next shell command
            if !self.shell_history.is_empty() {
                self.selected_shell_index = (self.selected_shell_index + 1)
                    .min(self.shell_history.len().saturating_sub(1));
                self.shell_viewer_scroll = 0; // Reset scroll when selecting new command
            }
            return Ok(());
        }

        if event.is_page_up() {
            // Scroll output up
            self.shell_viewer_scroll = self.shell_viewer_scroll.saturating_sub(10);
            return Ok(());
        }

        if event.is_page_down() {
            // Scroll output down
            self.shell_viewer_scroll = self.shell_viewer_scroll.saturating_add(10);
            return Ok(());
        }

        Ok(())
    }

    /// Handle events in full-screen conversation view mode
    pub(in crate::tui::app) async fn handle_conversation_fullscreen_event(&mut self, event: Event) -> Result<()> {
        // Handle mouse scroll events for conversation scrolling
        // In fullscreen mode, scroll works anywhere since the whole screen is conversation
        if let Some((_col, _row)) = event.mouse_scroll_up() {
            self.scroll_up(3);
            return Ok(());
        }

        if let Some((_col, _row)) = event.mouse_scroll_down() {
            self.scroll_down(3);
            return Ok(());
        }

        // Handle double-click to exit fullscreen mode
        if let Some((col, row)) = event.mouse_click() {
            if self.check_double_click(col, row) {
                self.mouse_capture_disabled = false;
                self.mode = AppMode::Normal;
            }
            return Ok(());
        }

        // Open file explorer with Ctrl+Alt+F
        if event.is_file_explorer() {
            use super::super::file_explorer::FileExplorerState;
            let start_dir = std::env::current_dir().unwrap_or_default();
            self.file_explorer_state = Some(FileExplorerState::new(start_dir));
            self.mode = AppMode::FileExplorer;
            return Ok(());
        }

        // Open Find dialog with Ctrl+F
        if event.is_find() {
            use super::super::find_replace::{FindReplaceState, FindReplaceContext};
            self.find_replace_state = Some(FindReplaceState::new_find(
                FindReplaceContext::ConversationView
            ));
            // Build search text from all messages
            let search_text: String = self.messages.iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            if let Some(ref mut state) = self.find_replace_state {
                state.update_matches(&search_text);
            }
            self.mode = AppMode::FindDialog;
            return Ok(());
        }

        // Open Git SCM with Ctrl+G
        if event.is_git_scm() {
            use super::super::git_scm::GitScmState;
            match GitScmState::new() {
                Ok(state) => {
                    self.git_scm_state = Some(state);
                    self.mode = AppMode::GitScm;
                }
                Err(e) => {
                    self.show_toast(format!("Git not available: {}", e), 3000);
                }
            }
            return Ok(());
        }

        if event.is_escape() || event.is_fullscreen_toggle() {
            // Exit full-screen conversation view (re-enable mouse capture if it was disabled)
            self.mouse_capture_disabled = false;
            self.mode = AppMode::Normal;
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

        if event.is_copy() {
            // Copy conversation contents to clipboard
            self.copy_conversation_to_clipboard();
            return Ok(());
        }

        if event.is_toggle_mouse() {
            // Toggle mouse capture for text selection
            self.mouse_capture_disabled = !self.mouse_capture_disabled;
            if self.mouse_capture_disabled {
                self.show_toast("Mouse disabled - select text with terminal".to_string(), 2000);
            } else {
                self.show_toast("Mouse enabled".to_string(), 1500);
            }
            return Ok(());
        }

        if event.is_up() {
            // Scroll conversation up
            self.scroll_up(1);
            return Ok(());
        }

        if event.is_down() {
            // Scroll conversation down
            self.scroll_down(1);
            return Ok(());
        }

        if event.is_page_up() {
            // Scroll conversation up
            self.scroll_up(10);
            return Ok(());
        }

        if event.is_page_down() {
            // Scroll conversation down
            self.scroll_down(10);
            return Ok(());
        }

        Ok(())
    }

    /// Handle events in full-screen input view mode
    pub(in crate::tui::app) async fn handle_input_fullscreen_event(&mut self, event: Event) -> Result<()> {
        // Handle paste events (bracketed paste mode)
        if let Event::Paste(text) = event {
            self.handle_paste(&text);
            return Ok(());
        }

        // Handle double-click to exit fullscreen mode
        if let Some((col, row)) = event.mouse_click() {
            if self.check_double_click(col, row) {
                self.mode = AppMode::Normal;
            }
            return Ok(());
        }

        // Open Find dialog with Ctrl+F
        if event.is_find() {
            use super::super::find_replace::{FindReplaceState, FindReplaceContext};
            self.find_replace_state = Some(FindReplaceState::new_find(
                FindReplaceContext::InputView
            ));
            let input_text = self.input_text();
            if let Some(ref mut state) = self.find_replace_state {
                state.update_matches(&input_text);
            }
            self.mode = AppMode::FindDialog;
            return Ok(());
        }

        // Open Find and Replace dialog with Ctrl+H
        if event.is_replace() {
            use super::super::find_replace::{FindReplaceState, FindReplaceContext};
            self.find_replace_state = Some(FindReplaceState::new_replace(
                FindReplaceContext::InputView
            ));
            let input_text = self.input_text();
            if let Some(ref mut state) = self.find_replace_state {
                state.update_matches(&input_text);
            }
            self.mode = AppMode::FindReplaceDialog;
            return Ok(());
        }

        // Escape or Alt+F exits fullscreen mode
        if event.is_escape() || event.is_fullscreen_toggle() {
            self.mode = AppMode::Normal;
            return Ok(());
        }

        // Shift+Enter or Alt+Enter: Insert newline
        if event.is_shift_enter() {
            self.input_state.insert_newline();
            return Ok(());
        }

        // Enter: Submit message (exit fullscreen and submit)
        if event.is_enter() {
            use super::super::message_processing::MessageProcessing;
            if !self.input_text().trim().is_empty() {
                self.mode = AppMode::Normal;
                self.submit_message().await?;
            }
            return Ok(());
        }

        // Up/Down arrow keys for cursor movement in text
        if event.is_up() {
            if self.is_multiline_input() && !self.cursor_on_first_line() {
                self.input_state.move_up();
            }
            return Ok(());
        }

        if event.is_down() {
            if self.is_multiline_input() && !self.cursor_on_last_line() {
                self.input_state.move_down();
            }
            return Ok(());
        }

        // Page Up/Down for larger cursor movements
        if event.is_page_up() {
            self.input_state.move_page_up();
            return Ok(());
        }

        if event.is_page_down() {
            self.input_state.move_page_down();
            return Ok(());
        }

        // Advanced text editing shortcuts
        if event.is_delete_word_backward() {
            self.input_state.delete_word_backward();
            return Ok(());
        }

        if event.is_delete_word_forward() {
            self.input_state.delete_word_forward();
            return Ok(());
        }

        if event.is_delete_to_start() {
            self.input_state.delete_to_line_start();
            return Ok(());
        }

        if event.is_delete_to_end() {
            self.input_state.delete_to_line_end();
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
            self.input_state.delete_char_backward();
            return Ok(());
        }

        // Character input and basic navigation
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
                    self.input_state.move_line_start();
                }
                KeyCode::End => {
                    self.input_state.move_line_end();
                }
                KeyCode::Delete => {
                    self.input_state.delete_char_forward();
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Copy conversation contents to clipboard
    fn copy_conversation_to_clipboard(&mut self) {
        use ratatui_interact::utils::{copy_to_clipboard, ClipboardResult};

        let content: String = self.messages
            .iter()
            .map(|msg| format!("[{}] {}", msg.role.to_uppercase(), msg.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        match copy_to_clipboard(&content) {
            ClipboardResult::Success => {
                self.show_toast(
                    format!("Copied {} messages to clipboard", self.messages.len()),
                    2000,
                );
            }
            ClipboardResult::Error(e) => {
                self.show_toast(format!("Failed to copy: {}", e), 3000);
            }
            ClipboardResult::NotAvailable => {
                self.show_toast("Clipboard not available".to_string(), 3000);
            }
        }
    }
}
