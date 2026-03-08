//! Streaming event types for A2A.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::task::{Task, TaskStatus};
use crate::types::{Artifact, Message};

/// Event notifying a change in task status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusUpdateEvent {
    /// Task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Context identifier.
    #[serde(rename = "contextId")]
    pub context_id: String,
    /// New task status.
    pub status: TaskStatus,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Event notifying an artifact update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifactUpdateEvent {
    /// Task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Context identifier.
    #[serde(rename = "contextId")]
    pub context_id: String,
    /// The artifact.
    pub artifact: Artifact,
    /// If true, append to previously sent artifact with same ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
    /// If true, this is the final chunk.
    #[serde(rename = "lastChunk", skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Union of all possible stream events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamEvent {
    /// Full task snapshot.
    Task(Task),
    /// Agent message.
    Message(Message),
    /// Task status change.
    StatusUpdate(TaskStatusUpdateEvent),
    /// Artifact update.
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

/// Response for `message/send` — either a Task or a Message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SendMessageResponse {
    /// A task was created or updated.
    Task(Task),
    /// A direct message response.
    Message(Message),
}
