//! Configuration types for the Telegram channel adapter.

use serde::{Deserialize, Serialize};

/// Configuration for the Telegram channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Telegram bot token (required). Obtain from @BotFather.
    pub telegram_token: String,
    /// WebSocket URL for the brainwires-gateway.
    pub gateway_url: String,
    /// Optional authentication token for the gateway handshake.
    pub gateway_token: Option<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            telegram_token: String::new(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            gateway_token: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_gateway_url() {
        let config = TelegramConfig::default();
        assert_eq!(config.gateway_url, "ws://127.0.0.1:18789/ws");
        assert!(config.telegram_token.is_empty());
        assert!(config.gateway_token.is_none());
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = TelegramConfig {
            telegram_token: "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11".to_string(),
            gateway_url: "ws://localhost:9999/ws".to_string(),
            gateway_token: Some("gw-secret".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TelegramConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.telegram_token, "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11");
        assert_eq!(parsed.gateway_url, "ws://localhost:9999/ws");
        assert_eq!(parsed.gateway_token.as_deref(), Some("gw-secret"));
    }

    #[test]
    fn config_from_env_pattern() {
        let token = "123456:BOT_TOKEN_TEST";
        let config = TelegramConfig {
            telegram_token: token.to_string(),
            ..Default::default()
        };
        assert_eq!(config.telegram_token, token);
        assert_eq!(config.gateway_url, "ws://127.0.0.1:18789/ws");
    }
}
