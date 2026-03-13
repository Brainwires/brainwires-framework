#![deny(missing_docs)]
//! # Brainwires Agent Network
//!
//! Agent networking layer for the Brainwires Agent Framework.
//!
//! Provides an MCP server framework, middleware pipeline, agent IPC,
//! remote bridge, and optional mesh networking support.
//!
//! ## Protocol-Layer Stack
//!
//! The networking layer is organized as a 5-layer protocol stack:
//!
//! 1. **Identity** — agent identity, capability advertisement, credentials
//! 2. **Transport** — how bytes move (IPC, Remote, TCP, A2A, Pub/Sub)
//! 3. **Routing** — where messages go (direct, topology, broadcast, content)
//! 4. **Discovery** — how agents find each other (mDNS, registry, manual)
//! 5. **Application** — user-facing API (NetworkManager, events)

// ============================================================================
// MCP Server Framework
// ============================================================================
/// WebSocket/HTTP connection types.
pub mod connection;
/// Error types for the agent network crate.
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
/// Networking transport layer — pluggable transports for agent communication.
pub mod transport;

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

// ============================================================================
// Protocol-Layer Stack (Identity, Network Core)
// ============================================================================
/// Peer discovery — how agents find each other on the network.
pub mod discovery;
/// Agent identity, capability advertisement, and credentials.
pub mod identity;
/// Core network types: message envelopes, events, and errors.
pub mod network;
/// Message routing — direct, broadcast, and content-based routing.
pub mod routing;

pub use identity::{AgentCard, AgentIdentity, ProtocolId};
pub use network::{
    ConnectionState, MessageEnvelope, MessageTarget, NetworkError, NetworkEvent, NetworkManager,
    NetworkManagerBuilder, Payload, TransportType,
};
pub use transport::{Transport, TransportAddress};

#[cfg(feature = "ipc-transport")]
pub use transport::IpcTransport;
#[cfg(feature = "pubsub-transport")]
pub use transport::PubSubTransport;
#[cfg(feature = "remote-transport")]
pub use transport::RemoteTransport;
#[cfg(feature = "tcp-transport")]
pub use transport::TcpTransport;
#[cfg(feature = "a2a-transport")]
pub use transport::{A2aTransport, a2a_message_to_envelope};
