//! MCP Server Implementation
//!
//! This module implements an MCP (Model Context Protocol) server that exposes
//! the CLI's capabilities over stdin/stdout using JSON-RPC 2.0.
//!
//! Features:
//! - Exposes all local tools as MCP tools
//! - Task agent spawning and management
//! - Can act as MCP client to other servers (bi-directional)
//! - Hierarchical task breakdown support

mod handler;
mod agent_tools;

pub use handler::McpServerHandler;
pub use agent_tools::AgentToolRegistry;
