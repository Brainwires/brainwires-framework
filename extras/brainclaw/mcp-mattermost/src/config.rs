//! Configuration for the Mattermost channel adapter.

use serde::{Deserialize, Serialize};

/// Configuration for the Mattermost channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    /// Mattermost server base URL (e.g. "https://mattermost.example.com").
    pub server_url: String,
    /// Personal access token for authentication.
    pub access_token: String,
    /// The bot's Mattermost user ID. Used to filter out self-messages.
    pub bot_user_id: String,
    /// WebSocket URL of the brainwires-gateway.
    pub gateway_url: String,
    /// Optional auth token for the gateway handshake.
    pub gateway_token: Option<String>,
    /// Team name or ID to scope channel operations.
    pub team_id: Option<String>,
    /// In public/private channels, only respond when the bot is @mentioned.
    /// DMs (direct message channels) always respond.
    #[serde(default)]
    pub group_mention_required: bool,
    /// The bot's display name or username (e.g. "@mybot") used to detect
    /// mentions in channel messages.
    #[serde(default)]
    pub bot_username: Option<String>,
    /// Additional keyword patterns (case-insensitive) that trigger a
    /// response in group channels even without an @mention.
    #[serde(default)]
    pub mention_patterns: Vec<String>,
    /// Channel IDs to include. Empty = all subscribed channels.
    #[serde(default)]
    pub channel_allowlist: Vec<String>,
}

impl Default for MattermostConfig {
    fn default() -> Self {
        Self {
            server_url: "https://mattermost.example.com".to_string(),
            access_token: String::new(),
            bot_user_id: String::new(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            gateway_token: None,
            team_id: None,
            group_mention_required: false,
            bot_username: None,
            mention_patterns: Vec::new(),
            channel_allowlist: Vec::new(),
        }
    }
}
