//! Agent Plan Mode Operations
//!
//! Handles entering, exiting, and operating in plan mode on the agent side.
//! This provides an isolated planning context separate from the main conversation.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;

use super::AgentState;
use brainwires::agent_network::ipc::{AgentMessage, DisplayMessage};
use crate::storage::PlanModeStore;
use crate::types::message::{Message, MessageContent, Role};
use crate::types::plan_mode::{PlanModeState, SavedMainContext};

/// Plan mode operations for AgentState
impl AgentState {
    /// Enter plan mode with optional focus/goal
    ///
    /// This saves the current main context and initializes (or restores)
    /// an isolated plan mode context.
    pub async fn enter_plan_mode(&mut self, focus: Option<String>) -> Result<AgentMessage> {
        // Don't enter plan mode if already in it
        if self.is_plan_mode {
            return Ok(AgentMessage::Error {
                message: "Already in plan mode".to_string(),
                fatal: false,
            });
        }

        // Save current main context
        self.saved_main_context = Some(SavedMainContext::new(
            self.messages.clone(),
            self.conversation_history.clone(),
            0, // Agent doesn't track scroll
            self.status.clone(),
        ));

        // Try to load existing plan mode state from storage
        let plan_store = PlanModeStore::new(Arc::new(
            crate::storage::LanceDatabase::new(
                crate::utils::paths::PlatformPaths::conversations_db_path()?
                    .to_str()
                    .context("Invalid DB path")?,
            )
            .await
            .context("Failed to connect to LanceDB")?,
        ));

        let plan_state = if let Ok(Some(existing)) = plan_store.get_by_main_session(&self.session_id).await {
            // Restore existing plan mode state
            let mut state = existing;
            state.activate();
            state.focus = focus.clone().or(state.focus);
            state
        } else {
            // Create new plan mode state
            PlanModeState::new(self.session_id.clone(), focus.clone())
        };

        // Set up plan mode context
        self.plan_mode_state = Some(plan_state.clone());
        self.is_plan_mode = true;

        // Switch to plan mode conversation (replace main context temporarily)
        self.messages = plan_state.messages.clone();
        self.conversation_history = self.build_plan_mode_conversation_history(&plan_state);

        // Update status
        let focus_str = focus.as_deref().unwrap_or("general planning");
        self.status = format!("Plan Mode - Focus: {}", focus_str);

        // Save plan mode state to storage
        if let Err(e) = plan_store.save(&plan_state).await {
            tracing::warn!("Failed to save plan mode state: {}", e);
        }

        Ok(AgentMessage::PlanModeEntered {
            plan_session_id: plan_state.plan_session_id,
            messages: plan_state.messages,
            status: self.status.clone(),
        })
    }

    /// Exit plan mode and return to main context
    ///
    /// This saves the plan mode state for potential resume and restores
    /// the main conversation context.
    pub async fn exit_plan_mode(&mut self) -> Result<AgentMessage> {
        // Don't exit if not in plan mode
        if !self.is_plan_mode {
            return Ok(AgentMessage::Error {
                message: "Not in plan mode".to_string(),
                fatal: false,
            });
        }

        // Save current plan mode state before exiting
        if let Some(ref mut state) = self.plan_mode_state {
            // Update plan mode messages from current state
            state.messages = self.messages.clone();
            // Update conversation history (without system prompt)
            state.conversation_history = self
                .conversation_history
                .iter()
                .filter(|m| m.role != Role::System)
                .cloned()
                .collect();
            state.deactivate();

            // Persist to storage
            let plan_store = PlanModeStore::new(Arc::new(
                crate::storage::LanceDatabase::new(
                    crate::utils::paths::PlatformPaths::conversations_db_path()?
                        .to_str()
                        .context("Invalid DB path")?,
                )
                .await
                .context("Failed to connect to LanceDB")?,
            ));

            if let Err(e) = plan_store.save(state).await {
                tracing::warn!("Failed to save plan mode state on exit: {}", e);
            }
        }

        // Generate summary if we have plan mode content
        let summary = self.generate_plan_summary();

        // Restore main context
        if let Some(saved) = self.saved_main_context.take() {
            self.messages = saved.messages;
            self.conversation_history = saved.conversation_history;
            self.status = saved.status;
        } else {
            // No saved context, just reset
            self.status = format!("Ready - Model: {}", self.model);
        }

        // Exit plan mode
        self.is_plan_mode = false;

        Ok(AgentMessage::PlanModeExited { summary })
    }

