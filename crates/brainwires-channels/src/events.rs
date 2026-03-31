//! Channel events representing things that happen on a messaging platform.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::identity::{ChannelUser, ConversationId};
use crate::message::{ChannelMessage, MessageId, ThreadId};

/// An event from a messaging channel.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ChannelEvent {
    /// A new message was received.
    MessageReceived(ChannelMessage),
    /// An existing message was edited.
    MessageEdited(ChannelMessage),
    /// A message was deleted.
    MessageDeleted {
        /// The ID of the deleted message.
        message_id: MessageId,
        /// The conversation the message was in.
        conversation: ConversationId,
    },
    /// A reaction was added to a message.
    ReactionAdded {
        /// The message that was reacted to.
        message_id: MessageId,
        /// The user who added the reaction.
        user: ChannelUser,
        /// The emoji used for the reaction.
        emoji: String,
    },
    /// A reaction was removed from a message.
    ReactionRemoved {
        /// The message the reaction was removed from.
        message_id: MessageId,
        /// The user who removed the reaction.
        user: ChannelUser,
        /// The emoji that was removed.
        emoji: String,
    },
    /// A user started typing in a conversation.
    TypingStarted {
        /// The conversation where typing is occurring.
        conversation: ConversationId,
        /// The user who started typing.
        user: ChannelUser,
    },
    /// A user's presence status changed.
    PresenceChanged {
        /// The user whose presence changed.
        user: ChannelUser,
        /// The new presence status.
        status: PresenceStatus,
    },
    /// A new thread was created from a message.
    ThreadCreated {
        /// The parent message that spawned the thread.
        parent_message_id: MessageId,
        /// The ID of the newly created thread.
        thread_id: ThreadId,
    },
}

/// A user's presence status on the platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PresenceStatus {
    /// User is online and active.
    Online,
    /// User is away or idle.
    Away,
    /// User has enabled do-not-disturb mode.
    DoNotDisturb,
    /// User is offline.
    Offline,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::ConversationId;
    use crate::message::MessageId;

    #[test]
    fn channel_event_serde_roundtrip() {
        let event = ChannelEvent::MessageDeleted {
            message_id: MessageId::new("msg-123"),
            conversation: ConversationId {
                platform: "discord".to_string(),
                channel_id: "general".to_string(),
                server_id: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ChannelEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            ChannelEvent::MessageDeleted {
                message_id,
                conversation,
            } => {
                assert_eq!(message_id, MessageId::new("msg-123"));
                assert_eq!(conversation.channel_id, "general");
            }
            _ => panic!("expected MessageDeleted variant"),
        }
    }

    #[test]
    fn presence_status_serde_roundtrip() {
        let status = PresenceStatus::DoNotDisturb;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: PresenceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}
