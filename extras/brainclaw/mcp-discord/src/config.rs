//! Configuration types for the Discord channel adapter.

use serde::{Deserialize, Serialize};

/// Configuration for the Discord channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Discord bot token (required).
    pub discord_token: String,
    /// WebSocket URL for the brainwires-gateway.
    pub gateway_url: String,
    /// Optional authentication token for the gateway handshake.
    pub gateway_token: Option<String>,
    /// Optional command prefix for the bot (e.g., "!").
    pub bot_prefix: Option<String>,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            discord_token: String::new(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            gateway_token: None,
            bot_prefix: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_gateway_url() {
        let config = DiscordConfig::default();
        assert_eq!(config.gateway_url, "ws://127.0.0.1:18789/ws");
        assert!(config.discord_token.is_empty());
        assert!(config.gateway_token.is_none());
        assert!(config.bot_prefix.is_none());
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = DiscordConfig {
            discord_token: "test-token".to_string(),
            gateway_url: "ws://localhost:9999/ws".to_string(),
            gateway_token: Some("gw-secret".to_string()),
            bot_prefix: Some("!".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.discord_token, "test-token");
        assert_eq!(parsed.gateway_url, "ws://localhost:9999/ws");
        assert_eq!(parsed.gateway_token.as_deref(), Some("gw-secret"));
        assert_eq!(parsed.bot_prefix.as_deref(), Some("!"));
    }

    #[test]
    fn config_from_env_pattern() {
        // Verify we can construct config from individual fields (as CLI/env would)
        let token = "BOT_TOKEN_12345";
        let config = DiscordConfig {
            discord_token: token.to_string(),
            ..Default::default()
        };
        assert_eq!(config.discord_token, token);
        assert_eq!(config.gateway_url, "ws://127.0.0.1:18789/ws");
    }
}
