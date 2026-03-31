//! WhatsApp adapter configuration.

/// Configuration for the WhatsApp channel adapter.
#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    /// Meta Graph API access token (WhatsApp Business token).
    pub token: String,
    /// WhatsApp phone number ID (from the Meta Business dashboard).
    pub phone_number_id: String,
    /// Webhook verify token (used to verify Meta's GET challenge request).
    pub verify_token: String,
    /// Port for the local Axum webhook server.
    pub webhook_port: u16,
    /// WebSocket URL of the brainwires-gateway.
    pub gateway_url: String,
    /// Optional auth token for the gateway handshake.
    pub gateway_token: Option<String>,
    /// Optional app secret for validating X-Hub-Signature-256 on webhook POSTs.
    pub app_secret: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> WhatsAppConfig {
        WhatsAppConfig {
            token: "EAAGtoken123".to_string(),
            phone_number_id: "107999999".to_string(),
            verify_token: "my_verify_token".to_string(),
            webhook_port: 8080,
            gateway_url: "ws://localhost:9000".to_string(),
            gateway_token: None,
            app_secret: None,
        }
    }

    #[test]
    fn config_fields_accessible() {
        let cfg = make_config();
        assert_eq!(cfg.token, "EAAGtoken123");
        assert_eq!(cfg.phone_number_id, "107999999");
        assert_eq!(cfg.verify_token, "my_verify_token");
        assert_eq!(cfg.webhook_port, 8080);
        assert!(cfg.gateway_token.is_none());
        assert!(cfg.app_secret.is_none());
    }

    #[test]
    fn config_with_optional_fields() {
        let mut cfg = make_config();
        cfg.gateway_token = Some("gw-token".to_string());
        cfg.app_secret = Some("app-secret-xyz".to_string());
        assert_eq!(cfg.gateway_token.as_deref(), Some("gw-token"));
        assert_eq!(cfg.app_secret.as_deref(), Some("app-secret-xyz"));
    }

    #[test]
    fn config_clone() {
        let cfg = make_config();
        let cloned = cfg.clone();
        assert_eq!(cloned.token, cfg.token);
        assert_eq!(cloned.phone_number_id, cfg.phone_number_id);
    }
}
