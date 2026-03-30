//! Inter-Process Communication for Agent-Viewer Architecture
//!
//! This module provides the IPC protocol for communication between the TUI viewer
//! and the Agent process. The core implementation lives in the `brainwires_agent_network`
//! framework crate. This CLI adapter re-exports those types and provides
//! convenience wrappers that inject `PlatformPaths::sessions_dir()`.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     IPC      ┌──────────────────────┐
//! │  TUI (viewer)   │◄────────────►│  Agent Process       │
//! │  - rendering    │   encrypted  │  - conversation      │
//! │  - input        │              │  - tokio runtime     │
//! │  - can detach   │              │  - MCP connections   │
//! └─────────────────┘              │  - all session state │
//!                                  └──────────────────────┘
//! ```
//!
//! # Security
//!
//! - Unix socket permissions (0600) restrict access to the owning user
//! - Session token authentication prevents unauthorized connections
//! - ChaCha20-Poly1305 encryption protects message confidentiality and integrity

use anyhow::Result;
use std::path::PathBuf;

use crate::utils::paths::PlatformPaths;

// ── Private imports from bridge ──────────────────────────────────────────

use brainwires::agent_network::ipc::{
    protocol::AgentMetadata,
    socket::{self as bridge_socket},
    discovery as bridge_discovery,
    IpcConnection,
};

// ============================================================================
// CLI Convenience Wrappers (inject PlatformPaths::sessions_dir())
// ============================================================================

/// Connect to an agent by session ID
///
/// CLI convenience wrapper that resolves the socket path via PlatformPaths.
pub async fn connect_to_agent(session_id: &str) -> Result<IpcConnection> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    IpcConnection::connect_to_agent(&sessions_dir, session_id).await
}

/// Get the socket path for an agent session
pub fn get_agent_socket_path(session_id: &str) -> Result<PathBuf> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    Ok(bridge_socket::get_agent_socket_path(&sessions_dir, session_id))
}

/// Get the token file path for an agent session
pub fn get_session_token_path(session_id: &str) -> Result<PathBuf> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    Ok(bridge_socket::get_session_token_path(&sessions_dir, session_id))
}

/// Write session token to disk with secure permissions (0600)
pub fn write_session_token(session_id: &str, token: &str) -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_socket::write_session_token(&sessions_dir, session_id, token)
}

/// Read session token from disk
pub fn read_session_token(session_id: &str) -> Result<Option<String>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_socket::read_session_token(&sessions_dir, session_id)
}

/// Delete session token file
pub fn delete_session_token(session_id: &str) -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_socket::delete_session_token(&sessions_dir, session_id)
}

/// Validate that a provided token matches the stored token for a session
pub fn validate_session_token(session_id: &str, provided_token: &str) -> bool {
    match PlatformPaths::sessions_dir() {
        Ok(sessions_dir) => {
            bridge_socket::validate_session_token(&sessions_dir, session_id, provided_token)
        }
        Err(e) => {
            tracing::error!("Failed to get sessions dir for token validation: {}", e);
            false
        }
    }
}

/// List all available agent sessions
pub fn list_agent_sessions() -> Result<Vec<String>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::list_agent_sessions(&sessions_dir)
}

/// Check if an agent session exists and is alive
pub async fn is_agent_alive(session_id: &str) -> bool {
    match PlatformPaths::sessions_dir() {
        Ok(sessions_dir) => bridge_discovery::is_agent_alive(&sessions_dir, session_id).await,
        Err(_) => false,
    }
}

/// Clean up stale socket files
pub async fn cleanup_stale_sockets() -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::cleanup_stale_sockets(&sessions_dir).await
}

/// Clean up all files for a specific session
pub fn cleanup_session(session_id: &str) -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::cleanup_session(&sessions_dir, session_id)
}

/// Get the metadata file path for an agent session
pub fn get_agent_metadata_path(session_id: &str) -> Result<PathBuf> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    Ok(bridge_discovery::get_agent_metadata_path(&sessions_dir, session_id))
}

/// Write agent metadata to disk
pub fn write_agent_metadata(metadata: &AgentMetadata) -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::write_agent_metadata(&sessions_dir, metadata)
}

/// Read agent metadata from disk
pub fn read_agent_metadata(session_id: &str) -> Result<Option<AgentMetadata>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::read_agent_metadata(&sessions_dir, session_id)
}

/// Update agent metadata (read-modify-write pattern)
pub fn update_agent_metadata<F>(session_id: &str, updater: F) -> Result<()>
where
    F: FnOnce(&mut AgentMetadata),
{
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::update_agent_metadata(&sessions_dir, session_id, updater)
}

/// Delete agent metadata file
pub fn delete_agent_metadata(session_id: &str) -> Result<()> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::delete_agent_metadata(&sessions_dir, session_id)
}

/// List all agent sessions with their metadata
pub fn list_agent_sessions_with_metadata() -> Result<Vec<AgentMetadata>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::list_agent_sessions_with_metadata(&sessions_dir)
}

/// Get children of a given agent
pub fn get_child_agents(parent_session_id: &str) -> Result<Vec<AgentMetadata>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::get_child_agents(&sessions_dir, parent_session_id)
}

/// Get the root agents (those without a parent)
pub fn get_root_agents() -> Result<Vec<AgentMetadata>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::get_root_agents(&sessions_dir)
}

/// Get the depth of an agent in the tree (root = 0)
pub fn get_agent_depth(session_id: &str) -> Result<u32> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::get_agent_depth(&sessions_dir, session_id)
}

/// Build a tree structure of agents for display
pub fn format_agent_tree(current_session_id: Option<&str>) -> Result<String> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    bridge_discovery::format_agent_tree(&sessions_dir, current_session_id)
}
