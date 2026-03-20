//! JSON-RPC 2.0 envelope types and method constants.

use serde::{Deserialize, Serialize};

use crate::error::A2aError;

// ---------------------------------------------------------------------------
// Method constants
// ---------------------------------------------------------------------------

/// Send a message to an agent.
pub const METHOD_MESSAGE_SEND: &str = "SendMessage";
/// Stream a message to an agent.
pub const METHOD_MESSAGE_STREAM: &str = "SendStreamingMessage";
/// Get a task by ID.
pub const METHOD_TASKS_GET: &str = "GetTask";
/// Cancel a task.
pub const METHOD_TASKS_CANCEL: &str = "CancelTask";
/// Resubscribe to task updates.
pub const METHOD_TASKS_RESUBSCRIBE: &str = "SubscribeToTask";
/// List tasks.
pub const METHOD_TASKS_LIST: &str = "ListTasks";
/// Set push notification configuration.
pub const METHOD_PUSH_CONFIG_SET: &str = "CreateTaskPushNotificationConfig";
/// Get push notification configuration.
pub const METHOD_PUSH_CONFIG_GET: &str = "GetTaskPushNotificationConfig";
/// List push notification configurations.
pub const METHOD_PUSH_CONFIG_LIST: &str = "ListTaskPushNotificationConfigs";
/// Delete push notification configuration.
pub const METHOD_PUSH_CONFIG_DELETE: &str = "DeleteTaskPushNotificationConfig";
/// Get authenticated extended agent card.
pub const METHOD_EXTENDED_CARD: &str = "GetExtendedAgentCard";

// ---------------------------------------------------------------------------
// Request ID
// ---------------------------------------------------------------------------

/// JSON-RPC request identifier (string or number).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String identifier.
    String(String),
    /// Numeric identifier.
    Number(i64),
}

// ---------------------------------------------------------------------------
// JSON-RPC envelope types
// ---------------------------------------------------------------------------

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (always "2.0").
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Request identifier.
    pub id: RequestId,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version (always "2.0").
    pub jsonrpc: String,
    /// Result on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<A2aError>,
    /// Request identifier echoed back.
    pub id: RequestId,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    pub fn error(id: RequestId, error: A2aError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}
