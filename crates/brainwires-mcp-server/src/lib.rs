#![deny(missing_docs)]
//! # Brainwires MCP Server
//!
//! MCP server framework with middleware pipeline for the Brainwires Agent Framework.
//!
//! Provides everything needed to build an MCP-compliant tool server:
//! - [`McpServer`] — async event loop that reads JSON-RPC, runs middleware, dispatches to handler
//! - [`McpHandler`] — trait defining how your server responds to initialize/list_tools/call_tool
//! - [`McpToolRegistry`] — stores tool definitions + handlers, dispatches tool calls
//! - [`MiddlewareChain`] — ordered middleware pipeline (auth, logging, rate-limiting, tool filtering)
//! - [`ServerTransport`] — pluggable transport (stdio included)

/// WebSocket/HTTP connection types.
pub mod connection;
/// Error types for the MCP server.
pub mod error;
/// MCP request handler trait.
pub mod handler;
/// MCP server transport (stdio).
pub mod mcp_transport;
/// Middleware pipeline (auth, logging, rate-limiting, tool filtering).
pub mod middleware;
/// MCP tool registry.
pub mod registry;
/// MCP server lifecycle.
pub mod server;

pub use connection::{ClientInfo, RequestContext};
pub use error::AgentNetworkError;
pub use handler::McpHandler;
pub use mcp_transport::{ServerTransport, StdioServerTransport};
pub use middleware::{Middleware, MiddlewareChain, MiddlewareResult};
pub use registry::{McpToolDef, McpToolRegistry, ToolHandler};
pub use server::McpServer;

// Re-export middleware implementations
pub use middleware::auth::AuthMiddleware;
pub use middleware::logging::LoggingMiddleware;
pub use middleware::rate_limit::RateLimitMiddleware;
pub use middleware::tool_filter::ToolFilterMiddleware;
