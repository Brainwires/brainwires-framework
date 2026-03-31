//! Configuration types for the Matrix channel adapter.

use serde::Deserialize;

/// Configuration for the Matrix channel adapter.
#[derive(Debug, Clone, Deserialize)]
pub struct MatrixConfig {
    /// Matrix homeserver URL (e.g. "https://matrix.org").
    pub homeserver_url: String,

    /// Matrix username (localpart only, e.g. "mybot" — not "@mybot:server").
    pub username: String,

    /// Matrix account password.
    pub password: String,

    /// Device display name shown in session list.
    pub device_name: String,

    /// WebSocket URL of the brainwires-gateway.
    pub gateway_url: String,

    /// Optional auth token for the gateway handshake.
    pub gateway_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config_json(gateway_token: Option<&str>) -> serde_json::Value {
        let mut v = serde_json::json!({
            "homeserver_url": "https://matrix.org",
            "username": "mybot",
            "password": "s3cr3t",
            "device_name": "BrainwireBot",
            "gateway_url": "ws://localhost:9000"
        });
        if let Some(tok) = gateway_token {
            v["gateway_token"] = serde_json::json!(tok);
        }
        v
    }

    #[test]
    fn config_deserializes_from_json() {
        let json = make_config_json(None);
        let cfg: MatrixConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.homeserver_url, "https://matrix.org");
        assert_eq!(cfg.username, "mybot");
        assert_eq!(cfg.password, "s3cr3t");
        assert_eq!(cfg.device_name, "BrainwireBot");
        assert_eq!(cfg.gateway_url, "ws://localhost:9000");
        assert!(cfg.gateway_token.is_none());
    }

    #[test]
    fn config_deserializes_with_gateway_token() {
        let json = make_config_json(Some("gw-token-abc"));
        let cfg: MatrixConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.gateway_token.as_deref(), Some("gw-token-abc"));
    }

    #[test]
    fn config_missing_required_field_errors() {
        let json = serde_json::json!({
            "username": "mybot",
            "password": "s3cr3t",
            "device_name": "Bot",
            "gateway_url": "ws://localhost:9000"
            // homeserver_url is missing
        });
        let result = serde_json::from_value::<MatrixConfig>(json);
        assert!(result.is_err());
    }

    #[test]
    fn config_clone() {
        let json = make_config_json(None);
        let cfg: MatrixConfig = serde_json::from_value(json).unwrap();
        let cloned = cfg.clone();
        assert_eq!(cloned.homeserver_url, cfg.homeserver_url);
        assert_eq!(cloned.username, cfg.username);
    }
}
