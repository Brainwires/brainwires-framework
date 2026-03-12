#![deny(missing_docs)]
//! # Brainwires Agent Network
//!
//! Agent networking layer for the Brainwires Agent Framework.
//!
//! Provides an MCP server framework, middleware pipeline, agent IPC,
//! remote bridge, and optional mesh networking support.

// ============================================================================
// MCP Server Framework
// ============================================================================
/// WebSocket/HTTP connection types.
pub mod connection;
/// Error types for the agent network crate.
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
pub use error::AgentNetworkError;
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
/// Authentication for agent network connections.
pub mod auth;
/// IPC (inter-process communication) socket protocol.
pub mod ipc;
/// Remote bridge and realtime protocol.
pub mod remote;
/// Common agent network traits.
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
// Client
// ============================================================================
/// Client for connecting to a remote agent network server.
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use client::{AgentConfig, AgentNetworkClient, AgentNetworkClientError};

// ============================================================================
// Mesh Networking (topology, routing, discovery, federation)
// ============================================================================
/// Distributed agent mesh networking — topology, routing, discovery, federation.
#[cfg(feature = "mesh")]
pub mod mesh;
