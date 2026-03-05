#![warn(missing_docs)]
//! # Brainwires Relay -- MCP Server Framework & Agent Communication
//!
//! Provides an MCP server framework, middleware pipeline, agent IPC,
//! remote relay bridging, and optional A2A protocol support.

// ============================================================================
// MCP Server Framework
// ============================================================================
/// WebSocket/HTTP connection types.
pub mod connection;
/// Error types for the relay crate.
pub mod error;
/// MCP request handler trait.
pub mod handler;
/// Middleware pipeline (auth, logging, rate-limiting, tool filtering).
pub mod middleware;
/// MCP tool registry.
pub mod registry;
/// MCP server lifecycle.
pub mod server;
/// Server transport (stdio).
pub mod transport;

pub use connection::{ClientInfo, RequestContext};
pub use error::RelayError;
pub use handler::McpHandler;
pub use middleware::{Middleware, MiddlewareChain, MiddlewareResult};
pub use registry::{McpToolDef, McpToolRegistry, ToolHandler};
pub use server::McpServer;
pub use transport::{ServerTransport, StdioServerTransport};

// Re-export middleware implementations
pub use middleware::auth::AuthMiddleware;
pub use middleware::logging::LoggingMiddleware;
pub use middleware::rate_limit::RateLimitMiddleware;
pub use middleware::tool_filter::ToolFilterMiddleware;

// ============================================================================
// Agent Communication Backbone (IPC, Auth, Remote)
// ============================================================================
/// Authentication for relay connections.
pub mod auth;
/// IPC (inter-process communication) socket protocol.
pub mod ipc;
/// Remote relay bridge and realtime protocol.
pub mod remote;
/// Common relay traits.
pub mod traits;

// ============================================================================
// Agent Management (tool registry + lifecycle trait)
// ============================================================================
/// Agent lifecycle management.
pub mod agent_manager;
/// Pre-built MCP tools for agent operations.
pub mod agent_tools;

pub use agent_manager::{AgentInfo, AgentManager, AgentResult, SpawnConfig};
pub use agent_tools::AgentToolRegistry;

// ============================================================================
// Relay Client (merged from brainwires-bridge-client)
// ============================================================================
/// Relay client for connecting to a remote relay server.
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use client::{RelayClient, RelayClientError, AgentConfig};

// ============================================================================
// A2A Protocol (merged from brainwires-a2a)
// ============================================================================
#[cfg(feature = "a2a")]
pub mod a2a;
