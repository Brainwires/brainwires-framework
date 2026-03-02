//! Core message and content types for the A2A protocol.
//!
//! These types map directly to the A2A specification's message model where agents
//! exchange [`Message`]s composed of [`Part`]s, and produce [`Artifact`]s as output.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A message exchanged between agents (or between a user and an agent).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The role of the message sender (`"user"` or `"agent"`).
    pub role: String,

    /// The content parts that make up this message.
    pub parts: Vec<Part>,

    /// Optional metadata associated with the message.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A single content part within a [`Message`] or [`Artifact`].
///
/// Parts carry the actual payload and are typed by their content kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Part {
    /// Plain text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },

    /// A file attachment (inline or by reference).
    #[serde(rename = "file")]
    File {
        /// The file name.
        name: String,

        /// MIME type of the file (e.g. `"application/pdf"`).
        mime_type: String,

        /// Base64-encoded file data, or a URI reference.
        data: String,
    },

    /// Structured data (JSON, XML, etc.) identified by MIME type.
    #[serde(rename = "data")]
    Data {
        /// MIME type of the data (e.g. `"application/json"`).
        mime_type: String,

        /// The serialized data payload.
        data: serde_json::Value,
    },
}

/// An artifact produced by an agent during task execution.
///
/// Artifacts represent the output/deliverables of a task and may be streamed
/// incrementally via chunked delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Human-readable name for this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description of what this artifact contains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The content parts that make up this artifact.
    pub parts: Vec<Part>,

    /// Zero-based index for ordering when multiple artifacts are produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,

    /// When `true`, the parts in this artifact should be appended to a
    /// previous artifact with the same `index` rather than replacing it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,

    /// When `true`, this is the final chunk for the artifact at `index`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,

    /// Optional metadata associated with the artifact.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
