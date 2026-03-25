//! Serenity `EventHandler` implementation that converts Discord events to `ChannelEvent`.

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::event::{MessageUpdateEvent, TypingStartEvent};
use serenity::model::gateway::Ready;
use serenity::model::prelude::{ChannelId, GuildId, Reaction};
use serenity::prelude::{Context, EventHandler};
use tokio::sync::mpsc;

use brainwires_channels::{ChannelEvent, ChannelUser, ConversationId, MessageId};

use crate::discord::discord_message_to_channel_message;

/// Serenity event handler that forwards Discord events as `ChannelEvent` values
/// over an mpsc channel.
pub struct DiscordEventHandler {
    /// Sender for forwarding events to the gateway client loop.
    pub event_tx: mpsc::Sender<ChannelEvent>,
}

impl DiscordEventHandler {
    /// Create a new event handler with the given event sender.
    pub fn new(event_tx: mpsc::Sender<ChannelEvent>) -> Self {
        Self { event_tx }
    }
}

#[async_trait]
impl EventHandler for DiscordEventHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!(
            user = %ready.user.name,
            guilds = ready.guilds.len(),
            "Discord bot connected"
        );
    }

    async fn message(&self, _ctx: Context, msg: Message) {
        // Skip bot messages to avoid loops
        if msg.author.bot {
            return;
        }

        let guild_id = msg
            .guild_id
            .map(|g| g.to_string())
            .unwrap_or_else(|| "0".to_string());

        let channel_message = discord_message_to_channel_message(&msg, &guild_id);
        let event = ChannelEvent::MessageReceived(channel_message);

        if let Err(e) = self.event_tx.send(event).await {
            tracing::error!("Failed to forward message event: {}", e);
        }
    }

    async fn message_update(
        &self,
        _ctx: Context,
        _old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        if let Some(msg) = new {
            let guild_id = event
                .guild_id
                .map(|g| g.to_string())
                .unwrap_or_else(|| "0".to_string());

            let channel_message = discord_message_to_channel_message(&msg, &guild_id);
            let evt = ChannelEvent::MessageEdited(channel_message);

            if let Err(e) = self.event_tx.send(evt).await {
                tracing::error!("Failed to forward message_update event: {}", e);
            }
        } else {
            tracing::debug!(
                message_id = %event.id,
                "message_update received without full message (cache miss)"
            );
        }
    }

    async fn message_delete(
        &self,
        _ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: serenity::model::id::MessageId,
        guild_id: Option<GuildId>,
    ) {
        let event = ChannelEvent::MessageDeleted {
            message_id: MessageId::new(deleted_message_id.to_string()),
            conversation: ConversationId {
                platform: "discord".to_string(),
                channel_id: channel_id.to_string(),
                server_id: guild_id.map(|g| g.to_string()),
            },
        };

        if let Err(e) = self.event_tx.send(event).await {
            tracing::error!("Failed to forward message_delete event: {}", e);
        }
    }

    async fn reaction_add(&self, _ctx: Context, reaction: Reaction) {
        let user = reaction
            .user_id
            .map(|uid| ChannelUser {
                platform: "discord".to_string(),
                platform_user_id: uid.to_string(),
                display_name: uid.to_string(),
                username: None,
                avatar_url: None,
            })
            .unwrap_or_else(|| ChannelUser {
                platform: "discord".to_string(),
                platform_user_id: "unknown".to_string(),
                display_name: "unknown".to_string(),
                username: None,
                avatar_url: None,
            });

        let emoji = reaction.emoji.to_string();

        let event = ChannelEvent::ReactionAdded {
            message_id: MessageId::new(reaction.message_id.to_string()),
            user,
            emoji,
        };

        if let Err(e) = self.event_tx.send(event).await {
            tracing::error!("Failed to forward reaction_add event: {}", e);
        }
    }

    async fn reaction_remove(&self, _ctx: Context, reaction: Reaction) {
        let user = reaction
            .user_id
            .map(|uid| ChannelUser {
                platform: "discord".to_string(),
                platform_user_id: uid.to_string(),
                display_name: uid.to_string(),
                username: None,
                avatar_url: None,
            })
            .unwrap_or_else(|| ChannelUser {
                platform: "discord".to_string(),
                platform_user_id: "unknown".to_string(),
                display_name: "unknown".to_string(),
                username: None,
                avatar_url: None,
            });

        let emoji = reaction.emoji.to_string();

        let event = ChannelEvent::ReactionRemoved {
            message_id: MessageId::new(reaction.message_id.to_string()),
            user,
            emoji,
        };

        if let Err(e) = self.event_tx.send(event).await {
            tracing::error!("Failed to forward reaction_remove event: {}", e);
        }
    }

    async fn typing_start(&self, _ctx: Context, event: TypingStartEvent) {
        let user = ChannelUser {
            platform: "discord".to_string(),
            platform_user_id: event.user_id.to_string(),
            display_name: event.user_id.to_string(),
            username: None,
            avatar_url: None,
        };

        let evt = ChannelEvent::TypingStarted {
            conversation: ConversationId {
                platform: "discord".to_string(),
                channel_id: event.channel_id.to_string(),
                server_id: event.guild_id.map(|g| g.to_string()),
            },
            user,
        };

        if let Err(e) = self.event_tx.send(evt).await {
            tracing::error!("Failed to forward typing_start event: {}", e);
        }
    }
}
