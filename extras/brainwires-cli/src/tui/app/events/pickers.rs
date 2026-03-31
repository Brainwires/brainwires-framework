//! Picker Event Handlers
//!
//! Event handling for session picker, tool picker, and file explorer modes.

use crate::tui::Event;
use crate::tui::app::state::{App, AppMode};
use anyhow::Result;
use crossterm::event::KeyCode;

impl App {
    /// Handle events in reverse search mode
    pub(in crate::tui::app) async fn handle_search_event(&mut self, event: Event) -> Result<()> {
        use super::super::history::HistoryOps;

        if event.is_escape() {
            // Exit search mode
            self.mode = AppMode::Normal;
            self.search_query.clear();
            self.search_results.clear();
            return Ok(());
        }

        if event.is_enter() {
            // Select current search result
            if let Some(matched) = self.get_current_search_result() {
                self.input_state.set_text(matched);
            }
            self.mode = AppMode::Normal;
            self.search_query.clear();
            self.search_results.clear();
            return Ok(());
        }

        if event.is_up() {
            // Navigate to next search result (older)
            if !self.search_results.is_empty()
                && self.search_result_index < self.search_results.len() - 1
            {
                self.search_result_index += 1;
            }
            return Ok(());
        }

        if event.is_down() {
            // Navigate to previous search result (newer)
            if self.search_result_index > 0 {
                self.search_result_index -= 1;
            }
            return Ok(());
        }

        if event.is_backspace() {
            // Remove from search query
            self.search_query.pop();
            self.update_search_results();
            return Ok(());
        }

        if let Some(c) = event.char() {
            // Add to search query
            self.search_query.push(c);
            self.update_search_results();
        }

        Ok(())
    }

