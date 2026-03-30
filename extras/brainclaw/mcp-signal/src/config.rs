//! Configuration for the Signal channel adapter.
//!
//! BrainClaw connects to Signal via the `signal-cli-rest-api` daemon
//! (see <https://github.com/bbernhard/signal-cli-rest-api>).  Start it with:
//!
//! ```text
//! signal-cli -a +1234567890 daemon --http 127.0.0.1:8080
//! ```
//!
//! or via the Docker image `bbernhard/signal-cli-rest-api`.

use serde::{Deserialize, Serialize};

/// Configuration for the Signal channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Base URL of the signal-cli REST API daemon (e.g. "http://127.0.0.1:8080").
    pub api_url: String,
    /// The bot's own Signal phone number in E.164 format (e.g. "+14155552671").
    pub phone_number: String,
    /// WebSocket URL of the brainwires-gateway.
    pub gateway_url: String,
    /// Optional auth token for the gateway handshake.
    pub gateway_token: Option<String>,
    /// In group chats, only respond when @mentioned by name.
    /// Direct messages always respond.
    #[serde(default)]
    pub group_mention_required: bool,
    /// The bot's display name used for @mention detection in group messages.
    #[serde(default)]
    pub bot_name: Option<String>,
    /// Additional keyword patterns (case-insensitive) that trigger a response
    /// in group messages even without an @mention.
    #[serde(default)]
    pub mention_patterns: Vec<String>,
    /// Allowed sender phone numbers. Empty = accept all senders.
    #[serde(default)]
    pub sender_allowlist: Vec<String>,
    /// Allowed group IDs (base64). Empty = accept all groups.
    #[serde(default)]
    pub group_allowlist: Vec<String>,
    /// Polling interval in milliseconds when WebSocket is not available.
    /// Default: 2000 ms.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

fn default_poll_interval_ms() -> u64 {
    2000
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:8080".to_string(),
            phone_number: String::new(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            gateway_token: None,
            group_mention_required: false,
            bot_name: None,
            mention_patterns: Vec::new(),
            sender_allowlist: Vec::new(),
            group_allowlist: Vec::new(),
            poll_interval_ms: 2000,
        }
    }
}
