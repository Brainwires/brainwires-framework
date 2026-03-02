//! Transport layer for A2A communication.
//!
//! The A2A protocol uses **HTTP + Server-Sent Events (SSE)** as its transport
//! mechanism, with messages wrapped in **JSON-RPC 2.0** envelopes.
//!
//! ## JSON-RPC Methods
//!
//! | Method                         | Description                                 |
//! |--------------------------------|---------------------------------------------|
//! | `tasks/send`                   | Send a message to create or continue a task |
//! | `tasks/sendSubscribe`          | Send + subscribe to streaming updates (SSE) |
//! | `tasks/get`                    | Query task status and history               |
//! | `tasks/cancel`                 | Cancel a running task                       |
//! | `tasks/pushNotification/set`   | Configure push notifications for a task     |
//! | `tasks/pushNotification/get`   | Get push notification config for a task     |
//! | `tasks/resubscribe`            | Re-subscribe to SSE for an existing task    |
//!
//! ## Status
//!
//! This module is a **scaffold**. Full HTTP/SSE transport and JSON-RPC envelope
//! handling will be implemented in a future iteration.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be `"2.0"`.
    pub jsonrpc: String,

    /// The JSON-RPC method name (e.g. `"tasks/send"`).
    pub method: String,

    /// Method parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    /// Request identifier for correlating responses.
    pub id: serde_json::Value,
}

/// A JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Must be `"2.0"`.
    pub jsonrpc: String,

    /// The result payload (present on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// The error payload (present on failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request identifier echoed back from the request.
    pub id: serde_json::Value,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i64,

    /// Short human-readable error message.
    pub message: String,

    /// Optional additional error data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
