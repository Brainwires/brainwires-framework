#![warn(missing_docs)]
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

/// MCP protocol types and JSON-RPC types.
pub mod types;
/// Stdio-based transport layer for MCP communication.
#[cfg(feature = "native")]
pub mod transport;
/// MCP client for connecting to external servers.
#[cfg(feature = "native")]
pub mod client;
/// MCP server configuration management.
pub mod config;

// Re-exports - native-only modules
#[cfg(feature = "native")]
pub use client::McpClient;
#[cfg(feature = "native")]
pub use transport::{StdioTransport, Transport};

// Re-exports - always available
pub use config::McpServerConfig;
#[cfg(feature = "native")]
pub use config::McpConfigManager;

// JSON-RPC types (always available)
pub use types::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError, JsonRpcNotification, JsonRpcMessage,
    ProgressParams, McpNotification,
};

// MCP types (require rmcp, native only)
#[cfg(feature = "native")]
pub use types::{
    McpTool, McpResource, McpPrompt, CallToolParams, CallToolResult,
    Content, ToolsCapability, ResourcesCapability, PromptsCapability,
    ServerCapabilities, ClientCapabilities, ServerInfo, ClientInfo,
    InitializeParams, InitializeResult,
    ListToolsResult, ListResourcesResult, ListPromptsResult,
    ReadResourceParams, ReadResourceResult, ResourceContent,
    GetPromptParams, GetPromptResult, PromptMessage, PromptContent, PromptArgument,
    ToolResultContent,
};

/// Prelude module for convenient imports
pub mod prelude {
    #[cfg(feature = "native")]
    pub use super::client::McpClient;
    #[cfg(feature = "native")]
    pub use super::config::McpConfigManager;
    pub use super::config::McpServerConfig;
    #[cfg(feature = "native")]
    pub use super::transport::{StdioTransport, Transport};
    pub use super::types::{
        JsonRpcRequest, JsonRpcResponse, JsonRpcNotification, JsonRpcMessage,
    };
    #[cfg(feature = "native")]
    pub use super::types::{
        McpTool, McpResource, McpPrompt, CallToolResult,
        ServerCapabilities, ClientCapabilities,
    };
}
