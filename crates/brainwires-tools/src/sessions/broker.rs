//! Types and trait for the host-provided session registry.
//!
//! `brainwires-tools` is a framework crate — it does not know about the
//! gateway's per-user session map or the concrete [`ChatAgent`] machinery.
//! The [`SessionBroker`] trait bridges that gap: the host (e.g. the BrainClaw
//! gateway) implements it against its real registry and hands an
//! `Arc<dyn SessionBroker>` to [`crate::sessions::SessionsTool`].

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Opaque identifier for a chat session.
///
/// Callers should treat this as an arbitrary string token; the gateway
/// picks the actual format (UUID, `platform:user`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    /// Construct a [`SessionId`] from anything convertible to `String`.
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Summary metadata for a single session, returned by `sessions_list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// The session identifier.
    pub id: SessionId,
    /// Originating channel (e.g. `"discord"`, `"web"`, `"internal"`).
    pub channel: String,
    /// Peer handle — user id on the channel, or `"spawned-by-<parent>"`.
    pub peer: String,
    /// When the session was first created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session last received or produced a message.
    pub last_active: chrono::DateTime<chrono::Utc>,
    /// Number of messages currently in the session's transcript.
    pub message_count: usize,
    /// Parent session that spawned this one, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<SessionId>,
}

/// A single message from a session's transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// `"user"` | `"assistant"` | `"system"` | `"tool"`.
    pub role: String,
    /// Message text. Tool calls/results are stringified.
    pub content: String,
    /// When the message was recorded (may be approximate if the underlying
    /// agent does not track per-message timestamps).
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Parameters for [`SessionBroker::spawn`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Initial user message to seed the new session with.
    pub prompt: String,
    /// Optional provider/model override. `None` = inherit from parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional system prompt override. `None` = inherit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Tools to allow in the spawned session. `None` = inherit parent's toolset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// If `true`, block until the spawned session produces its first
    /// assistant message (or [`Self::wait_timeout_secs`] elapses) and return
    /// that in the tool result. Default: `false` — return immediately with
    /// just the new session id.
    #[serde(default)]
    pub wait_for_first_reply: bool,
    /// Seconds to wait when [`Self::wait_for_first_reply`] is `true`.
    /// Default: `60`.
    #[serde(default = "default_wait_timeout_secs")]
    pub wait_timeout_secs: u64,
}

fn default_wait_timeout_secs() -> u64 {
    60
}

impl Default for SpawnRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: None,
            system: None,
            tools: None,
            wait_for_first_reply: false,
            wait_timeout_secs: default_wait_timeout_secs(),
        }
    }
}

/// Result of [`SessionBroker::spawn`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedSession {
    /// The id of the newly-created session.
    pub id: SessionId,
    /// Set iff `wait_for_first_reply` was `true` and the first assistant
    /// message arrived within the timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_reply: Option<SessionMessage>,
}

/// Host-provided bridge from session tools to the real session registry.
///
/// Implementations must be cheap to clone-via-`Arc` and safe to call from
/// the tool executor's async context.
#[async_trait]
pub trait SessionBroker: Send + Sync {
    /// List every live session the host knows about.
    async fn list(&self) -> anyhow::Result<Vec<SessionSummary>>;

    /// Read a session's transcript, newest-last, capped at `limit` entries
    /// (`None` = use the host's sensible default).
    async fn history(
        &self,
        id: &SessionId,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<SessionMessage>>;

    /// Inject a user-role message into `id`'s inbound queue. Fire-and-forget
    /// — the target session processes it asynchronously.
    async fn send(&self, id: &SessionId, text: String) -> anyhow::Result<()>;

    /// Create a new session as a child of `parent`, seeded with `req.prompt`.
    async fn spawn(&self, parent: &SessionId, req: SpawnRequest) -> anyhow::Result<SpawnedSession>;
}
