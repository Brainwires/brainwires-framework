//! Modal Event Handlers
//!
//! Event handling for task viewer, nano editor, git SCM, and question modes.

use crate::tui::app::state::{App, AppMode};
use crate::tui::Event;
use anyhow::Result;
use crossterm::event::KeyCode;

impl App {
    /// Handle events in task viewer mode
    pub(in crate::tui::app) async fn handle_task_viewer_event(&mut self, event: Event) -> Result<()> {
        if event.is_escape() {
            // Close task viewer
            self.mode = AppMode::Normal;
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => {
                    self.task_viewer_move_up();
                }
                KeyCode::Down => {
                    self.task_viewer_move_down();
                }
                KeyCode::Enter | KeyCode::Left | KeyCode::Right => {
                    // Toggle expand/collapse
                    self.task_viewer_toggle_collapse();
                }
                KeyCode::Char(' ') => {
                    // Toggle task status (pending -> in_progress -> completed)
                    self.task_viewer_toggle_status().await;
                }
                KeyCode::PageUp => {
                    // Scroll up
                    self.task_viewer_state.scroll = self.task_viewer_state.scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    // Scroll down
                    self.task_viewer_state.scroll = self.task_viewer_state.scroll.saturating_add(10);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Move up in task viewer
    fn task_viewer_move_up(&mut self) {
        if self.task_viewer_state.selected_index > 0 {
            self.task_viewer_state.selected_index -= 1;
        }
        // Adjust scroll if needed
        if (self.task_viewer_state.selected_index as u16) < self.task_viewer_state.scroll {
            self.task_viewer_state.scroll = self.task_viewer_state.selected_index as u16;
        }
    }

    /// Move down in task viewer
    fn task_viewer_move_down(&mut self) {
        let max_index = if self.task_viewer_state.visible_tasks.is_empty() {
            // Fall back to cache line count
            self.task_tree_cache.lines().count().saturating_sub(1)
        } else {
            self.task_viewer_state.visible_tasks.len().saturating_sub(1)
        };

        if self.task_viewer_state.selected_index < max_index {
            self.task_viewer_state.selected_index += 1;
        }
    }

    /// Toggle expand/collapse for current task
    fn task_viewer_toggle_collapse(&mut self) {
        if let Some((task_id, _, _)) = self.task_viewer_state.visible_tasks
            .get(self.task_viewer_state.selected_index)
            .cloned()
        {
            if self.task_viewer_state.collapsed.contains(&task_id) {
                self.task_viewer_state.collapsed.remove(&task_id);
            } else {
                self.task_viewer_state.collapsed.insert(task_id);
            }
        }
    }

    /// Toggle task status (cycles: pending -> in_progress -> completed -> pending)
    async fn task_viewer_toggle_status(&mut self) {
        use crate::types::agent::TaskStatus;

        // Get task ID from visible_tasks or cache
        let task_id = if let Some((id, _, _)) = self.task_viewer_state.visible_tasks
            .get(self.task_viewer_state.selected_index)
        {
            Some(id.clone())
        } else {
            None
        };

        if let Some(task_id) = task_id {
            let mut manager = self.task_manager.write().await;
            if let Some(task) = manager.get_task(&task_id).await {
                let new_status = match task.status {
                    TaskStatus::Pending => TaskStatus::InProgress,
                    TaskStatus::InProgress => TaskStatus::Completed,
                    TaskStatus::Completed => TaskStatus::Pending,
                    TaskStatus::Failed => TaskStatus::Pending,
                    TaskStatus::Blocked => TaskStatus::Pending,
                    TaskStatus::Skipped => TaskStatus::Pending,
                };

                // Update task status
                match new_status {
                    TaskStatus::InProgress => {
                        let _ = manager.start_task(&task_id).await;
                    }
                    TaskStatus::Completed => {
                        let _ = manager.complete_task(&task_id, "Manually completed".to_string()).await;
                    }
                    _ => {
                        // Reset to pending - would need a reset method
                    }
                }
            }
            drop(manager);

            // Refresh the task tree cache
            self.refresh_task_tree_cache().await;
        }
    }

    /// Refresh the task tree cache
    pub(in crate::tui::app) async fn refresh_task_tree_cache(&mut self) {
        let manager = self.task_manager.read().await;
        self.task_tree_cache = manager.format_tree().await;
        self.task_count_cache = manager.count().await;
    }

    /// Handle events in nano editor mode
    pub(in crate::tui::app) async fn handle_nano_editor_event(&mut self, event: Event) -> Result<()> {
        use super::super::nano_editor::CursorDirection;

        let Some(state) = &mut self.nano_editor_state else {
            return Ok(());
        };

        // Exit on Ctrl+X
        if event.is_ctrl_x() {
            if state.is_modified() {
                // Show warning but exit anyway (simplified behavior)
                self.show_toast("Discarded unsaved changes".to_string(), 2000);
            }
            self.nano_editor_state = None;
            self.mode = AppMode::FileExplorer;
            return Ok(());
        }

        // Save on Ctrl+S or Ctrl+O
        if event.is_save() || event.is_ctrl_o() {
            match state.save() {
                Ok(()) => {
                    self.show_toast("File saved".to_string(), 2000);
                }
                Err(e) => {
                    self.show_toast(format!("Error saving: {}", e), 3000);
                }
            }
            return Ok(());
        }

        // Cut line on Ctrl+K
        if event.is_delete_to_end() {
            state.cut_line();
            return Ok(());
        }

        // Paste on Ctrl+U
        if event.is_delete_to_start() {
            state.paste();
            return Ok(());
        }

        // Navigation and editing
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => state.move_cursor(CursorDirection::Up),
                KeyCode::Down => state.move_cursor(CursorDirection::Down),
                KeyCode::Left => state.move_cursor(CursorDirection::Left),
                KeyCode::Right => state.move_cursor(CursorDirection::Right),
                KeyCode::Home => state.move_to_line_boundary(true),
                KeyCode::End => state.move_to_line_boundary(false),
                KeyCode::PageUp => state.page_move(true, 20),
                KeyCode::PageDown => state.page_move(false, 20),
                KeyCode::Backspace => state.delete_backward(),
                KeyCode::Delete => state.delete_forward(),
                KeyCode::Enter => state.insert_newline(),
                KeyCode::Tab => state.insert_char('\t'),
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        state.insert_char(c);
                    }
                }
                KeyCode::Esc => {
                    // Alternative exit without save
                    if state.is_modified() {
                        self.show_toast("Unsaved changes (Ctrl+X to exit, Ctrl+S to save)".to_string(), 2000);
                    } else {
                        self.nano_editor_state = None;
                        self.mode = AppMode::FileExplorer;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle events in Git SCM mode
    pub(in crate::tui::app) async fn handle_git_scm_event(&mut self, event: Event) -> Result<()> {
        use super::super::git_scm::{GitOperationMode, ScmPanel, GitAction};

        let Some(state) = &mut self.git_scm_state else {
            return Ok(());
        };

        // Handle commit message mode separately
        if let GitOperationMode::CommitMessage = state.mode {
            if event.is_escape() {
                state.mode = GitOperationMode::Browse;
                state.commit_message.clear();
                return Ok(());
            }

            if event.is_enter() {
                // Execute commit
                if !state.commit_message.is_empty() {
                    match state.execute_action(GitAction::Commit).await {
                        Ok(()) => {
                            self.show_toast("Changes committed".to_string(), 2000);
                        }
                        Err(e) => {
                            self.show_toast(format!("Commit failed: {}", e), 3000);
                        }
                    }
                }
                return Ok(());
            }

            if event.is_backspace() {
                state.commit_message.pop();
                return Ok(());
            }

            if let Some(c) = event.char() {
                state.commit_message.push(c);
                return Ok(());
            }

            return Ok(());
        }

        // Handle confirm mode
        if let GitOperationMode::Confirm { action, .. } = &state.mode.clone() {
            let action_clone = action.clone();
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        match state.execute_action(action_clone).await {
                            Ok(()) => {
                                self.show_toast("Action completed".to_string(), 2000);
                            }
                            Err(e) => {
                                self.show_toast(format!("Action failed: {}", e), 3000);
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        state.mode = GitOperationMode::Browse;
                        state.clear_messages();
                    }
                    _ => {}
                }
            }
            return Ok(());
        }

        // Browse mode
        if event.is_escape() {
            // Exit Git SCM
            self.git_scm_state = None;
            self.mode = AppMode::ConversationFullscreen;
            return Ok(());
        }

        if event.is_tab() {
            // Switch panel
            state.next_panel();
            return Ok(());
        }

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up => state.cursor_up(),
                KeyCode::Down => state.cursor_down(),
                KeyCode::PageUp => state.page_up(10),
                KeyCode::PageDown => state.page_down(10),
                KeyCode::Char(' ') => state.toggle_selection(),
                KeyCode::Enter | KeyCode::Char('s') => {
                    // Stage selected/current file
                    let files_to_stage: Vec<_> = match state.current_panel {
                        ScmPanel::Changes => state.changed_files
                            .iter()
                            .filter(|f| f.selected || state.cursor_index == state.changed_files.iter().position(|x| x.path == f.path).unwrap_or(usize::MAX))
                            .map(|f| f.path.clone())
                            .collect(),
                        ScmPanel::Untracked => state.untracked_files
                            .iter()
                            .filter(|f| f.selected || state.cursor_index == state.untracked_files.iter().position(|x| x.path == f.path).unwrap_or(usize::MAX))
                            .map(|f| f.path.clone())
                            .collect(),
                        _ => vec![],
                    };

                    if !files_to_stage.is_empty() {
                        match state.stage_files(&files_to_stage).await {
                            Ok(()) => {
                                self.show_toast(format!("Staged {} file(s)", files_to_stage.len()), 1500);
                            }
                            Err(e) => {
                                self.show_toast(format!("Stage failed: {}", e), 3000);
                            }
                        }
                    }
                }
                KeyCode::Char('u') => {
                    // Unstage selected/current file
                    if state.current_panel == ScmPanel::Staged {
                        let files_to_unstage: Vec<_> = state.staged_files
                            .iter()
                            .filter(|f| f.selected || state.cursor_index == state.staged_files.iter().position(|x| x.path == f.path).unwrap_or(usize::MAX))
                            .map(|f| f.path.clone())
                            .collect();

                        if !files_to_unstage.is_empty() {
                            match state.unstage_files(&files_to_unstage).await {
                                Ok(()) => {
                                    self.show_toast(format!("Unstaged {} file(s)", files_to_unstage.len()), 1500);
                                }
                                Err(e) => {
                                    self.show_toast(format!("Unstage failed: {}", e), 3000);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('d') => {
                    // Discard changes (with confirmation)
                    if state.current_panel == ScmPanel::Changes {
                        let files: Vec<_> = state.changed_files
                            .iter()
                            .filter(|f| f.selected || state.cursor_index == state.changed_files.iter().position(|x| x.path == f.path).unwrap_or(usize::MAX))
                            .map(|f| f.path.clone())
                            .collect();

                        if !files.is_empty() {
                            state.mode = GitOperationMode::Confirm {
                                message: format!("Discard changes to {} file(s)? (y/n)", files.len()),
                                action: GitAction::Discard(files),
                            };
                        }
                    }
                }
                KeyCode::Char('c') => {
                    // Start commit (enter commit message mode)
                    if !state.staged_files.is_empty() {
                        state.mode = GitOperationMode::CommitMessage;
                        state.commit_message.clear();
                    } else {
                        self.show_toast("No files staged for commit".to_string(), 2000);
                    }
                }
                KeyCode::Char('P') => {
                    // Push to remote
                    match state.execute_action(GitAction::Push).await {
                        Ok(()) => {
                            self.show_toast("Pushed to remote".to_string(), 2000);
                        }
                        Err(e) => {
                            self.show_toast(format!("Push failed: {}", e), 3000);
                        }
                    }
                }
                KeyCode::Char('p') => {
                    // Pull from remote
                    match state.execute_action(GitAction::Pull).await {
                        Ok(()) => {
                            self.show_toast("Pulled from remote".to_string(), 2000);
                        }
                        Err(e) => {
                            self.show_toast(format!("Pull failed: {}", e), 3000);
                        }
                    }
                }
                KeyCode::Char('f') => {
                    // Fetch from remote
                    match state.execute_action(GitAction::Fetch).await {
                        Ok(()) => {
                            self.show_toast("Fetched from remote".to_string(), 2000);
                        }
                        Err(e) => {
                            self.show_toast(format!("Fetch failed: {}", e), 3000);
                        }
                    }
                }
                KeyCode::Char('r') => {
                    // Refresh status
                    if let Err(e) = state.refresh() {
                        self.show_toast(format!("Refresh failed: {}", e), 3000);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle events in question answer mode
    pub(in crate::tui::app) async fn handle_question_event(&mut self, event: Event) -> Result<()> {
        use crate::tui::question_parser;
        use crate::types::message::{Message, MessageContent, Role};

        // Get a reference to pending questions (if any)
        let questions = match &self.pending_questions {
            Some(q) => q.clone(),
            None => {
                // No questions, return to normal mode
                self.mode = AppMode::Normal;
                return Ok(());
            }
        };

        // Handle Escape - skip all questions
        if event.is_escape() {
            let decline_msg = question_parser::format_declined_message();

            // Add decline message to conversation
            self.messages.push(super::super::state::TuiMessage {
                role: "user".to_string(),
                content: decline_msg.clone(),
                created_at: chrono::Utc::now().timestamp(),
            });

            self.conversation_history.push(Message {
                role: Role::User,
                content: MessageContent::Text(decline_msg),
                name: None,
                metadata: None,
            });

            // Clear questions and return to normal mode
            self.pending_questions = None;
            self.question_state = crate::types::question::QuestionAnswerState::default();
            self.mode = AppMode::Normal;

            // Continue conversation with the AI
            self.call_ai_provider().await?;

            return Ok(());
        }

        // Handle Enter - submit answers (on last question) or toggle + advance
        if event.is_enter() {
            if self.question_state.is_last_question(&questions) {
                // Submit all answers
                let answers_msg = question_parser::format_answers_natural(&questions, &self.question_state);

                // Add user answer message to conversation
                self.messages.push(super::super::state::TuiMessage {
                    role: "user".to_string(),
                    content: answers_msg.clone(),
                    created_at: chrono::Utc::now().timestamp(),
                });

                self.conversation_history.push(Message {
                    role: Role::User,
                    content: MessageContent::Text(answers_msg),
                    name: None,
                    metadata: None,
                });

                // Clear questions and return to normal mode
                self.pending_questions = None;
                self.question_state = crate::types::question::QuestionAnswerState::default();
                self.mode = AppMode::Normal;

                // Continue conversation with the AI
                self.call_ai_provider().await?;
            } else {
                // Toggle current selection and move to next question
                self.question_state.toggle_current(&questions);
                self.question_state.next_question(&questions);
            }
            return Ok(());
        }

        // Handle Tab - move to next question (or toggle "Other" editing)
        if event.is_tab() {
            if self.question_state.editing_other {
                // Exit "Other" editing mode and move to next question
                self.question_state.editing_other = false;
                if !self.question_state.next_question(&questions) {
                    // Already on last question, do nothing special
                }
            } else if self.question_state.is_cursor_on_other(&questions) {
                // Enter "Other" editing mode
                self.question_state.editing_other = true;
                // Also select "Other"
                self.question_state.toggle_current(&questions);
            } else {
                // Move to next question
                self.question_state.next_question(&questions);
            }
            return Ok(());
        }

        // Handle Shift+Tab - move to previous question
        if event.is_shift_tab() {
            self.question_state.editing_other = false;
            self.question_state.prev_question();
            return Ok(());
        }

        // Handle keyboard input
        if let Event::Key(key) = event {
            // If editing "Other" text, handle text input
            if self.question_state.editing_other {
                match key.code {
                    KeyCode::Char(c) => {
                        self.question_state.append_other_char(c);
                    }
                    KeyCode::Backspace => {
                        self.question_state.backspace_other();
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Normal navigation
            match key.code {
                KeyCode::Up => {
                    self.question_state.cursor_up();
                }
                KeyCode::Down => {
                    self.question_state.cursor_down(&questions);
                }
                KeyCode::Char(' ') => {
                    // Toggle selection
                    self.question_state.toggle_current(&questions);

                    // If we selected "Other", enter editing mode
                    if self.question_state.is_cursor_on_other(&questions) {
                        let q_idx = self.question_state.current_question_idx;
                        if self.question_state.other_selected.get(q_idx).copied().unwrap_or(false) {
                            self.question_state.editing_other = true;
                        }
                    }
                }
                KeyCode::Char(c) => {
                    // If cursor is on "Other", start editing
                    if self.question_state.is_cursor_on_other(&questions) {
                        let q_idx = self.question_state.current_question_idx;
                        if let Some(selected) = self.question_state.other_selected.get_mut(q_idx) {
                            *selected = true;
                        }
                        self.question_state.editing_other = true;
                        self.question_state.append_other_char(c);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
