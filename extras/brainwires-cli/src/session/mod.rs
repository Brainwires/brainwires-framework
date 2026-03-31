//! Session Management Module
//!
//! Provides PTY-based session persistence similar to abduco/dtach.
//! The TUI runs inside a PTY managed by a session server.
//! Clients connect via Unix socket and I/O is proxied.
//!
//! Architecture:
//! ```text
//! ┌─────────────┐     Unix Socket     ┌──────────────────┐      PTY      ┌─────────────┐
//! │   Client    │◄──────────────────►│   Session Server  │◄─────────────►│  TUI App    │
//! │  (terminal) │    I/O relay        │  (background)     │               │  (running)  │
//! └─────────────┘                     └──────────────────┘               └─────────────┘
//! ```
//!
//! - Detach: Client disconnects, TUI keeps running in PTY
//! - Attach: Client reconnects, resumes where it left off

pub mod client;
pub mod server;

pub use client::SessionClient;
pub use server::SessionServer;

use anyhow::Result;
use std::path::PathBuf;

use crate::utils::paths::PlatformPaths;

/// Get the socket path for a session (PTY/terminal attach)
/// Note: This is separate from the agent IPC socket
pub fn get_session_socket_path(session_id: &str) -> Result<PathBuf> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    Ok(sessions_dir.join(format!("{}.pty.sock", session_id)))
}

/// List all active sessions (sessions with running agents)
///
/// This finds sessions where the agent is still running (IPC socket responsive).
/// The PTY server may be suspended (e.g., Ctrl+Z backgrounded), but as long as
/// the agent is alive, the session can be attached to (which will resume the PTY).
pub fn list_sessions() -> Result<Vec<String>> {
    let sessions_dir = PlatformPaths::sessions_dir()?;

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        // Look for .pty.sock files (PTY session sockets)
        if let Some(name) = path.file_name().and_then(|s| s.to_str())
            && name.ends_with(".pty.sock")
        {
            // Extract session ID by removing .pty.sock suffix
            let session_id = name.trim_end_matches(".pty.sock");
            // Check if the agent (IPC socket) is alive - not the PTY socket
            // The PTY server may be suspended but agent still running
            if is_agent_socket_alive(session_id, &sessions_dir) {
                sessions.push(session_id.to_string());
            }
        }
    }

    Ok(sessions)
}

/// Check if an agent's IPC socket is alive (synchronous version)
fn is_agent_socket_alive(session_id: &str, sessions_dir: &std::path::Path) -> bool {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let socket_path = sessions_dir.join(format!("{}.sock", session_id));

    if !socket_path.exists() {
        return false;
    }

    // Try to connect to the socket
    match UnixStream::connect(&socket_path) {
        Ok(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
            true
        }
        Err(_) => false,
    }
}

/// Check if a session socket is alive
fn is_session_alive(socket_path: &PathBuf) -> bool {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    match UnixStream::connect(socket_path) {
        Ok(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
            true
        }
        Err(_) => false,
    }
}

/// Generate a new session ID
pub fn generate_session_id() -> String {
    format!("session-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"))
}

/// Spawn a background session with TUI
///
/// Creates a new PTY session running the TUI in the background.
/// Returns the session ID and socket path.
pub async fn spawn_background_session(session_id: &str, model: &str) -> Result<PathBuf> {
    // Build TUI args for the session
    let tui_args = vec![
        "--session".to_string(),
        session_id.to_string(),
        "--pty-session".to_string(),
        "--model".to_string(),
        model.to_string(),
    ];

    // Spawn the session server with TUI
    let (_session_id, socket_path) = server::spawn_session(Some(session_id.to_string()), tui_args)?;

    // Wait a moment for the session to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify the session is running
    if is_session_alive(&socket_path) {
        Ok(socket_path)
    } else {
        anyhow::bail!("Session failed to start - socket not responding")
    }
}
