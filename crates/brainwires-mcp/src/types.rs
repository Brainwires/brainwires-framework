//! MCP Protocol Types
//!
//! This module provides type definitions for the Model Context Protocol.
//! It now uses the official `rmcp` crate with compatibility aliases for
//! backward compatibility during migration.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export rmcp types with compatibility aliases (native only)
#[cfg(feature = "native")]
pub use rmcp::model::{
    Tool as RmcpTool,
    Resource as RmcpResource,
    Prompt as RmcpPrompt,
    CallToolRequestParam,
    CallToolResult,
    Content,
    ProtocolVersion,
};

// Re-export capabilities (native only)
#[cfg(feature = "native")]
pub use rmcp::model::{
    ServerCapabilities as RmcpServerCapabilities,
    ClientCapabilities as RmcpClientCapabilities,
    ToolsCapability,
    ResourcesCapability,
    PromptsCapability,
};

// ===========================================================================
// BACKWARD COMPATIBILITY ALIASES (native only - require rmcp)
// ===========================================================================

#[cfg(feature = "native")]
/// Compatibility alias for Tool
pub type McpTool = RmcpTool;

#[cfg(feature = "native")]
/// Compatibility alias for Resource
pub type McpResource = RmcpResource;

#[cfg(feature = "native")]
/// Compatibility alias for Prompt
pub type McpPrompt = RmcpPrompt;

#[cfg(feature = "native")]
/// Compatibility alias for CallToolParams
pub type CallToolParams = CallToolRequestParam;

#[cfg(feature = "native")]
/// Compatibility alias for ServerCapabilities
pub type ServerCapabilities = RmcpServerCapabilities;

#[cfg(feature = "native")]
/// Compatibility alias for ClientCapabilities
pub type ClientCapabilities = RmcpClientCapabilities;

// ===========================================================================
// ADDITIONAL TYPES NOT DIRECTLY PROVIDED BY RMCP
// ===========================================================================
// These types are still custom as they handle JSON-RPC layer or are
// specific to our implementation

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    /// Returns an error if params cannot be serialized to JSON.
    pub fn new<T: Serialize>(
        id: impl Into<Value>,
        method: String,
        params: Option<T>,
    ) -> Result<Self, serde_json::Error> {
        let params_value = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };
        Ok(Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method,
            params: params_value,
        })
    }

    /// Create a new JSON-RPC request, panicking if serialization fails.
    /// Use this only when you're certain serialization cannot fail.
    pub fn new_unchecked<T: Serialize>(
        id: impl Into<Value>,
        method: String,
        params: Option<T>,
    ) -> Self {
        Self::new(id, method, params).expect("Failed to serialize JSON-RPC request params")
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification (no id field).
    /// Returns an error if params cannot be serialized to JSON.
    pub fn new<T: Serialize>(
        method: impl Into<String>,
        params: Option<T>,
    ) -> Result<Self, serde_json::Error> {
        let params_value = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };
        Ok(Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: params_value,
        })
    }

    /// Create a new JSON-RPC notification, panicking if serialization fails.
    /// Use this only when you're certain serialization cannot fail.
    pub fn new_unchecked<T: Serialize>(method: impl Into<String>, params: Option<T>) -> Self {
        Self::new(method, params).expect("Failed to serialize JSON-RPC notification params")
    }
}

/// Generic JSON-RPC message that could be a response or notification
/// Used for bidirectional MCP communication where servers can send notifications
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    /// A response to a request (has id field)
    Response(JsonRpcResponse),
    /// A server-initiated notification (no id field)
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Check if this is a response
    pub fn is_response(&self) -> bool {
        matches!(self, JsonRpcMessage::Response(_))
    }

    /// Check if this is a notification
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }

    /// Try to get the response if this is one
    pub fn as_response(self) -> Option<JsonRpcResponse> {
        match self {
            JsonRpcMessage::Response(r) => Some(r),
            _ => None,
        }
    }

    /// Try to get the notification if this is one
    pub fn as_notification(self) -> Option<JsonRpcNotification> {
        match self {
            JsonRpcMessage::Notification(n) => Some(n),
            _ => None,
        }
    }
}

// ===========================================================================
// MCP PROGRESS NOTIFICATION TYPES
// ===========================================================================

/// Progress notification parameters from MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    /// Token identifying which request this progress is for
    #[serde(rename = "progressToken")]
    pub progress_token: String,
    /// Current progress value
    pub progress: f64,
    /// Total expected value (for calculating percentage)
    pub total: Option<f64>,
    /// Human-readable progress message
    pub message: Option<String>,
}

/// Parsed MCP notification types
#[derive(Debug, Clone)]
pub enum McpNotification {
    /// Progress update for a long-running operation
    Progress(ProgressParams),
    /// Unknown/unhandled notification type
    Unknown { method: String, params: Option<Value> },
}

impl McpNotification {
    /// Parse a JsonRpcNotification into a typed McpNotification
    pub fn from_notification(notif: &JsonRpcNotification) -> Self {
        match notif.method.as_str() {
            "notifications/progress" => {
                if let Some(ref params) = notif.params {
                    if let Ok(progress) = serde_json::from_value::<ProgressParams>(params.clone()) {
                        return McpNotification::Progress(progress);
                    }
                }
                McpNotification::Unknown {
                    method: notif.method.clone(),
                    params: notif.params.clone(),
                }
            }
            _ => McpNotification::Unknown {
                method: notif.method.clone(),
                params: notif.params.clone(),
            },
        }
    }
}

// ===========================================================================
// MCP INITIALIZATION TYPES
// ===========================================================================

// ===========================================================================
// MCP TYPES (require rmcp - native only)
// ===========================================================================

#[cfg(feature = "native")]
/// MCP Initialize Request Parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[cfg(feature = "native")]
/// MCP Initialize Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[cfg(feature = "native")]
/// Tools List Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<McpTool>,
}

#[cfg(feature = "native")]
/// Resources List Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<McpResource>,
}

#[cfg(feature = "native")]
/// Prompts List Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<McpPrompt>,
}

#[cfg(feature = "native")]
/// Resource Read Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

#[cfg(feature = "native")]
/// Resource Read Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContent>,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceContent {
    Text { uri: String, mime_type: Option<String>, text: String },
    Blob { uri: String, mime_type: Option<String>, blob: String },
}

#[cfg(feature = "native")]
/// Prompt Get Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

#[cfg(feature = "native")]
/// Prompt Get Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    pub description: String,
    pub messages: Vec<PromptMessage>,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: PromptContent,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PromptContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResource },
}

#[cfg(feature = "native")]
/// Prompt Argument Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[cfg(feature = "native")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolResultContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResource },
}

// ===========================================================================
// TESTS
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_rpc_request_new() {
        let request = JsonRpcRequest::new(1, "test_method".to_string(), Some(json!({"key": "value"}))).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, json!(1));
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest::new(1, "test".to_string(), None::<()>).unwrap();
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("jsonrpc"));
        assert!(json.contains("2.0"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
        };

        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_type_aliases_work() {
        // Test that our type aliases are properly set up
        let _tool: McpTool;
        let _resource: McpResource;
        let _prompt: McpPrompt;
        // If this compiles, the aliases are working
    }
}