    /// Handle events in session picker mode
    pub(in crate::tui::app) async fn handle_picker_event(&mut self, event: Event) -> Result<()> {
        use super::super::session_management::SessionManagement;

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    // Exit picker mode
                    self.mode = AppMode::Normal;
                    self.status = "Session picker cancelled".to_string();
                }
                KeyCode::Up => {
                    // Move selection up
                    if self.selected_session_index > 0 {
                        self.selected_session_index -= 1;
                        // Update scroll position to keep selection visible
                        // Each session takes 2 lines (title, timestamp)
                        let selected_line = (self.selected_session_index * 2) as u16 + 2; // +2 for header
                        if selected_line < self.session_picker_scroll {
                            self.session_picker_scroll = selected_line;
                        }
                    }
                }
                KeyCode::Down => {
                    // Move selection down
                    if self.selected_session_index < self.available_sessions.len().saturating_sub(1)
                    {
                        self.selected_session_index += 1;
                        // Update scroll position to keep selection visible
                        // Each session takes 2 lines (title, timestamp)
                        let selected_line = (self.selected_session_index * 2) as u16 + 2; // +2 for header
                        let visible_lines = 20; // Conservative estimate
                        if selected_line + 1 >= self.session_picker_scroll + visible_lines {
                            self.session_picker_scroll =
                                selected_line.saturating_sub(visible_lines - 3);
                        }
                    }
                }
                KeyCode::Enter => {
                    // Load selected session
                    let conversation_id = self
                        .available_sessions
                        .get(self.selected_session_index)
                        .map(|s| s.conversation_id.clone());

                    if let Some(id) = conversation_id {
                        self.load_session(&id).await?;
                    }
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Handle events in tool picker mode
    pub(in crate::tui::app) async fn handle_tool_picker_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        if event.is_escape() {
            // Cancel picker
            self.tool_picker_state = None;
            self.mode = AppMode::Normal;
            self.status = "Tool picker cancelled".to_string();
            return Ok(());
        }

        if event.is_enter() {
            // Confirm selection
            self.confirm_tool_selection();
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => {
                    self.tool_picker_move_up();
                }
                KeyCode::Down => {
                    self.tool_picker_move_down();
                }
                KeyCode::Char(' ') => {
                    self.tool_picker_toggle();
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.tool_picker_select_all();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.tool_picker_select_none();
                }
                KeyCode::Left => {
                    // Collapse current category
                    if let Some(state) = &mut self.tool_picker_state {
                        state.collapsed.insert(state.selected_category);
                        state.selected_tool = None;
                    }
                }
                KeyCode::Right => {
                    // Expand current category
                    if let Some(state) = &mut self.tool_picker_state {
                        state.collapsed.remove(&state.selected_category);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Move up in tool picker
    fn tool_picker_move_up(&mut self) {
        let Some(state) = &mut self.tool_picker_state else {
            return;
        };

        if let Some(tool_idx) = state.selected_tool {
            // Move up within tools
            if tool_idx > 0 {
                state.selected_tool = Some(tool_idx - 1);
            } else {
                // Move to category header
                state.selected_tool = None;
            }
        } else {
            // Move to previous category's last tool (or header if collapsed)
            if state.selected_category > 0 {
                state.selected_category -= 1;
                if state.collapsed.contains(&state.selected_category) {
                    state.selected_tool = None;
                } else {
                    let tools_count = state
                        .categories
                        .get(state.selected_category)
                        .map(|(_, tools)| tools.len())
                        .unwrap_or(0);
                    if tools_count > 0 {
                        state.selected_tool = Some(tools_count - 1);
                    }
                }
            }
        }
    }

    /// Move down in tool picker
    fn tool_picker_move_down(&mut self) {
        let Some(state) = &mut self.tool_picker_state else {
            return;
        };

        let current_cat_tools = state
            .categories
            .get(state.selected_category)
            .map(|(_, tools)| tools.len())
            .unwrap_or(0);
        let is_collapsed = state.collapsed.contains(&state.selected_category);

        if let Some(tool_idx) = state.selected_tool {
            // Move down within tools
            if tool_idx + 1 < current_cat_tools {
                state.selected_tool = Some(tool_idx + 1);
            } else {
                // Move to next category
                if state.selected_category + 1 < state.categories.len() {
                    state.selected_category += 1;
                    state.selected_tool = None;
                }
            }
        } else {
            // On category header
            if !is_collapsed && current_cat_tools > 0 {
                // Move to first tool
                state.selected_tool = Some(0);
            } else {
                // Move to next category
                if state.selected_category + 1 < state.categories.len() {
                    state.selected_category += 1;
                    state.selected_tool = None;
                }
            }
        }
    }

    /// Toggle selection in tool picker
    fn tool_picker_toggle(&mut self) {
        let Some(state) = &mut self.tool_picker_state else {
            return;
        };

        if let Some(tool_idx) = state.selected_tool {
            // Toggle specific tool
            if let Some((_, tools)) = state.categories.get_mut(state.selected_category)
                && let Some((_, _, selected)) = tools.get_mut(tool_idx)
            {
                *selected = !*selected;
            }
        } else {
            // Toggle entire category
            if let Some((_, tools)) = state.categories.get_mut(state.selected_category) {
                let all_selected = tools.iter().all(|(_, _, s)| *s);
                for (_, _, selected) in tools.iter_mut() {
                    *selected = !all_selected;
                }
            }
        }
    }

    /// Select all tools in picker
    fn tool_picker_select_all(&mut self) {
        let Some(state) = &mut self.tool_picker_state else {
            return;
        };

        for (_, tools) in state.categories.iter_mut() {
            for (_, _, selected) in tools.iter_mut() {
                *selected = true;
            }
        }
    }

    /// Select no tools in picker
    fn tool_picker_select_none(&mut self) {
        let Some(state) = &mut self.tool_picker_state else {
            return;
        };

        for (_, tools) in state.categories.iter_mut() {
            for (_, _, selected) in tools.iter_mut() {
                *selected = false;
            }
        }
    }

    /// Handle events in file explorer mode
    pub(in crate::tui::app) async fn handle_file_explorer_event(
        &mut self,
        event: Event,
    ) -> Result<()> {
        use super::super::file_explorer::{EntryType, FileExplorerMode};
        use super::super::nano_editor::NanoEditorState;

        let Some(state) = &mut self.file_explorer_state else {
            return Ok(());
        };

        // Handle search mode separately
        if state.mode == FileExplorerMode::Search {
            if event.is_escape() {
                state.exit_search();
                return Ok(());
            }

            if event.is_enter() {
                // Select current filtered item and exit search
                state.exit_search();
                return Ok(());
            }

            if event.is_backspace() {
                let mut query = state.search_query.clone();
                query.pop();
                state.update_search(&query);
                return Ok(());
            }

            if let Some(c) = event.char() {
                let query = format!("{}{}", state.search_query, c);
                state.update_search(&query);
                return Ok(());
            }

            return Ok(());
        }

        // Browser mode controls
        if event.is_escape() {
            // Exit file explorer
            self.file_explorer_state = None;
            self.mode = AppMode::ConversationFullscreen;
            return Ok(());
        }

        if event.is_enter() {
            if let Some(entry) = state.current_entry().cloned() {
                match &entry.entry_type {
                    EntryType::Directory | EntryType::ParentDir => {
                        let _ = state.enter_directory(&entry.path);
                    }
                    EntryType::File { .. } => {
                        // Open file in editor
                        match NanoEditorState::open(entry.path.clone()) {
                            Ok(editor) => {
                                self.nano_editor_state = Some(editor);
                                self.mode = AppMode::NanoEditor;
                            }
                            Err(e) => {
                                self.show_toast(format!("Failed to open: {}", e), 3000);
                            }
                        }
                    }
                    EntryType::Symlink { target } => {
                        if target.is_dir() {
                            let _ = state.enter_directory(target);
                        } else {
                            match NanoEditorState::open(target.clone()) {
                                Ok(editor) => {
                                    self.nano_editor_state = Some(editor);
                                    self.mode = AppMode::NanoEditor;
                                }
                                Err(e) => {
                                    self.show_toast(format!("Failed to open: {}", e), 3000);
                                }
                            }
                        }
                    }
                }
            }
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => state.cursor_up(),
                KeyCode::Down => state.cursor_down(),
                KeyCode::PageUp => state.page_up(20),
                KeyCode::PageDown => state.page_down(20),
                KeyCode::Left | KeyCode::Backspace => {
                    let _ = state.go_up();
                }
                KeyCode::Right => {
                    // Enter directory if on one
                    if let Some(entry) = state.current_entry().cloned()
                        && matches!(
                            entry.entry_type,
                            EntryType::Directory | EntryType::ParentDir
                        )
                    {
                        let _ = state.enter_directory(&entry.path);
                    }
                }
                KeyCode::Char(' ') => state.toggle_selection(),
                KeyCode::Char('/') => state.start_search(),
                KeyCode::Char('e') => {
                    // Edit current file
                    if let Some(entry) = state.current_entry().cloned()
                        && matches!(entry.entry_type, EntryType::File { .. })
                    {
                        match NanoEditorState::open(entry.path) {
                            Ok(editor) => {
                                self.nano_editor_state = Some(editor);
                                self.mode = AppMode::NanoEditor;
                            }
                            Err(e) => {
                                self.show_toast(format!("Failed to open: {}", e), 3000);
                            }
                        }
                    }
                }
                KeyCode::Char('a') => state.select_all_files(),
                KeyCode::Char('n') => state.clear_selection(),
                KeyCode::Char('.') => state.toggle_hidden(),
                KeyCode::Char('r') => {
                    let _ = state.refresh();
                }
                KeyCode::Char('i') => {
                    // Insert selected files into working set
                    let paths = state.get_selected_paths();
                    let mut added = 0;
                    for path in &paths {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            let tokens = content.len() / 4; // Rough estimate
                            self.working_set.add(path.clone(), tokens);
                            added += 1;
                        }
                    }
                    // Need to re-borrow state mutably for clear_selection
                    if added > 0 {
                        self.show_toast(format!("Added {} file(s) to context", added), 2000);
                        if let Some(state) = &mut self.file_explorer_state {
                            state.clear_selection();
                        }
                    } else {
                        self.show_toast(
                            "No files selected (use Space to select)".to_string(),
                            2000,
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
