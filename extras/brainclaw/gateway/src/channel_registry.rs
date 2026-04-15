//! Channel registry for tracking connected channel adapters.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use brainwires_network::channels::ChannelCapabilities;

/// A connected channel adapter with its WebSocket send handle.
pub struct ConnectedChannel {
    /// Unique identifier assigned during handshake.
    pub id: Uuid,
    /// The type of channel (e.g., "discord", "telegram", "slack").
    pub channel_type: String,
    /// Capabilities advertised during handshake.
    pub capabilities: ChannelCapabilities,
    /// When the channel connected.
    pub connected_at: DateTime<Utc>,
    /// Last heartbeat/message timestamp.
    pub last_heartbeat: DateTime<Utc>,
    /// Sender for pushing messages back to this channel's WebSocket.
    pub message_tx: mpsc::Sender<String>,
}

/// Summary information about a connected channel (without the tx handle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    /// Unique identifier.
    pub id: Uuid,
    /// The type of channel.
    pub channel_type: String,
    /// Capabilities as a bitflag integer.
    pub capabilities: ChannelCapabilities,
    /// When the channel connected.
    pub connected_at: DateTime<Utc>,
    /// Last heartbeat/message timestamp.
    pub last_heartbeat: DateTime<Utc>,
}

/// Registry of all currently connected channel adapters.
pub struct ChannelRegistry {
    channels: DashMap<Uuid, ConnectedChannel>,
}

impl ChannelRegistry {
    /// Create a new empty channel registry.
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    /// Register a newly connected channel.
    pub fn register(&self, channel: ConnectedChannel) {
        self.channels.insert(channel.id, channel);
    }

    /// Unregister a channel by its ID (e.g., on disconnect).
    pub fn unregister(&self, id: &Uuid) {
        self.channels.remove(id);
    }

    /// Get the message sender for a specific channel.
    pub fn get_sender(&self, id: &Uuid) -> Option<mpsc::Sender<String>> {
        self.channels.get(id).map(|entry| entry.message_tx.clone())
    }

    /// Get summary info for a specific channel.
    pub fn get_info(&self, id: &Uuid) -> Option<ChannelInfo> {
        self.channels.get(id).map(|entry| ChannelInfo {
            id: entry.id,
            channel_type: entry.channel_type.clone(),
            capabilities: entry.capabilities,
            connected_at: entry.connected_at,
            last_heartbeat: entry.last_heartbeat,
        })
    }

    /// List summary info for all connected channels.
    pub fn list(&self) -> Vec<ChannelInfo> {
        self.channels
            .iter()
            .map(|entry| ChannelInfo {
                id: entry.id,
                channel_type: entry.channel_type.clone(),
                capabilities: entry.capabilities,
                connected_at: entry.connected_at,
                last_heartbeat: entry.last_heartbeat,
            })
            .collect()
    }

    /// Find all channel IDs matching a given channel type.
    pub fn find_by_type(&self, channel_type: &str) -> Vec<Uuid> {
        self.channels
            .iter()
            .filter(|entry| entry.channel_type == channel_type)
            .map(|entry| entry.id)
            .collect()
    }

    /// Update the last heartbeat timestamp for a channel.
    pub fn touch_heartbeat(&self, id: &Uuid) {
        if let Some(mut entry) = self.channels.get_mut(id) {
            entry.last_heartbeat = Utc::now();
        }
    }

    /// Return the number of connected channels.
    pub fn count(&self) -> usize {
        self.channels.len()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel(channel_type: &str) -> ConnectedChannel {
        let (tx, _rx) = mpsc::channel(16);
        ConnectedChannel {
            id: Uuid::new_v4(),
            channel_type: channel_type.to_string(),
            capabilities: ChannelCapabilities::RICH_TEXT | ChannelCapabilities::REACTIONS,
            connected_at: Utc::now(),
            last_heartbeat: Utc::now(),
            message_tx: tx,
        }
    }

    #[test]
    fn register_and_list() {
        let registry = ChannelRegistry::new();
        let ch = make_channel("discord");
        let id = ch.id;
        registry.register(ch);

        assert_eq!(registry.count(), 1);
        let infos = registry.list();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].id, id);
        assert_eq!(infos[0].channel_type, "discord");
    }

    #[test]
    fn unregister_removes_channel() {
        let registry = ChannelRegistry::new();
        let ch = make_channel("telegram");
        let id = ch.id;
        registry.register(ch);
        assert_eq!(registry.count(), 1);

        registry.unregister(&id);
        assert_eq!(registry.count(), 0);
        assert!(registry.get_info(&id).is_none());
    }

    #[test]
    fn get_info_for_missing_returns_none() {
        let registry = ChannelRegistry::new();
        assert!(registry.get_info(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn get_sender_returns_sender() {
        let registry = ChannelRegistry::new();
        let ch = make_channel("slack");
        let id = ch.id;
        registry.register(ch);

        assert!(registry.get_sender(&id).is_some());
        assert!(registry.get_sender(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn find_by_type_filters_correctly() {
        let registry = ChannelRegistry::new();
        let ch1 = make_channel("discord");
        let id1 = ch1.id;
        registry.register(ch1);
        registry.register(make_channel("telegram"));
        let ch3 = make_channel("discord");
        let id3 = ch3.id;
        registry.register(ch3);

        let discord_ids = registry.find_by_type("discord");
        assert_eq!(discord_ids.len(), 2);
        assert!(discord_ids.contains(&id1));
        assert!(discord_ids.contains(&id3));

        let telegram_ids = registry.find_by_type("telegram");
        assert_eq!(telegram_ids.len(), 1);

        let slack_ids = registry.find_by_type("slack");
        assert!(slack_ids.is_empty());
    }
}
