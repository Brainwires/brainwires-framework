//! Core A2A message types: Message, Part, Artifact, FileContent, Role.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sender role in A2A communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Client-to-server message.
    User,
    /// Server-to-client message.
    Agent,
}

/// A single unit of communication content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Part {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text value.
        text: String,
        /// Optional metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// File content (inline bytes or URI reference).
    #[serde(rename = "file")]
    File {
        /// The file content.
        file: FileContent,
        /// Optional metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Structured data content.
    #[serde(rename = "data")]
    Data {
        /// Arbitrary JSON data.
        data: serde_json::Value,
        /// Optional metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

/// File content — either inline bytes or a URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    /// Inline base64-encoded bytes.
    Bytes {
        /// Base64-encoded file bytes.
        bytes: String,
        /// MIME type of the file.
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        /// File name.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// URI-referenced file.
    Uri {
        /// URI pointing to the file.
        uri: String,
        /// MIME type of the file.
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        /// File name.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

/// A single communication message between client and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Discriminator field.
    #[serde(default = "kind_message")]
    pub kind: String,
}

fn kind_message() -> String {
    "message".to_string()
}

impl Message {
    /// Create a new user message with text content.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            parts: vec![Part::Text {
                text: text.into(),
                metadata: None,
            }],
            context_id: None,
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
            kind: "message".to_string(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
