//! Checkpoint system for conversation history
//!
//! Allows creating named snapshots of conversation state that can be restored later.
//! Checkpoints are stored as JSON files in the platform-specific config directory.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::PlatformPaths;
use crate::types::message::Message;

/// A checkpoint represents a saved conversation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID
    pub id: String,
    /// Optional user-friendly name
    pub name: Option<String>,
    /// Conversation ID this checkpoint belongs to
    pub conversation_id: String,
    /// Messages at the time of checkpoint
    pub messages: Vec<Message>,
    /// Metadata (model, user info, etc.)
    pub metadata: HashMap<String, String>,
    /// Timestamp when checkpoint was created
    pub created_at: i64,
}

/// Manages conversation checkpoints
pub struct CheckpointManager {
    /// Directory where checkpoints are stored
    checkpoints_dir: PathBuf,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new() -> Result<Self> {
        let checkpoints_dir = PlatformPaths::config_dir()?.join("checkpoints");

        // Create checkpoints directory if it doesn't exist
        if !checkpoints_dir.exists() {
            fs::create_dir_all(&checkpoints_dir)
                .context("Failed to create checkpoints directory")?;
        }

        Ok(Self { checkpoints_dir })
    }

    /// Create a checkpoint from current conversation state
    pub async fn create_checkpoint(
        &self,
        name: Option<String>,
        conversation_id: String,
        messages: Vec<Message>,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();

        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            name,
            conversation_id,
            messages,
            metadata,
            created_at: Utc::now().timestamp(),
        };

        // Save checkpoint to disk
        self.save_checkpoint(&checkpoint).await?;

        Ok(checkpoint_id)
    }

    /// Save a checkpoint to disk
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let checkpoint_path = self.checkpoints_dir.join(format!("{}.json", checkpoint.id));

        let json = serde_json::to_string_pretty(checkpoint)
            .context("Failed to serialize checkpoint")?;

        fs::write(&checkpoint_path, json)
            .with_context(|| format!("Failed to write checkpoint to {:?}", checkpoint_path))?;

        Ok(())
    }

    /// Load a checkpoint from disk
    pub async fn load_checkpoint(&self, checkpoint_id: &str) -> Result<Checkpoint> {
        let checkpoint_path = self.checkpoints_dir.join(format!("{}.json", checkpoint_id));

        let json = fs::read_to_string(&checkpoint_path)
            .with_context(|| format!("Failed to read checkpoint from {:?}", checkpoint_path))?;

        let checkpoint: Checkpoint = serde_json::from_str(&json)
            .context("Failed to deserialize checkpoint")?;

        Ok(checkpoint)
    }

    /// Restore conversation state from a checkpoint
    pub async fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<Checkpoint> {
        self.load_checkpoint(checkpoint_id).await
    }

    /// List all checkpoints for a conversation
    pub async fn list_checkpoints(&self, conversation_id: &str) -> Result<Vec<Checkpoint>> {
        let mut checkpoints = Vec::new();

        // Read all checkpoint files
        let entries = fs::read_dir(&self.checkpoints_dir)
            .context("Failed to read checkpoints directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&json) {
                        if checkpoint.conversation_id == conversation_id {
                            checkpoints.push(checkpoint);
                        }
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(checkpoints)
    }

    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let checkpoint_path = self.checkpoints_dir.join(format!("{}.json", checkpoint_id));

        if checkpoint_path.exists() {
            fs::remove_file(&checkpoint_path)
                .with_context(|| format!("Failed to delete checkpoint {:?}", checkpoint_path))?;
        }

        Ok(())
    }

    /// Get the most recent checkpoint for a conversation
    pub async fn get_latest_checkpoint(&self, conversation_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list_checkpoints(conversation_id).await?;
        Ok(checkpoints.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::message::{MessageContent, Role};

    #[test]
    fn test_checkpoint_manager_new() {
        let manager = CheckpointManager::new();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_create_and_load_checkpoint() {
        let manager = CheckpointManager::new().unwrap();

        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Test message".to_string()),
                name: None,
                metadata: None,
            }
        ];

        let mut metadata = HashMap::new();
        metadata.insert("model".to_string(), "test-model".to_string());

        let checkpoint_id = manager.create_checkpoint(
            Some("Test Checkpoint".to_string()),
            "conv-123".to_string(),
            messages.clone(),
            metadata.clone(),
        ).await.unwrap();

        // Load the checkpoint
        let loaded = manager.load_checkpoint(&checkpoint_id).await.unwrap();
        assert_eq!(loaded.id, checkpoint_id);
        assert_eq!(loaded.name, Some("Test Checkpoint".to_string()));
        assert_eq!(loaded.conversation_id, "conv-123");
        assert_eq!(loaded.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let manager = CheckpointManager::new().unwrap();

        let messages = vec![];
        let metadata = HashMap::new();

        // Create multiple checkpoints for the same conversation
        let _id1 = manager.create_checkpoint(
            Some("Checkpoint 1".to_string()),
            "conv-456".to_string(),
            messages.clone(),
            metadata.clone(),
        ).await.unwrap();

        let _id2 = manager.create_checkpoint(
            Some("Checkpoint 2".to_string()),
            "conv-456".to_string(),
            messages.clone(),
            metadata.clone(),
        ).await.unwrap();

        // List checkpoints
        let checkpoints = manager.list_checkpoints("conv-456").await.unwrap();
        assert!(checkpoints.len() >= 2);
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let manager = CheckpointManager::new().unwrap();

        let messages = vec![];
        let metadata = HashMap::new();

        let checkpoint_id = manager.create_checkpoint(
            Some("To Delete".to_string()),
            "conv-789".to_string(),
            messages,
            metadata,
        ).await.unwrap();

        // Delete the checkpoint
        let result = manager.delete_checkpoint(&checkpoint_id).await;
        assert!(result.is_ok());

        // Try to load - should fail
        let load_result = manager.load_checkpoint(&checkpoint_id).await;
        assert!(load_result.is_err());
    }
}
