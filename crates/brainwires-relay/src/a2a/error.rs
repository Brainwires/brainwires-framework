//! Error types for the A2A protocol implementation.

use thiserror::Error;

/// Errors that can occur during A2A protocol operations.
#[derive(Debug, Error)]
pub enum A2aError {
    /// The requested task was not found.
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// The task is in an invalid state for the requested operation.
    #[error("invalid task state: {0}")]
    InvalidState(String),

    /// Authentication or authorization failure.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Transport-level error (HTTP, SSE, connection failure, etc.).
    #[error("transport error: {0}")]
    Transport(String),

    /// Internal server or agent error.
    #[error("internal error: {0}")]
    Internal(String),

    /// Protocol-level error (malformed JSON-RPC, unknown method, etc.).
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Serialization or deserialization failure.
    #[error("serialization error: {source}")]
    Serialization {
        /// The underlying serialization error.
        #[from]
        source: serde_json::Error,
    },
}
