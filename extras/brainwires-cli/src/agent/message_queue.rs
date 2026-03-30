//! Message queue for agent processing
//!
//! Provides a queue for messages that need to be injected during agent processing.
//! Messages are persisted to ensure they survive agent restarts.

use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Priority level for queued messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority - processed last
    Low,
    /// Normal priority - standard processing order
    Normal,
    /// High priority - processed before normal messages
    High,
    /// System priority - processed immediately
    System,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A message queued for injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// Unique message ID
    pub id: String,
    /// Message content
    pub content: String,
    /// Priority level
    pub priority: MessagePriority,
    /// When the message was queued
    pub queued_at: DateTime<Utc>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

impl QueuedMessage {
    /// Create a new queued message with default settings
    pub fn new(content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            priority: MessagePriority::Normal,
            queued_at: Utc::now(),
            retry_count: 0,
            max_retries: 3,
            metadata: None,
        }
    }

    /// Create a high priority message
    pub fn high_priority(content: String) -> Self {
        Self {
            priority: MessagePriority::High,
            ..Self::new(content)
        }
    }

    /// Create a system priority message
    pub fn system(content: String) -> Self {
        Self {
            priority: MessagePriority::System,
            ..Self::new(content)
        }
    }

    /// Increment retry count and return updated message
    pub fn increment_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }

    /// Check if retries are exhausted
    pub fn retries_exhausted(&self) -> bool {
        self.retry_count > self.max_retries
    }
}

/// Message queue with persistence
pub struct MessageQueue {
    /// Queue of messages waiting to be processed
    queue: VecDeque<QueuedMessage>,
    /// Path for persistence (if enabled)
    persist_path: Option<PathBuf>,
    /// Maximum queue size
    max_size: usize,
}

