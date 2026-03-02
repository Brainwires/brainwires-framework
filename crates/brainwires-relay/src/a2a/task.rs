//! Task lifecycle types for the A2A protocol.
//!
//! A [`Task`] represents a unit of work sent to an agent. It progresses through
//! a set of well-defined [`TaskState`]s and accumulates [`Message`]s and
//! [`Artifact`]s as execution proceeds.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::{Artifact, Message};

/// The lifecycle state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    /// The task has been received but not yet started.
    Submitted,

    /// The agent is actively working on the task.
    Working,

    /// The agent needs additional input from the user/caller before proceeding.
    InputRequired,

    /// The task has completed successfully.
    Completed,

    /// The task has failed.
    Failed,

    /// The task was canceled by the caller.
    Canceled,
}

/// A task being executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Unique identifier for this task.
    pub id: String,

    /// Session identifier for grouping related tasks into a conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Current lifecycle state of the task.
    pub state: TaskState,

    /// Messages exchanged during task execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,

    /// Artifacts produced by the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,

    /// Optional metadata associated with the task.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,

    /// State transition history (when `stateTransitionHistory` capability is enabled).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<TaskState>,
}

/// Parameters for sending a message to create or continue a task.
///
/// Corresponds to the `tasks/send` JSON-RPC method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSendParams {
    /// Task ID. If this ID already exists the message is appended to the
    /// existing task; otherwise a new task is created.
    pub id: String,

    /// Optional session ID for grouping tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// The message to send to the agent.
    pub message: Message,

    /// Maximum number of history entries to return in the response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<u32>,

    /// Optional metadata to attach to the task.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Push notification configuration for receiving task updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_notification: Option<PushNotificationConfig>,
}

/// Parameters for querying the status of an existing task.
///
/// Corresponds to the `tasks/get` JSON-RPC method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskQueryParams {
    /// The ID of the task to query.
    pub id: String,

    /// Maximum number of history entries to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<u32>,
}

/// Configuration for receiving push notifications about task updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationConfig {
    /// The URL to send push notifications to.
    pub url: String,

    /// Optional authentication token for the push notification endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}
