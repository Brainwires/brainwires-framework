//! Task lifecycle types: Task, TaskStatus, TaskState.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Artifact, Message};

/// Possible lifecycle states of a Task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    /// Unknown or indeterminate state.
    Unknown,
    /// Task has been submitted and acknowledged.
    Submitted,
    /// Task is actively being processed.
    Working,
    /// Task finished successfully (terminal).
    Completed,
    /// Task finished with an error (terminal).
    Failed,
    /// Task was canceled (terminal).
    Canceled,
    /// Task was rejected by the agent (terminal).
    Rejected,
    /// Agent requires additional user input (interrupted).
    InputRequired,
    /// Authentication is required to proceed (interrupted).
    AuthRequired,
}

/// Current status of a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    /// Current state.
    pub state: TaskState,
    /// Optional message associated with the status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// ISO 8601 timestamp when the status was recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// The core unit of action in A2A.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier (UUID).
    pub id: String,
    /// Context identifier for the conversation/session.
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Current task status.
    pub status: TaskStatus,
    /// Output artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,
    /// History of interactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<Message>>,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Discriminator field.
    #[serde(default = "kind_task")]
    pub kind: String,
}

fn kind_task() -> String {
    "task".to_string()
}