impl MessageQueue {
    /// Create a new in-memory message queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            persist_path: None,
            max_size,
        }
    }

    /// Create a message queue with persistence
    pub fn with_persistence(max_size: usize, persist_path: PathBuf) -> Result<Self> {
        let mut queue = Self {
            queue: VecDeque::new(),
            persist_path: Some(persist_path.clone()),
            max_size,
        };

        // Load existing queue if present
        if persist_path.exists() {
            queue.load()?;
        }

        Ok(queue)
    }

    /// Push a message to the queue
    pub fn push(&mut self, message: QueuedMessage) -> Result<()> {
        if self.queue.len() >= self.max_size {
            anyhow::bail!("Message queue is full (max {})", self.max_size);
        }

        // Insert based on priority
        let position = self.find_insert_position(&message);
        self.queue.insert(position, message);

        // Persist if enabled
        if self.persist_path.is_some() {
            self.save()?;
        }

        Ok(())
    }

    /// Push content as a new message with default settings
    pub fn push_content(&mut self, content: String) -> Result<()> {
        self.push(QueuedMessage::new(content))
    }

    /// Pop the next message from the queue
    pub fn pop(&mut self) -> Option<QueuedMessage> {
        let message = self.queue.pop_front();

        // Persist if enabled and we removed something
        if message.is_some() && self.persist_path.is_some() {
            let _ = self.save(); // Best effort save
        }

        message
    }

    /// Peek at the next message without removing it
    pub fn peek(&self) -> Option<&QueuedMessage> {
        self.queue.front()
    }

    /// Re-queue a message (e.g., after failed delivery)
    pub fn requeue(&mut self, message: QueuedMessage) -> Result<()> {
        let message = message.increment_retry();

        if message.retries_exhausted() {
            tracing::warn!(
                "Message {} exhausted retries, discarding: {}",
                message.id,
                &message.content[..message.content.len().min(50)]
            );
            return Ok(());
        }

        self.push(message)
    }

    /// Get the number of messages in the queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) -> Result<()> {
        self.queue.clear();

        if self.persist_path.is_some() {
            self.save()?;
        }

        Ok(())
    }

    /// Drain all messages from the queue
    pub fn drain(&mut self) -> Vec<QueuedMessage> {
        let messages: Vec<_> = self.queue.drain(..).collect();

        if self.persist_path.is_some() {
            let _ = self.save(); // Best effort save
        }

        messages
    }

    /// Get all messages by priority
    pub fn by_priority(&self, priority: MessagePriority) -> Vec<&QueuedMessage> {
        self.queue.iter().filter(|m| m.priority == priority).collect()
    }

    /// Find the correct position to insert a message based on priority
    fn find_insert_position(&self, message: &QueuedMessage) -> usize {
        // Higher priority messages go to the front
        for (i, existing) in self.queue.iter().enumerate() {
            if Self::priority_value(message.priority) > Self::priority_value(existing.priority) {
                return i;
            }
        }
        self.queue.len()
    }

    /// Convert priority to numeric value for comparison
    fn priority_value(priority: MessagePriority) -> u8 {
        match priority {
            MessagePriority::System => 3,
            MessagePriority::High => 2,
            MessagePriority::Normal => 1,
            MessagePriority::Low => 0,
        }
    }

    /// Save queue to disk
    fn save(&self) -> Result<()> {
        if let Some(ref path) = self.persist_path {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create message queue directory")?;
            }

            let data = serde_json::to_string_pretty(&self.queue)
                .context("Failed to serialize message queue")?;

            std::fs::write(path, data).context("Failed to write message queue")?;
        }
        Ok(())
    }

    /// Load queue from disk
    fn load(&mut self) -> Result<()> {
        if let Some(ref path) = self.persist_path {
            if path.exists() {
                let data = std::fs::read_to_string(path)
                    .context("Failed to read message queue")?;

                self.queue = serde_json::from_str(&data)
                    .context("Failed to deserialize message queue")?;

                tracing::info!("Loaded {} messages from queue", self.queue.len());
            }
        }
        Ok(())
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_push_pop() {
        let mut queue = MessageQueue::new(10);

        queue.push_content("message 1".to_string()).unwrap();
        queue.push_content("message 2".to_string()).unwrap();

        assert_eq!(queue.len(), 2);

        let msg1 = queue.pop().unwrap();
        assert_eq!(msg1.content, "message 1");

        let msg2 = queue.pop().unwrap();
        assert_eq!(msg2.content, "message 2");

        assert!(queue.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = MessageQueue::new(10);

        // Add in reverse priority order
        queue.push_content("low priority".to_string()).unwrap();
        queue.push(QueuedMessage::high_priority("high priority".to_string())).unwrap();
        queue.push(QueuedMessage::system("system priority".to_string())).unwrap();

        // Should come out in priority order
        assert_eq!(queue.pop().unwrap().content, "system priority");
        assert_eq!(queue.pop().unwrap().content, "high priority");
        assert_eq!(queue.pop().unwrap().content, "low priority");
    }

    #[test]
    fn test_max_size() {
        let mut queue = MessageQueue::new(2);

        queue.push_content("msg 1".to_string()).unwrap();
        queue.push_content("msg 2".to_string()).unwrap();

        // Should fail - queue is full
        let result = queue.push_content("msg 3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_requeue_with_retry() {
        let mut queue = MessageQueue::new(10);

        let msg = QueuedMessage {
            max_retries: 2,
            ..QueuedMessage::new("test".to_string())
        };

        queue.push(msg.clone()).unwrap();
        let popped = queue.pop().unwrap();

        // Requeue - retry 1
        queue.requeue(popped).unwrap();
        assert_eq!(queue.len(), 1);

        let popped = queue.pop().unwrap();
        assert_eq!(popped.retry_count, 1);

        // Requeue - retry 2
        queue.requeue(popped).unwrap();
        let popped = queue.pop().unwrap();
        assert_eq!(popped.retry_count, 2);

        // Requeue - should be discarded (retries exhausted)
        queue.requeue(popped).unwrap();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_persistence() {
        let temp_dir = std::env::temp_dir();
        let persist_path = temp_dir.join("test_queue.json");

        // Clean up from previous runs
        let _ = std::fs::remove_file(&persist_path);

        // Create queue and add messages
        {
            let mut queue = MessageQueue::with_persistence(10, persist_path.clone()).unwrap();
            queue.push_content("persisted message".to_string()).unwrap();
        }

        // Load queue in new instance
        {
            let mut queue = MessageQueue::with_persistence(10, persist_path.clone()).unwrap();
            assert_eq!(queue.len(), 1);
            assert_eq!(queue.pop().unwrap().content, "persisted message");
        }

        // Clean up
        let _ = std::fs::remove_file(&persist_path);
    }

    #[test]
    fn test_drain() {
        let mut queue = MessageQueue::new(10);

        queue.push_content("msg 1".to_string()).unwrap();
        queue.push_content("msg 2".to_string()).unwrap();
        queue.push_content("msg 3".to_string()).unwrap();

        let messages = queue.drain();
        assert_eq!(messages.len(), 3);
        assert!(queue.is_empty());
    }
}
