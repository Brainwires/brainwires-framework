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
