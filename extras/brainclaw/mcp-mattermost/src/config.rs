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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = MattermostConfig::default();
        assert_eq!(cfg.server_url, "https://mattermost.example.com");
        assert_eq!(cfg.gateway_url, "ws://127.0.0.1:18789/ws");
        assert!(cfg.access_token.is_empty());
        assert!(cfg.bot_user_id.is_empty());
        assert!(cfg.gateway_token.is_none());
        assert!(cfg.team_id.is_none());
        assert!(!cfg.group_mention_required);
        assert!(cfg.bot_username.is_none());
        assert!(cfg.mention_patterns.is_empty());
        assert!(cfg.channel_allowlist.is_empty());
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = MattermostConfig {
            server_url: "https://chat.example.com".to_string(),
            access_token: "tok-abc".to_string(),
            bot_user_id: "u123".to_string(),
            gateway_url: "ws://gw:18789/ws".to_string(),
            gateway_token: Some("gw-secret".to_string()),
            team_id: Some("team-x".to_string()),
            group_mention_required: true,
            bot_username: Some("@mybot".to_string()),
            mention_patterns: vec!["help".to_string(), "brainwires".to_string()],
            channel_allowlist: vec!["ch1".to_string(), "ch2".to_string()],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: MattermostConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_url, cfg.server_url);
        assert_eq!(back.access_token, cfg.access_token);
        assert_eq!(back.bot_user_id, cfg.bot_user_id);
        assert_eq!(back.gateway_url, cfg.gateway_url);
        assert_eq!(back.gateway_token, cfg.gateway_token);
        assert_eq!(back.team_id, cfg.team_id);
        assert_eq!(back.group_mention_required, cfg.group_mention_required);
        assert_eq!(back.bot_username, cfg.bot_username);
        assert_eq!(back.mention_patterns, cfg.mention_patterns);
        assert_eq!(back.channel_allowlist, cfg.channel_allowlist);
    }

    #[test]
    fn config_defaults_applied_for_missing_serde_fields() {
        // Minimal JSON with only required fields
        let json = r#"{
            "server_url": "https://mm.example.com",
            "access_token": "tok",
            "bot_user_id": "uid",
            "gateway_url": "ws://localhost/ws"
        }"#;
        let cfg: MattermostConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.group_mention_required);
        assert!(cfg.mention_patterns.is_empty());
        assert!(cfg.channel_allowlist.is_empty());
    }
}
