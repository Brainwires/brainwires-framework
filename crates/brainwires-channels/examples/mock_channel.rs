//! Mock channel example — reference implementation of the `Channel` trait.
//!
//! Shows how to implement the universal `Channel` contract for a new messaging
//! platform. The `MockChannel` here stores messages in memory and is useful as
//! a template when wiring in Discord, Telegram, Slack, or any other adapter.
//!
//! ```bash
//! cargo run -p brainwires-channels --example mock_channel
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use async_trait::async_trait;
use brainwires_channels::{
    Channel, ChannelCapabilities, ChannelMessage, ConversationId, MessageContent, MessageId,
};
use chrono::Utc;

// ── MockChannel ──────────────────────────────────────────────────────────────

/// In-memory channel adapter — stores sent messages in a `Vec`.
///
/// Acts as the reference blueprint for real channel adapters (Discord, Telegram,
/// Slack, etc.). A real adapter would replace the `Vec` with calls to the
/// platform's REST/WebSocket API.
#[derive(Default)]
pub struct MockChannel {
    /// Stores every sent/edited message, keyed by `MessageId`.
    messages: Arc<Mutex<HashMap<String, ChannelMessage>>>,
    /// Auto-incrementing message counter for generating unique IDs.
    counter: Arc<Mutex<u64>>,
}

impl MockChannel {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> MessageId {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        MessageId::new(format!("msg-{:04}", *c))
    }

    /// Return all messages sent to a conversation (in insertion order).
    pub fn all_messages(&self, target: &ConversationId) -> Vec<ChannelMessage> {
        self.messages
            .lock()
            .unwrap()
            .values()
            .filter(|m| m.conversation == *target)
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Channel for MockChannel {
    fn channel_type(&self) -> &str {
        "mock"
    }

    fn capabilities(&self) -> ChannelCapabilities {
        // Declare what this adapter supports
        ChannelCapabilities::RICH_TEXT
            | ChannelCapabilities::REACTIONS
            | ChannelCapabilities::EDIT_MESSAGES
            | ChannelCapabilities::DELETE_MESSAGES
            | ChannelCapabilities::TYPING_INDICATOR
    }

    async fn send_message(
        &self,
        target: &ConversationId,
        message: &ChannelMessage,
    ) -> Result<MessageId> {
        let id = self.next_id();
        let mut stored = message.clone();
        stored.id = id.clone();
        stored.conversation = target.clone();
        self.messages
            .lock()
            .unwrap()
            .insert(id.0.clone(), stored);
        println!("[mock] sent {} → {:?}", id, message.content);
        Ok(id)
    }

    async fn edit_message(&self, id: &MessageId, message: &ChannelMessage) -> Result<()> {
        let mut map = self.messages.lock().unwrap();
        if let Some(entry) = map.get_mut(&id.0) {
            entry.content = message.content.clone();
            println!("[mock] edited {id}");
        }
        Ok(())
    }

    async fn delete_message(&self, id: &MessageId) -> Result<()> {
        self.messages.lock().unwrap().remove(&id.0);
        println!("[mock] deleted {id}");
        Ok(())
    }

    async fn send_typing(&self, target: &ConversationId) -> Result<()> {
        println!("[mock] typing in {}/{}", target.platform, target.channel_id);
        Ok(())
    }

    async fn add_reaction(&self, id: &MessageId, emoji: &str) -> Result<()> {
        println!("[mock] reacted to {id} with {emoji}");
        Ok(())
    }

    async fn get_history(
        &self,
        target: &ConversationId,
        limit: usize,
    ) -> Result<Vec<ChannelMessage>> {
        let mut msgs = self.all_messages(target);
        msgs.sort_by_key(|m| m.timestamp);
        msgs.truncate(limit);
        Ok(msgs)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_message(conv: ConversationId, author: &str, text: &str) -> ChannelMessage {
    ChannelMessage {
        id: MessageId::new("pending"),
        conversation: conv,
        author: author.to_string(),
        content: MessageContent::Text(text.to_string()),
        thread_id: None,
        reply_to: None,
        timestamp: Utc::now(),
        attachments: vec![],
        metadata: HashMap::new(),
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let channel = MockChannel::new();

    let conv = ConversationId {
        platform: "mock".to_string(),
        channel_id: "general".to_string(),
        server_id: None,
    };

    // Capabilities
    let caps = channel.capabilities();
    println!("Capabilities: {caps:?}\n");

    // Send a few messages
    let id1 = channel
        .send_message(&conv, &make_message(conv.clone(), "alice", "Hello!"))
        .await?;
    let id2 = channel
        .send_message(&conv, &make_message(conv.clone(), "bot", "Hi Alice!"))
        .await?;

    // Typing indicator
    channel.send_typing(&conv).await?;

    // Add a reaction
    channel.add_reaction(&id2, "👍").await?;

    // Edit the bot's reply
    channel
        .edit_message(&id2, &make_message(conv.clone(), "bot", "Hi Alice! How can I help?"))
        .await?;

    // Fetch history
    let history = channel.get_history(&conv, 10).await?;
    println!("\nHistory ({} messages):", history.len());
    for msg in &history {
        if let MessageContent::Text(t) = &msg.content {
            println!("  [{}] {}: {}", msg.id, msg.author, t);
        }
    }

    // Delete the first message
    channel.delete_message(&id1).await?;
    println!(
        "\nAfter delete: {} message(s) remaining",
        channel.all_messages(&conv).len()
    );

    println!("\nDone.");
    Ok(())
}
