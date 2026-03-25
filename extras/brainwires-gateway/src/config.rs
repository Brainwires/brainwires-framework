//! Gateway configuration.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for the gateway daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Host address to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Maximum number of concurrent channel connections.
    pub max_connections: usize,
    /// Session inactivity timeout before automatic cleanup.
    pub session_timeout: Duration,
    /// Allowed API keys for channel connections.
    pub auth_tokens: Vec<String>,
    /// Whether the webhook endpoint is enabled.
    pub webhook_enabled: bool,
    /// URL path for the webhook endpoint.
    pub webhook_path: String,
    /// Whether the admin API is enabled.
    pub admin_enabled: bool,
    /// URL path prefix for admin endpoints.
    pub admin_path: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 18789,
            max_connections: 256,
            session_timeout: Duration::from_secs(3600),
            auth_tokens: Vec::new(),
            webhook_enabled: true,
            webhook_path: "/webhook".to_string(),
            admin_enabled: true,
            admin_path: "/admin".to_string(),
        }
    }
}

impl GatewayConfig {
    /// Validate an auth token against the configured allowed tokens.
    ///
    /// If no auth tokens are configured, all tokens are accepted (open mode).
    pub fn validate_token(&self, token: &str) -> bool {
        if self.auth_tokens.is_empty() {
            return true;
        }
        self.auth_tokens.iter().any(|t| t == token)
    }

    /// Returns the full bind address as `host:port`.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = GatewayConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 18789);
        assert_eq!(config.max_connections, 256);
        assert_eq!(config.session_timeout, Duration::from_secs(3600));
        assert!(config.auth_tokens.is_empty());
        assert!(config.webhook_enabled);
        assert_eq!(config.webhook_path, "/webhook");
        assert!(config.admin_enabled);
        assert_eq!(config.admin_path, "/admin");
    }

    #[test]
    fn validate_token_open_mode() {
        let config = GatewayConfig::default();
        assert!(config.validate_token("anything"));
        assert!(config.validate_token(""));
    }

    #[test]
    fn validate_token_with_configured_tokens() {
        let config = GatewayConfig {
            auth_tokens: vec!["secret-1".to_string(), "secret-2".to_string()],
            ..Default::default()
        };
        assert!(config.validate_token("secret-1"));
        assert!(config.validate_token("secret-2"));
        assert!(!config.validate_token("wrong-token"));
        assert!(!config.validate_token(""));
    }

    #[test]
    fn bind_address_format() {
        let config = GatewayConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:18789");
    }
}
