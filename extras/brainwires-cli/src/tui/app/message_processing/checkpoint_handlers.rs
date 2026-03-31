//! Checkpoint Handlers
//!
//! Handles checkpoint create, restore, and list operations.

use super::super::state::{App, TuiMessage};
use crate::types::message::{MessageContent, Role};
use anyhow::Result;

impl App {
    /// Handle create checkpoint command
    pub(super) async fn handle_create_checkpoint(&mut self, name: Option<String>) -> Result<()> {
        let messages = self.conversation_history.clone();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("model".to_string(), self.model.clone());

        match self
            .checkpoint_manager
            .create_checkpoint(name.clone(), self.session_id.clone(), messages, metadata)
            .await
        {
            Ok(checkpoint_id) => {
                let display_name = name.unwrap_or_else(|| checkpoint_id[..8].to_string());
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: format!("Checkpoint created: {}", display_name),
                    created_at: chrono::Utc::now().timestamp(),
                });
                self.status = format!("Checkpoint created: {}", display_name);
            }
            Err(e) => {
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: format!("Failed to create checkpoint: {}", e),
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }
        self.clear_input();
        Ok(())
    }

    /// Handle restore checkpoint command
    pub(super) async fn handle_restore_checkpoint(&mut self, checkpoint_id: String) -> Result<()> {
        match self
            .checkpoint_manager
            .restore_checkpoint(&checkpoint_id)
            .await
        {
            Ok(checkpoint) => {
                // Restore messages
                self.messages.clear();
                self.conversation_history.clear();

                for msg in &checkpoint.messages {
                    let role = match msg.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::System => "system",
                        Role::Tool => "tool",
                    }
                    .to_string();

                    let content = match &msg.content {
                        MessageContent::Text(t) => t.clone(),
                        MessageContent::Blocks(_) => "[Content blocks]".to_string(),
                    };

                    self.messages.push(TuiMessage {
                        role,
                        content,
                        created_at: chrono::Utc::now().timestamp(),
                    });
                    self.conversation_history.push(msg.clone());
                }

                let display_name = checkpoint
                    .name
                    .unwrap_or_else(|| checkpoint.id[..8].to_string());
                self.status = format!("Restored checkpoint: {}", display_name);
            }
            Err(e) => {
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: format!("Failed to restore checkpoint: {}", e),
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }
        self.clear_input();
        Ok(())
    }

    /// Handle list checkpoints command
    pub(super) async fn handle_list_checkpoints(&mut self) -> Result<()> {
        match self
            .checkpoint_manager
            .list_checkpoints(&self.session_id)
            .await
        {
            Ok(checkpoints) => {
                let content = if checkpoints.is_empty() {
                    "No checkpoints found".to_string()
                } else {
                    let mut lines = vec!["Checkpoints:".to_string()];
                    for (i, checkpoint) in checkpoints.iter().enumerate() {
                        let name = checkpoint.name.as_deref().unwrap_or("Unnamed");
                        let created = chrono::DateTime::from_timestamp(checkpoint.created_at, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        lines.push(format!(
                            "{}. {} - {} messages ({})",
                            i + 1,
                            name,
                            checkpoint.messages.len(),
                            created
                        ));
                        lines.push(format!("   ID: {}", &checkpoint.id[..8]));
                    }
                    lines.join("\n")
                };

                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content,
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
            Err(e) => {
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: format!("Failed to list checkpoints: {}", e),
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }
        self.clear_input();
        Ok(())
    }
}
