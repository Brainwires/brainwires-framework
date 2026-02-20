//! Brainwires MCP - Model Context Protocol client and types
//!
//! This crate provides MCP client functionality for the Brainwires Agent Framework:
//!
//! - **McpClient**: Connect to external MCP servers, list/call tools, resources, prompts
//! - **Transport**: Stdio-based transport layer for MCP communication
//! - **Types**: JSON-RPC 2.0 types and MCP protocol types (with rmcp compatibility)
//! - **Config**: MCP server configuration management

// Re-export core types
pub use brainwires_core;

pub mod types;
pub mod transport;
pub mod client;
pub mod config;

// Re-exports
pub use client::McpClient;
pub use config::{McpConfigManager, McpServerConfig};
pub use transport::{StdioTransport, Transport};
pub use types::{
    // JSON-RPC types
    JsonRpcRequest, JsonRpcResponse, JsonRpcError, JsonRpcNotification, JsonRpcMessage,
    // MCP types
    McpTool, McpResource, McpPrompt, CallToolParams, CallToolResult,
    ServerCapabilities, ClientCapabilities, ServerInfo, ClientInfo,
    InitializeParams, InitializeResult,
    // List results
    ListToolsResult, ListResourcesResult, ListPromptsResult,
    // Resource operations
    ReadResourceParams, ReadResourceResult, ResourceContent,
    // Prompt operations
    GetPromptParams, GetPromptResult, PromptMessage, PromptContent, PromptArgument,
    // Notifications
    ProgressParams, McpNotification,
    // Tool content
    ToolResultContent,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::client::McpClient;
    pub use super::config::{McpConfigManager, McpServerConfig};
    pub use super::transport::{StdioTransport, Transport};
    pub use super::types::{
        JsonRpcRequest, JsonRpcResponse, JsonRpcNotification, JsonRpcMessage,
        McpTool, McpResource, McpPrompt, CallToolResult,
        ServerCapabilities, ClientCapabilities,
    };
}
