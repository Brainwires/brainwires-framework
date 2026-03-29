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
