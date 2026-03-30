//! Attach to backgrounded TUI sessions
//!
//! This module provides commands to attach to backgrounded brainwires sessions,
//! list available sessions, and terminate backgrounded sessions.
//!
//! With the PTY-based session architecture:
//! - Sessions are TUI processes running inside a PTY managed by a session server
//! - Attaching connects as a PTY client, proxying terminal I/O to the server
//! - The TUI runs inside the PTY and the Agent handles AI/tools via IPC

use anyhow::{bail, Result};

use crate::ipc;
use crate::session;

/// Attach to a backgrounded session
///
/// This connects as a PTY client to the session server, which is running
/// the TUI inside a PTY. Terminal I/O is proxied between the client terminal
/// and the PTY.
#[cfg(unix)]
pub async fn attach(session: Option<String>) -> Result<()> {
    // Use session client for PTY-based attachment
    session::client::attach(session.as_deref())
}

/// Stub for non-Unix platforms
#[cfg(not(unix))]
pub async fn attach(_session: Option<String>) -> Result<()> {
    bail!("Attach is not supported on this platform")
}

/// Exit/terminate a backgrounded session
///
/// Sends a termination signal to the session.
/// If the session is stale (process dead but files remain), cleans up the files.
#[cfg(unix)]
pub async fn exit_session(session: Option<String>) -> Result<()> {
    let session_id = if let Some(id) = session {
        id
    } else {
        find_most_recent_pty_session()?
    };

    let pty_socket_path = session::get_session_socket_path(&session_id)?;

    if !pty_socket_path.exists() {
        // Session is stale - clean up the files
        println!("Session {} is stale (socket not found), cleaning up...", session_id);
        ipc::cleanup_session(&session_id)?;
        println!("Session cleaned up.");
        return Ok(());
    }

    println!("Terminating session: {}", session_id);

    // For PTY sessions, we need to connect to the agent IPC socket (not PTY socket)
    // and send an Exit command. The agent will clean up the PTY session.

    // Read session token for authenticated connection
    let session_token = match ipc::read_session_token(&session_id) {
        Ok(Some(token)) => token,
        Ok(None) => {
            println!("Warning: No session token found, cleaning up files only...");
            ipc::cleanup_session(&session_id)?;
            println!("Session cleaned up.");
            return Ok(());
        }
        Err(e) => {
            bail!("Failed to read session token: {}", e);
        }
    };

    // Connect to the agent and send Exit message
    match ipc::connect_to_agent(&session_id).await {
        Ok(mut conn) => {
            use brainwires::agent_network::ipc::{ViewerMessage, Handshake, HandshakeResponse};

            // Perform authenticated handshake
            let handshake = Handshake::reattach(session_id.clone(), session_token);
            if let Err(e) = conn.writer.write(&handshake).await {
                bail!("Failed to send handshake: {}", e);
            }

            // Wait for handshake response
            let response: HandshakeResponse = conn.reader.read().await?
                .ok_or_else(|| anyhow::anyhow!("Session closed during handshake"))?;

            if !response.accepted {
                bail!("Session rejected connection: {}",
                    response.error.unwrap_or_else(|| "Unknown error".to_string()));
            }

            // Send exit command
            let _ = conn.writer.write(&ViewerMessage::Exit).await;
            println!("Session terminated.");
        }
        Err(e) => {
            bail!("Failed to connect to session: {}", e);
        }
    }

    Ok(())
}

/// Stub for non-Unix platforms
#[cfg(not(unix))]
pub async fn exit_session(_session: Option<String>) -> Result<()> {
    bail!("Exit is not supported on this platform")
}

/// Forcefully kill a backgrounded session
///
/// Sends SIGKILL to the process immediately, then cleans up files.
/// Use this when `exit` doesn't work or you need immediate termination.
#[cfg(unix)]
pub async fn kill_session(session: Option<String>) -> Result<()> {
    let session_id = if let Some(id) = session {
        id
    } else {
        find_most_recent_pty_session()?
    };

    // Get PID from metadata
    let meta = ipc::read_agent_metadata(&session_id)?
        .ok_or_else(|| anyhow::anyhow!("Session metadata not found: {}", session_id))?;
    let pid = meta.pid
        .ok_or_else(|| anyhow::anyhow!("Session {} has no PID in metadata", session_id))?;

    let pty_socket_path = session::get_session_socket_path(&session_id)?;

    if !pty_socket_path.exists() {
        // Session is stale - just clean up
        println!("Session {} is already dead, cleaning up...", session_id);
        ipc::cleanup_session(&session_id)?;
        println!("Session cleaned up.");
        return Ok(());
    }

    println!("Killing session {} (PID {})...", session_id, pid);

    // Send SIGKILL to forcefully terminate
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }

    // Wait briefly for process to die
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Clean up session files
    ipc::cleanup_session(&session_id)?;

    println!("Session killed and cleaned up.");
    Ok(())
}

/// Find the most recent PTY session ID
fn find_most_recent_pty_session() -> Result<String> {
    let sessions = session::list_sessions()?;

    // Sessions are sorted by modification time, use the first one (most recent)
    sessions
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No backgrounded sessions found"))
}

/// Stub for non-Unix platforms
#[cfg(not(unix))]
pub async fn kill_session(_session: Option<String>) -> Result<()> {
    bail!("Kill is not supported on this platform")
}

/// List all backgrounded sessions
pub async fn list_sessions() -> Result<()> {
    // List sessions with running agents
    let sessions = session::list_sessions()?;

    if sessions.is_empty() {
        println!("No backgrounded sessions.");
        return Ok(());
    }

    println!("Backgrounded sessions:");
    println!();

    for session_id in sessions {
        // Check PTY socket status for additional info
        let pty_socket_path = session::get_session_socket_path(&session_id)?;
        let pty_status = if pty_socket_path.exists() && is_pty_responsive(&pty_socket_path) {
            "running"
        } else if pty_socket_path.exists() {
            "suspended" // PTY exists but not responding (e.g., Ctrl+Z)
        } else {
            "agent-only" // No PTY socket, but agent is alive
        };
        println!("  {} ({})", session_id, pty_status);
    }

    println!();
    println!("Use 'brainwires attach <session_id>' to reconnect.");

    Ok(())
}

/// Check if a PTY socket is responsive
fn is_pty_responsive(socket_path: &std::path::Path) -> bool {
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
