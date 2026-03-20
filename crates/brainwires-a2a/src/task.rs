//! Task lifecycle types: Task, TaskStatus, TaskState.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Artifact, Message};

/// Possible lifecycle states of a Task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Unspecified or indeterminate state.
    #[serde(rename = "TASK_STATE_UNSPECIFIED")]
    Unspecified,
    /// Task has been submitted and acknowledged.
    #[serde(rename = "TASK_STATE_SUBMITTED")]
    Submitted,
    /// Task is actively being processed.
    #[serde(rename = "TASK_STATE_WORKING")]
    Working,
    /// Task finished successfully (terminal).
    #[serde(rename = "TASK_STATE_COMPLETED")]
    Completed,
    /// Task finished with an error (terminal).
    #[serde(rename = "TASK_STATE_FAILED")]
    Failed,
    /// Task was canceled (terminal).
    #[serde(rename = "TASK_STATE_CANCELED")]
    Canceled,
    /// Task was rejected by the agent (terminal).
    #[serde(rename = "TASK_STATE_REJECTED")]
    Rejected,
    /// Agent requires additional user input (interrupted).
    #[serde(rename = "TASK_STATE_INPUT_REQUIRED")]
    InputRequired,
    /// Authentication is required to proceed (interrupted).
    #[serde(rename = "TASK_STATE_AUTH_REQUIRED")]
    AuthRequired,
}

/// Current status of a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}