    /// Process user input while in plan mode
    ///
    /// This runs the AI with the plan mode context and returns a message
    /// that can be broadcast to viewers.
    pub async fn process_plan_mode_input(
        &mut self,
        content: String,
        update_tx: &broadcast::Sender<AgentMessage>,
    ) -> Result<()> {
        if !self.is_plan_mode {
            return Err(anyhow::anyhow!("Not in plan mode"));
        }

        // Add user message to plan mode context
        let timestamp = chrono::Utc::now().timestamp_millis();
        let user_display = DisplayMessage::new("user", &content, timestamp);

        self.messages.push(user_display.clone());
        self.conversation_history.push(Message::user(&content));

        // Update plan mode state
        if let Some(ref mut state) = self.plan_mode_state {
            state.add_message(user_display.clone());
            state.add_to_history(Message::user(&content));
        }

        // Notify viewers of the user message
        let _ = update_tx.send(AgentMessage::PlanModeMessageAdded {
            message: user_display,
        });

        // Set busy status
        self.is_busy = true;
        let _ = update_tx.send(AgentMessage::StatusUpdate {
            status: "Plan Mode - Thinking...".to_string(),
        });

        // Stream AI response in plan mode
        // Use read-only tools for plan mode (no file modifications)
        let tools = self.get_plan_mode_tools();
        let provider = self.provider.clone();
        let conversation_history = self.conversation_history.clone();
        let _model = self.model.clone();

        // Build ChatOptions with plan mode system prompt
        let system_prompt = self.build_plan_mode_system_prompt();
        let mut options = crate::types::provider::ChatOptions::default();
        options.system = Some(system_prompt);

        // Stream the response
        use crate::types::message::StreamChunk;
        use futures::StreamExt;

        let mut stream = provider.stream_chat(&conversation_history, Some(&tools), &options);
        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(StreamChunk::Text(text)) => {
                    full_response.push_str(&text);
                    let _ = update_tx.send(AgentMessage::PlanModeStreamChunk { text });
                }
                Ok(StreamChunk::Done) => {
                    let _ = update_tx.send(AgentMessage::PlanModeStreamEnd {
                        finish_reason: Some("stop".to_string()),
                    });
                    break;
                }
                Ok(_) => continue,
                Err(e) => {
                    tracing::error!("Plan mode stream error: {}", e);
                    let _ = update_tx.send(AgentMessage::Error {
                        message: e.to_string(),
                        fatal: false,
                    });
                    break;
                }
            }
        }

        // Add assistant response to plan mode context
        if !full_response.is_empty() {
            let assistant_display = DisplayMessage::new(
                "assistant",
                &full_response,
                chrono::Utc::now().timestamp_millis(),
            );

            self.messages.push(assistant_display.clone());
            self.conversation_history
                .push(Message::assistant(&full_response));

            // Update plan mode state
            if let Some(ref mut state) = self.plan_mode_state {
                state.add_message(assistant_display.clone());
                state.add_to_history(Message::assistant(&full_response));
            }

            // Notify viewers
            let _ = update_tx.send(AgentMessage::PlanModeMessageAdded {
                message: assistant_display,
            });
        }

        // Clear busy status
        self.is_busy = false;
        let focus_str = self
            .plan_mode_state
            .as_ref()
            .and_then(|s| s.focus.as_deref())
            .unwrap_or("general planning");
        let _ = update_tx.send(AgentMessage::StatusUpdate {
            status: format!("Plan Mode - Focus: {}", focus_str),
        });

        Ok(())
    }

    /// Build the system prompt for plan mode
    ///
    /// This creates a planning-focused system prompt that emphasizes
    /// research and planning without making changes.
    fn build_plan_mode_system_prompt(&self) -> String {
        let focus = self
            .plan_mode_state
            .as_ref()
            .and_then(|s| s.focus.as_deref())
            .unwrap_or("the task at hand");

        format!(
            r#"You are in PLAN MODE - an isolated planning context.

## Your Role
You are a planning assistant focused on: {}

## Guidelines
1. **Research & Explore**: Use read-only tools to understand the codebase and gather information.
2. **No Modifications**: Do NOT create, edit, or delete files. Only read and search.
3. **Think Through**: Consider multiple approaches and their trade-offs.
4. **Document Your Plan**: Create a clear, actionable plan with:
   - Summary of what needs to be done
   - Key files that will be affected
   - Step-by-step implementation approach
   - Potential risks or considerations

## Available Actions
- Read files to understand existing code
- Search for patterns and implementations
- Ask clarifying questions
- Propose implementation approaches

## Output Format
When you have a plan ready, format it clearly with headers and bullet points.
The plan should be concrete enough that it can be directly executed.

Remember: This is a PLANNING context. Your research and exploration here is isolated
from the main conversation. Only the final plan will be shared with the main context."#,
            focus
        )
    }

    /// Get read-only tools for plan mode
    ///
    /// Plan mode only has access to read-only tools to prevent
    /// accidental modifications during the planning phase.
    fn get_plan_mode_tools(&self) -> Vec<crate::types::tool::Tool> {
        // Filter to read-only tools
        let read_only_tools = [
            "read_file",
            "list_directory",
            "search_files",
            "grep",
            "find_files",
            "get_file_info",
            "tree",
            "cat",
            "head",
            "tail",
        ];

        self.tools
            .iter()
            .filter(|tool| {
                read_only_tools
                    .iter()
                    .any(|name| tool.name.to_lowercase().contains(name))
            })
            .cloned()
            .collect()
    }

    /// Build conversation history for plan mode
    ///
    /// This includes the plan mode system prompt and existing messages.
    fn build_plan_mode_conversation_history(&self, plan_state: &PlanModeState) -> Vec<Message> {
        let mut history = vec![Message {
            role: Role::System,
            content: MessageContent::Text(self.build_plan_mode_system_prompt()),
            name: None,
            metadata: None,
        }];

        // Add existing plan mode messages
        history.extend(plan_state.conversation_history.clone());

        history
    }

    /// Generate a summary of the plan mode session
    ///
    /// This extracts key points from the plan mode conversation.
    fn generate_plan_summary(&self) -> Option<String> {
        // Find the last assistant message that looks like a plan
        let last_assistant = self
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant" && m.content.len() > 100)?;

        // If it looks like a plan (has headers or lists), return a truncated version
        let content = &last_assistant.content;
        if content.contains('#') || content.contains("- ") || content.contains("1.") {
            // Return first 500 chars as summary
            let summary = if content.len() > 500 {
                format!("{}...", &content[..500])
            } else {
                content.clone()
            };
            Some(summary)
        } else {
            None
        }
    }

    /// Create a sync message for plan mode
    pub fn create_plan_mode_sync_message(&self) -> AgentMessage {
        if let Some(ref state) = self.plan_mode_state {
            AgentMessage::PlanModeSync {
                plan_session_id: state.plan_session_id.clone(),
                main_session_id: self.session_id.clone(),
                messages: self.messages.clone(),
                status: self.status.clone(),
                is_busy: self.is_busy,
            }
        } else {
            // Fallback to regular sync if not in plan mode
            self.create_sync_message()
        }
    }

    /// Clear plan mode history
    pub async fn clear_plan_mode(&mut self) -> Result<()> {
        if let Some(ref mut state) = self.plan_mode_state {
            state.clear();

            // Persist the cleared state
            let plan_store = PlanModeStore::new(Arc::new(
                crate::storage::LanceDatabase::new(
                    crate::utils::paths::PlatformPaths::conversations_db_path()?
                        .to_str()
                        .context("Invalid DB path")?,
                )
                .await
                .context("Failed to connect to LanceDB")?,
            ));

            if let Err(e) = plan_store.save(state).await {
                tracing::warn!("Failed to save cleared plan mode state: {}", e);
            }
        }

        if self.is_plan_mode {
            self.messages.clear();
            // Keep system prompt
            self.conversation_history.retain(|m| m.role == Role::System);
        }

        Ok(())
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
