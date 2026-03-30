//! Session Management
//!
//! Handles loading and managing conversation sessions.

use super::state::{App, TuiMessage};
use crate::types::message::{Message, MessageContent, Role};
use anyhow::Result;

pub trait SessionManagement {
    async fn load_session(&mut self, conversation_id: &str) -> Result<()>;
    async fn load_conversation(&mut self, conversation_id: &str) -> Result<()>;
    async fn force_save_conversation(&mut self);
}

impl SessionManagement for App {
    /// Load a session from storage
    async fn load_session(&mut self, conversation_id: &str) -> Result<()> {
        // Load messages from storage
        let message_metadata = self.message_store.get_by_conversation(conversation_id).await?;

        // Clear current state
        self.messages.clear();
        self.conversation_history.clear();
        self.clear_input();
        self.scroll = 0; // Reset scroll - will be set to bottom via pending_scroll_to_bottom
        self.conversation_line_count = 0; // Reset line count - will be set on next render

        // Convert and load messages
        for msg_meta in message_metadata {
            // Parse role
            let role = match msg_meta.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                "tool" => Role::Tool,
                _ => Role::User, // Default to user
            };

            // Create Message for conversation history
            let message = Message {
                role: role.clone(),
                content: MessageContent::Text(msg_meta.content.clone()),
                name: None,
                metadata: None,
            };
            self.conversation_history.push(message);

            // Create TuiMessage for display
            let tui_message = TuiMessage {
                role: msg_meta.role.clone(),
                content: msg_meta.content.clone(),
                created_at: msg_meta.created_at,
            };
            self.messages.push(tui_message);
        }

        // Update session ID
        self.session_id = conversation_id.to_string();

        // Load persisted tasks for this conversation
        let task_count = match self.task_store.get_by_conversation(conversation_id).await {
            Ok(tasks) if !tasks.is_empty() => {
                let count = tasks.len();
                let task_manager = self.task_manager.write().await;
                task_manager.load_tasks(tasks).await;
                drop(task_manager);

                // Update task tree cache
                let manager = self.task_manager.read().await;
                self.task_tree_cache = manager.format_tree().await;
                self.task_count_cache = count;
                count
            }
            _ => {
                // Clear tasks if none found or error
                let task_manager = self.task_manager.write().await;
                task_manager.clear().await;
                self.task_tree_cache = "No tasks".to_string();
                self.task_count_cache = 0;
                0
            }
        };

        // Update status
        let task_info = if task_count > 0 {
            format!(", {} tasks", task_count)
        } else {
            String::new()
        };
        self.status = format!("Loaded session: {} ({} messages{})",
                            &conversation_id[..8.min(conversation_id.len())],
                            self.messages.len(),
                            task_info);

        // Check if the last message is from the user - if so, we need to resume AI response
        // This happens when backgrounding during AI streaming (partial response was not saved)
        if let Some(last_msg) = self.messages.last() {
            if last_msg.role == "user" {
                self.pending_resume_ai = true;
            }
        }

        // Scroll to bottom after loading (will be processed after first render when line count is available)
        self.pending_scroll_to_bottom = true;

        Ok(())
    }

    /// Load a conversation from storage (public wrapper for load_session)
    async fn load_conversation(&mut self, conversation_id: &str) -> Result<()> {
        self.load_session(conversation_id).await
    }

    /// Force save all messages in the current conversation to storage
    /// Used before backgrounding to ensure state can be restored on reattach
    async fn force_save_conversation(&mut self) {
        // Skip if no messages to save
        if self.messages.is_empty() {
            return;
        }

        // Ensure conversation exists in store
        let title = if let Some(first_msg) = self.messages.first() {
            let content = &first_msg.content;
            if content.len() > 50 {
                format!("{}...", &content[..47])
            } else {
                content.clone()
            }
        } else {
            "Backgrounded session".to_string()
        };

        // Create/update conversation record with current message count
        let message_count = self.messages.len() as i32;
        if let Err(e) = self.conversation_store.create(
            self.session_id.clone(),
            Some(title),
            Some(self.model.clone()),
            Some(message_count),
        ).await {
            // May already exist, which is fine
            tracing::debug!("Conversation create result: {}", e);
        }

        // Check how many messages are already saved
        let existing_count = match self.message_store.get_by_conversation(&self.session_id).await {
            Ok(msgs) => msgs.len(),
            Err(_) => 0,
        };

        // Only save messages that haven't been persisted yet
        // Messages are saved in order, so we can skip the first `existing_count` messages
        //
        // IMPORTANT: If streaming_msg_idx is set, that means we're in the middle of
        // streaming an AI response. Don't save that partial message - the Agent will
        // detect that the last saved message is from the user and re-request the AI response.
        let skip_streaming_msg = self.streaming_msg_idx.is_some();
        let messages_to_save: Vec<(usize, &TuiMessage)> = self.messages.iter()
            .enumerate()
            .skip(existing_count)
            .filter(|(idx, _)| {
                // Skip the streaming message if we're mid-stream
                if skip_streaming_msg {
                    if let Some(streaming_idx) = self.streaming_msg_idx {
                        return *idx != streaming_idx;
                    }
                }
                true
            })
            .collect();

        if messages_to_save.is_empty() {
            tracing::debug!("All {} messages already persisted", self.messages.len());
            return;
        }

        let mut saved_count = 0;
        for (_idx, msg) in messages_to_save {
            let msg_metadata = crate::storage::MessageMetadata {
                message_id: uuid::Uuid::new_v4().to_string(),
                conversation_id: self.session_id.clone(),
                role: msg.role.clone(),
                content: msg.content.clone(),
                token_count: None,
                model_id: Some(self.model.clone()),
                images: None,
                created_at: msg.created_at,
                expires_at: None,
            };

            if let Err(e) = self.message_store.add(msg_metadata).await {
                tracing::warn!("Failed to save message on background: {}", e);
            } else {
                saved_count += 1;
            }
        }

        if skip_streaming_msg {
            tracing::info!("Saved {} messages before backgrounding (skipped partial streaming message)", saved_count);
        } else {
            tracing::info!("Saved {} new messages before backgrounding (total: {})", saved_count, self.messages.len());
        }

        // Update conversation's updated_at timestamp and message count
        if saved_count > 0 {
            let _ = self.conversation_store.update(
                &self.session_id,
                None,  // keep title
                Some(self.messages.len() as i32),  // update message count
            ).await;
        }
    }
}
