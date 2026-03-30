use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A typed analytics event emitted anywhere in the framework.
///
/// All variants are self-contained (no imports from other brainwires crates)
/// and fully serializable. The `session_id` field, when present, groups related
/// events across multiple emitting components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AnalyticsEvent {
    /// A provider `chat()` call completed (success or failure).
    ProviderCall {
        session_id:        Option<String>,
        provider:          String,
        model:             String,
        prompt_tokens:     u32,
        completion_tokens: u32,
        duration_ms:       u64,
        cost_usd:          f64,
        success:           bool,
        timestamp:         DateTime<Utc>,
    },

    /// A TaskAgent run completed.
    AgentRun {
        session_id:              Option<String>,
        agent_id:                String,
        task_id:                 String,
        prompt_hash:             String,
        success:                 bool,
        total_iterations:        u32,
        total_tool_calls:        u32,
        tool_error_count:        u32,
        tools_used:              Vec<String>,
        total_prompt_tokens:     u32,
        total_completion_tokens: u32,
        total_cost_usd:          f64,
        duration_ms:             u64,
        failure_category:        Option<String>,
        timestamp:               DateTime<Utc>,
    },

    /// A single tool call within an agent run.
    ToolCall {
        session_id:  Option<String>,
        agent_id:    Option<String>,
        tool_name:   String,
        tool_use_id: String,
        is_error:    bool,
        duration_ms: Option<u64>,
        timestamp:   DateTime<Utc>,
    },

    /// An MCP server request was handled.
    McpRequest {
        session_id:  Option<String>,
        server_name: String,
        tool_name:   String,
        success:     bool,
        duration_ms: u64,
        timestamp:   DateTime<Utc>,
    },

    /// A channel message was sent or received (Discord, Telegram, Slack, etc.).
    ChannelMessage {
        session_id:   Option<String>,
        channel_type: String,
        direction:    String,
        message_len:  usize,
        timestamp:    DateTime<Utc>,
    },

    /// A storage operation completed.
    StorageOp {
        session_id:  Option<String>,
        store_type:  String,
        operation:   String,
        success:     bool,
        duration_ms: u64,
        timestamp:   DateTime<Utc>,
    },

    /// A network message was sent or received over the agent network.
    NetworkMessage {
        session_id: Option<String>,
        protocol:   String,
        direction:  String,
        bytes:      u64,
        success:    bool,
        timestamp:  DateTime<Utc>,
    },

    /// A dream consolidation cycle completed.
    DreamCycle {
        session_id:          Option<String>,
        sessions_processed:  usize,
        messages_summarized: usize,
        facts_extracted:     usize,
        tokens_before:       usize,
        tokens_after:        usize,
        duration_ms:         u64,
        timestamp:           DateTime<Utc>,
    },

    /// An autonomy session completed.
    AutonomySession {
        session_id:      Option<String>,
        tasks_attempted: u32,
        tasks_succeeded: u32,
        tasks_failed:    u32,
        total_cost_usd:  f64,
        duration_ms:     u64,
        timestamp:       DateTime<Utc>,
    },

    /// Escape hatch for user-defined events.
    Custom {
        session_id: Option<String>,
        name:       String,
        payload:    serde_json::Value,
        timestamp:  DateTime<Utc>,
    },
}

impl AnalyticsEvent {
    /// Returns the event's timestamp regardless of variant.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::ProviderCall  { timestamp, .. } => *timestamp,
            Self::AgentRun      { timestamp, .. } => *timestamp,
            Self::ToolCall      { timestamp, .. } => *timestamp,
            Self::McpRequest    { timestamp, .. } => *timestamp,
            Self::ChannelMessage{ timestamp, .. } => *timestamp,
            Self::StorageOp     { timestamp, .. } => *timestamp,
            Self::NetworkMessage{ timestamp, .. } => *timestamp,
            Self::DreamCycle    { timestamp, .. } => *timestamp,
            Self::AutonomySession{timestamp, .. } => *timestamp,
            Self::Custom        { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the session_id if present.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::ProviderCall  { session_id, .. } => session_id.as_deref(),
            Self::AgentRun      { session_id, .. } => session_id.as_deref(),
            Self::ToolCall      { session_id, .. } => session_id.as_deref(),
            Self::McpRequest    { session_id, .. } => session_id.as_deref(),
            Self::ChannelMessage{ session_id, .. } => session_id.as_deref(),
            Self::StorageOp     { session_id, .. } => session_id.as_deref(),
            Self::NetworkMessage{ session_id, .. } => session_id.as_deref(),
            Self::DreamCycle    { session_id, .. } => session_id.as_deref(),
            Self::AutonomySession{session_id, .. } => session_id.as_deref(),
            Self::Custom        { session_id, .. } => session_id.as_deref(),
        }
    }

    /// Returns the serde discriminant tag for this event (matches the SQLite `event_type` column).
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::ProviderCall   { .. } => "provider_call",
            Self::AgentRun       { .. } => "agent_run",
            Self::ToolCall       { .. } => "tool_call",
            Self::McpRequest     { .. } => "mcp_request",
            Self::ChannelMessage { .. } => "channel_message",
            Self::StorageOp      { .. } => "storage_op",
            Self::NetworkMessage { .. } => "network_message",
            Self::DreamCycle     { .. } => "dream_cycle",
            Self::AutonomySession{ .. } => "autonomy_session",
            Self::Custom         { .. } => "custom",
        }
    }
}
