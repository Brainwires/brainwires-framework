// ============================================================================
// MCP Server Framework
// ============================================================================
pub mod connection;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod registry;
pub mod server;
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
pub mod auth;
pub mod ipc;
pub mod remote;
pub mod traits;

// ============================================================================
// Agent Management (tool registry + lifecycle trait)
// ============================================================================
pub mod agent_manager;
pub mod agent_tools;

pub use agent_manager::{AgentInfo, AgentManager, AgentResult, SpawnConfig};
pub use agent_tools::AgentToolRegistry;

// ============================================================================
// Relay Client (merged from brainwires-bridge-client)
// ============================================================================
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use client::{RelayClient, RelayClientError, AgentConfig};
