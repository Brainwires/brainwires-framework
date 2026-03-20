//! Core A2A message types: Message, Part, Artifact, Role.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sender role in A2A communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Client-to-server message.
    #[serde(rename = "ROLE_USER")]
    User,
    /// Server-to-client message.
    #[serde(rename = "ROLE_AGENT")]
    Agent,
    /// Unspecified role.
    #[serde(rename = "ROLE_UNSPECIFIED")]
    Unspecified,
}

/// A single unit of communication content.
///
/// Exactly one of `text`, `raw`, `url`, or `data` must be set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Part {
    /// Text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded raw bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// URL reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Structured JSON data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// MIME type of the content.
    #[serde(skip_serializing_if = "Option::is_none", rename = "mediaType")]
    pub media_type: Option<String>,
    /// File name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// A single communication message between client and server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Unique message identifier.
    #[serde(rename = "messageId")]
    pub message_id: String,
    /// Sender role.
    pub role: Role,
    /// Content parts.
    pub parts: Vec<Part>,
    /// Context identifier (conversation/session).
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Associated task identifier.
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Referenced task identifiers for additional context.
    #[serde(rename = "referenceTaskIds", skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<String>>,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Extension URIs present in this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

impl Message {
    /// Create a new user message with text content.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            parts: vec![Part {
                text: Some(text.into()),
                raw: None,
                url: None,
                data: None,
                media_type: None,
                filename: None,
                metadata: None,
            }],
            context_id: None,
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        }
    }

    /// Create a new agent message with text content.
    pub fn agent_text(text: impl Into<String>) -> Self {
        let mut msg = Self::user_text(text);
        msg.role = Role::Agent;
        msg
    }
}

/// Task output artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artifact {
    /// Unique artifact identifier (unique within a task).
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    /// Human-readable name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Artifact content parts.
    pub parts: Vec<Part>,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Extension URIs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}
