//! Plan Mode Operations for TUI
//!
//! Handles entering, exiting, and operating in plan mode which provides
//! an isolated planning context separate from the main conversation.

use anyhow::Result;

use super::autocomplete::AutocompleteOps;
use super::state::{App, AppMode, LogLevel, TuiMessage};
use crate::tui::Event;
use crate::types::plan_mode::{PlanModeState, SavedMainContext};
use brainwires::agent_network::ipc::{AgentMessage, DisplayMessage, ViewerMessage};

impl App {
    /// Enter plan mode with optional focus/goal
    pub async fn enter_plan_mode(&mut self, focus: Option<String>) -> Result<()> {
        // Don't enter plan mode if already in it
        if self.mode == AppMode::PlanMode {
            self.add_console_message("Already in plan mode".to_string());
            return Ok(());
        }

        // Convert TuiMessages to DisplayMessages for saving
        let display_messages: Vec<DisplayMessage> = self
            .messages
            .iter()
            .map(|m| DisplayMessage::new(&m.role, &m.content, m.created_at))
            .collect();

        // Save current main context
        self.plan_mode_saved_main = Some(SavedMainContext::new(
            display_messages,
            self.conversation_history.clone(),
            self.scroll,
            self.status.clone(),
        ));

        // Save current prompt mode and switch to Plan
        self.pre_plan_prompt_mode = Some(self.prompt_mode.clone());
        self.prompt_mode = super::state::PromptMode::Plan;

        // Create or restore plan mode state
        let plan_state = if let Some(existing) = self.plan_mode_state.take() {
            // Restore existing plan mode state if it exists
            let mut state = existing;
            state.activate();
            state
        } else {
            // Create new plan mode state
            PlanModeState::new(self.session_id.clone(), focus.clone())
        };

        // Update focus if provided
        let mut plan_state = plan_state;
        if focus.is_some() {
            plan_state.focus = focus.clone();
        }

        // Set plan mode state
        self.plan_mode_state = Some(plan_state.clone());

        // Switch UI to plan mode
        self.mode = AppMode::PlanMode;

        // Restore plan mode messages to TUI
        self.messages = self
            .plan_mode_state
            .as_ref()
            .map(|s| {
                s.messages
                    .iter()
                    .map(|m| TuiMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                        created_at: m.created_at,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Restore plan mode conversation history
        self.conversation_history = self
            .plan_mode_state
            .as_ref()
            .map(|s| s.conversation_history.clone())
            .unwrap_or_default();

        // Reset scroll
        self.scroll = 0;

        // Update status
        let focus_str = focus.as_deref().unwrap_or("general planning");
        self.set_status(
            LogLevel::Info,
            format!("Plan Mode - Focus: {} (Ctrl+P to exit)", focus_str),
        );

        // Add console message
        self.add_console_message(format!("Entered plan mode: {}", focus_str));

        // If in IPC mode, notify the agent
        if self.is_ipc_mode
            && let Some(ref mut writer) = self.ipc_writer
        {
            let _ = writer
                .write(&ViewerMessage::EnterPlanMode {
                    focus: self.plan_mode_state.as_ref().and_then(|s| s.focus.clone()),
                })
                .await;
        }

        Ok(())
    }

    /// Exit plan mode and return to main context
    pub async fn exit_plan_mode(&mut self) -> Result<()> {
        // Don't exit if not in plan mode
        if self.mode != AppMode::PlanMode {
            return Ok(());
        }

        // Save current plan mode state before exiting
        if let Some(ref mut state) = self.plan_mode_state {
            // Update plan mode messages from current TUI state
            state.messages = self
                .messages
                .iter()
                .map(|m| DisplayMessage::new(&m.role, &m.content, m.created_at))
                .collect();

            // Update conversation history
            state.conversation_history = self.conversation_history.clone();

            // Deactivate but keep for potential resume
            state.deactivate();
        }

        // Restore main context
        if let Some(saved) = self.plan_mode_saved_main.take() {
            // Restore messages
            self.messages = saved
                .messages
                .iter()
                .map(|m| TuiMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    created_at: m.created_at,
                })
                .collect();

            // Restore conversation history
            self.conversation_history = saved.conversation_history;

            // Restore scroll position
            self.scroll = saved.scroll;

            // Restore status
            self.set_status(LogLevel::Info, saved.status);
        } else {
            // No saved context, just reset
            self.set_status(LogLevel::Info, format!("Ready - Model: {}", self.model));
        }

        // Exit plan mode
        self.mode = AppMode::Normal;

        // Restore prompt mode from before entering plan mode
        self.prompt_mode = self
            .pre_plan_prompt_mode
            .take()
            .unwrap_or(super::state::PromptMode::Edit);

        // Add console message
        self.add_console_message("Exited plan mode".to_string());

        // If in IPC mode, notify the agent
        if self.is_ipc_mode
            && let Some(ref mut writer) = self.ipc_writer
        {
            let _ = writer.write(&ViewerMessage::ExitPlanMode).await;
        }

        Ok(())
    }

    /// Handle events in plan mode
    ///
    /// Plan mode uses the same input handling as normal mode,
    /// but with Escape exiting plan mode and Enter submitting to plan context.
    pub async fn handle_plan_mode_event(&mut self, event: Event) -> Result<()> {
        use crossterm::event::KeyCode;

        // Handle paste events
        if let Event::Paste(text) = event {
            self.handle_paste(&text);
            return Ok(());
        }

        // Handle escape to exit plan mode
        if event.is_escape() {
            self.exit_plan_mode().await?;
            return Ok(());
        }

        // Handle enter to submit
        if event.is_enter() {
            // If autocomplete is showing, accept suggestion and submit
            if self.show_autocomplete {
                self.autocomplete_accept(false);
                if !self.input_text().trim().is_empty() {
                    self.submit_plan_mode_input().await?;
                }
            } else if !self.input_text().trim().is_empty() {
                self.submit_plan_mode_input().await?;
            }
            return Ok(());
        }

        // Handle tab for autocomplete or focus toggle
        if event.is_tab() && !self.show_autocomplete {
            // Cycle focus
            match self.focused_panel {
                super::state::FocusedPanel::Input => {
                    self.focused_panel = super::state::FocusedPanel::Conversation;
                }
                super::state::FocusedPanel::Conversation => {
                    self.focused_panel = super::state::FocusedPanel::Input;
                }
                super::state::FocusedPanel::StatusBar => {
                    self.focused_panel = super::state::FocusedPanel::Input;
                }
            }
            return Ok(());
        }

        // Handle autocomplete navigation
        if event.is_up() && self.show_autocomplete {
            self.autocomplete_prev();
            return Ok(());
        }
        if event.is_down() && self.show_autocomplete {
            self.autocomplete_next();
            return Ok(());
        }

        // Handle scrolling in conversation panel
        if event.is_up() && self.focused_panel == super::state::FocusedPanel::Conversation {
            self.scroll_up(1);
            return Ok(());
        }
        if event.is_down() && self.focused_panel == super::state::FocusedPanel::Conversation {
            self.scroll_down(1);
            return Ok(());
        }
        if event.is_page_up() {
            self.scroll_up(10);
            return Ok(());
        }
        if event.is_page_down() {
            self.scroll_down(10);
            return Ok(());
        }

        // Handle backspace
        if event.is_backspace() {
            if self.input_state.delete_char_backward() {
                self.update_autocomplete();
            }
            return Ok(());
        }

        // Handle text input and navigation
        if let Event::Key(key) = event {
            match key.code {
                // Text editing
                KeyCode::Char(c) => {
                    // Filter out control characters
                    if !c.is_control() && c != '\x1b' {
                        self.input_state.insert_char(c);
                        self.update_autocomplete();
                    }
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
                KeyCode::Tab => {
                    // Accept autocomplete with space
                    self.autocomplete_accept(true);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Submit input while in plan mode
    async fn submit_plan_mode_input(&mut self) -> Result<()> {
        let input = self.input_text();
        self.clear_input();

        // Add user message to plan mode
        let timestamp = chrono::Utc::now().timestamp_millis();
        self.messages.push(TuiMessage {
            role: "user".to_string(),
            content: input.clone(),
            created_at: timestamp,
        });

        // Update plan mode state
        if let Some(ref mut state) = self.plan_mode_state {
            state.add_message(DisplayMessage::new("user", &input, timestamp));
        }

        // If in IPC mode, send to agent
        if self.is_ipc_mode
            && let Some(ref mut writer) = self.ipc_writer
        {
            let _ = writer
                .write(&ViewerMessage::PlanModeUserInput {
                    content: input,
                    context_files: Vec::new(),
                })
                .await;
        }

        // Scroll to bottom
        self.pending_scroll_to_bottom = true;

        Ok(())
    }

    /// Handle plan mode IPC messages
    pub fn handle_plan_mode_ipc(&mut self, message: &AgentMessage) {
        match message {
            AgentMessage::PlanModeEntered {
                plan_session_id,
                messages,
                status,
            } => {
                // Update plan mode state from agent
                if let Some(ref mut state) = self.plan_mode_state {
                    state.plan_session_id = plan_session_id.clone();
                    state.messages = messages.clone();
                }

                // Update TUI messages
                self.messages = messages
                    .iter()
                    .map(|m| TuiMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                        created_at: m.created_at,
                    })
                    .collect();

                self.set_status(LogLevel::Info, status.clone());
                self.mode = AppMode::PlanMode;
            }

            AgentMessage::PlanModeExited { summary } => {
                // Exit plan mode
                if let Some(summary) = summary {
                    self.add_console_message(format!("Plan mode summary: {}", summary));
                }

                // Restore main context if we have saved state
                if let Some(saved) = self.plan_mode_saved_main.take() {
                    self.messages = saved
                        .messages
                        .iter()
                        .map(|m| TuiMessage {
                            role: m.role.clone(),
                            content: m.content.clone(),
                            created_at: m.created_at,
                        })
                        .collect();
                    self.conversation_history = saved.conversation_history;
                    self.scroll = saved.scroll;
                    self.set_status(LogLevel::Info, saved.status);
                }

                self.mode = AppMode::Normal;
            }

            AgentMessage::PlanModeSync {
                plan_session_id,
                main_session_id: _,
                messages,
                status,
                is_busy: _,
            } => {
                // Sync plan mode state from agent
                if let Some(ref mut state) = self.plan_mode_state {
                    state.plan_session_id = plan_session_id.clone();
                    state.messages = messages.clone();
                }

                // Update TUI messages
                self.messages = messages
                    .iter()
                    .map(|m| TuiMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                        created_at: m.created_at,
                    })
                    .collect();

                self.set_status(LogLevel::Info, status.clone());
            }

            AgentMessage::PlanModeMessageAdded { message } => {
                // Add message to plan mode
                self.messages.push(TuiMessage {
                    role: message.role.clone(),
                    content: message.content.clone(),
                    created_at: message.created_at,
                });

                if let Some(ref mut state) = self.plan_mode_state {
                    state.add_message(message.clone());
                }

                self.pending_scroll_to_bottom = true;
            }

            AgentMessage::PlanModeStreamChunk { text } => {
                // Handle streaming in plan mode - append to last assistant message
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.content.push_str(text);
                    }
                } else {
                    // Create new assistant message
                    let timestamp = chrono::Utc::now().timestamp_millis();
                    self.messages.push(TuiMessage {
                        role: "assistant".to_string(),
                        content: text.clone(),
                        created_at: timestamp,
                    });
                }
            }

            AgentMessage::PlanModeStreamEnd { .. } => {
                // Stream ended - update plan mode state
                if let Some(ref mut state) = self.plan_mode_state {
                    // Sync messages to state
                    state.messages = self
                        .messages
                        .iter()
                        .map(|m| DisplayMessage::new(&m.role, &m.content, m.created_at))
                        .collect();
                }
            }

            _ => {}
        }
    }

    /// Clear plan mode history
    pub fn clear_plan_mode(&mut self) {
        if let Some(ref mut state) = self.plan_mode_state {
            state.clear();
        }

        if self.mode == AppMode::PlanMode {
            self.messages.clear();
            self.conversation_history.clear();
            self.scroll = 0;
        }

        self.add_console_message("Plan mode history cleared".to_string());
    }

    /// Get plan mode status
    pub fn plan_mode_status(&self) -> String {
        if let Some(ref state) = self.plan_mode_state {
            let focus = state.focus.as_deref().unwrap_or("general");
            let msg_count = state.message_count();
            let active = if state.active { "active" } else { "inactive" };

            format!(
                "Plan Mode: {}\nSession: {}\nFocus: {}\nMessages: {}",
                active, state.plan_session_id, focus, msg_count
            )
        } else {
            "Plan mode not initialized".to_string()
        }
    }
}
