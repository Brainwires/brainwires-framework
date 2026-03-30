//! Agent Process Spawning
//!
//! Helpers for spawning and managing Agent processes from the TUI.

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use tokio::process::Command;

use brainwires::agent_network::ipc::AgentMetadata;
use crate::ipc::{get_agent_socket_path, write_agent_metadata};
use crate::mdap::MdapConfig;

/// Options for spawning an agent process
#[derive(Debug, Clone, Default)]
pub struct SpawnOptions {
    /// Model to use for the agent
    pub model: Option<String>,
    /// MDAP configuration
    pub mdap_config: Option<MdapConfig>,
    /// Parent agent ID (for child agents)
    pub parent_agent_id: Option<String>,
    /// Reason for spawning (displayed in agent tree)
    pub spawn_reason: Option<String>,
    /// Working directory for the agent
    pub working_directory: Option<PathBuf>,
}

impl SpawnOptions {
    /// Create new spawn options with a model
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: Some(model.into()),
            ..Default::default()
        }
    }

    /// Set the parent agent
    pub fn with_parent(mut self, parent_id: impl Into<String>, reason: Option<String>) -> Self {
        self.parent_agent_id = Some(parent_id.into());
        self.spawn_reason = reason;
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }

    /// Set MDAP config
    pub fn with_mdap(mut self, config: MdapConfig) -> Self {
        self.mdap_config = Some(config);
        self
    }
}

/// Spawn an Agent process for the given session
///
/// This spawns a new `brainwires agent <session_id>` process that runs in the
/// background and handles all AI interactions. The TUI connects to this Agent
/// via IPC.
///
/// Returns the socket path once the Agent is ready to accept connections.
pub async fn spawn_agent_process(
    session_id: &str,
    model: Option<&str>,
    mdap_config: Option<&MdapConfig>,
) -> Result<PathBuf> {
    // Use the new options-based function
    let options = SpawnOptions {
        model: model.map(String::from),
        mdap_config: mdap_config.cloned(),
        ..Default::default()
    };
    spawn_agent_process_with_options(session_id, options).await
}

/// Maximum depth for agent tree (prevents runaway recursion)
pub const MAX_AGENT_DEPTH: u32 = 5;

/// Spawn an Agent process with full options
///
/// This is the full-featured spawn function that supports parent-child relationships.
pub async fn spawn_agent_process_with_options(
    session_id: &str,
    options: SpawnOptions,
) -> Result<PathBuf> {
    // Check recursion depth to prevent runaway agent spawning
    if let Some(parent_id) = &options.parent_agent_id {
        let parent_depth = crate::ipc::get_agent_depth(parent_id)?;
        if parent_depth >= MAX_AGENT_DEPTH {
            bail!(
                "Maximum agent depth ({}) reached. Cannot spawn more children. \
                Current parent '{}' is at depth {}.",
                MAX_AGENT_DEPTH,
                parent_id,
                parent_depth
            );
        }
    }

    let socket_path = get_agent_socket_path(session_id)?;

    // Check if agent is already running
    if socket_path.exists() {
        // Try to connect to verify it's alive
        if crate::ipc::is_agent_alive(session_id).await {
            return Ok(socket_path);
        }
        // Stale socket - remove it
        let _ = std::fs::remove_file(&socket_path);
    }

    // Determine working directory
    let working_dir = options.working_directory.clone()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    // Determine model name for metadata
    let model_name = options.model.clone().unwrap_or_else(|| "default".to_string());

    // Create and write metadata BEFORE spawning the process
    // This ensures metadata exists when the agent starts
    let mut metadata = AgentMetadata::new(
        session_id.to_string(),
        model_name.clone(),
        working_dir.to_string_lossy().to_string(),
    );

    // Set parent info if this is a child agent
    if let Some(parent_id) = &options.parent_agent_id {
        metadata = metadata.with_parent(parent_id.clone(), options.spawn_reason.clone());
    }

    // Write metadata (PID will be updated after spawn)
    write_agent_metadata(&metadata)?;

    // Get the current executable path
    let exe_path = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Build command arguments
    let mut args = vec![
        "agent".to_string(),
        session_id.to_string(),
    ];

    if let Some(m) = &options.model {
        args.push("--model".to_string());
        args.push(m.clone());
    }

    // Pass parent info via command line args
    if let Some(parent_id) = &options.parent_agent_id {
        args.push("--parent".to_string());
        args.push(parent_id.clone());
    }

    if let Some(reason) = &options.spawn_reason {
        args.push("--spawn-reason".to_string());
        args.push(reason.clone());
    }

    if let Some(mdap) = &options.mdap_config {
        args.push("--mdap".to_string());
        args.push("--mdap-k".to_string());
        args.push(mdap.k.to_string());
        args.push("--mdap-target".to_string());
        args.push(mdap.target_success_rate.to_string());
        args.push("--mdap-parallel".to_string());
        args.push(mdap.parallel_samples.to_string());
        args.push("--mdap-max-samples".to_string());
        args.push(mdap.max_samples_per_subtask.to_string());
        if mdap.fail_fast {
            args.push("--mdap-fail-fast".to_string());
        }
    }

    // Spawn the agent process detached
    // We redirect stdout/stderr to files for debugging, but the agent
    // primarily communicates via the Unix socket
    let log_dir = crate::utils::paths::PlatformPaths::sessions_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let stdout_path = log_dir.join(format!("{}.stdout.log", session_id));
    let stderr_path = log_dir.join(format!("{}.stderr.log", session_id));

    let stdout_file = std::fs::File::create(&stdout_path)
        .context("Failed to create agent stdout log")?;
    let stderr_file = std::fs::File::create(&stderr_path)
        .context("Failed to create agent stderr log")?;

    // Spawn as a detached process
    #[cfg(unix)]
    let child_pid = {
        use std::os::unix::process::CommandExt;

        let mut cmd = std::process::Command::new(&exe_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .current_dir(&working_dir);

        // Create new process group so it survives TUI exit
        unsafe {
            cmd.pre_exec(|| {
                // Create new session - this detaches from controlling terminal
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let child = cmd.spawn()
            .context("Failed to spawn agent process")?;
        child.id()
    };

    #[cfg(not(unix))]
    let child_pid = {
        let child = Command::new(&exe_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(stdout_file)
            .stderr(stderr_file)
            .current_dir(&working_dir)
            .spawn()
            .context("Failed to spawn agent process")?;
        child.id()
    };

    // Update metadata with PID
    crate::ipc::update_agent_metadata(session_id, |m| {
        m.pid = Some(child_pid);
    }).ok(); // Ignore errors updating metadata

    // Wait for socket to be created (up to 10 seconds)
    for _ in 0..100 {
        if socket_path.exists() {
            // Give the agent a moment to start accepting connections
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if crate::ipc::is_agent_alive(session_id).await {
                return Ok(socket_path);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Clean up metadata on failure
    let _ = crate::ipc::delete_agent_metadata(session_id);

    bail!("Agent process failed to start (socket not created after 10 seconds)")
}

/// Spawn a child agent from a parent agent
///
/// This is a convenience function for spawning child agents during tool execution.
/// It automatically sets up the parent-child relationship.
pub async fn spawn_child_agent(
    parent_session_id: &str,
    reason: impl Into<String>,
    model: Option<String>,
    working_directory: Option<PathBuf>,
) -> Result<(String, PathBuf)> {
    // Generate a new session ID for the child
    let child_session_id = generate_child_session_id(parent_session_id);

    let options = SpawnOptions {
        model,
        parent_agent_id: Some(parent_session_id.to_string()),
        spawn_reason: Some(reason.into()),
        working_directory,
        ..Default::default()
    };

    let socket_path = spawn_agent_process_with_options(&child_session_id, options).await?;

    Ok((child_session_id, socket_path))
}

/// Generate a session ID for a child agent
fn generate_child_session_id(parent_session_id: &str) -> String {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    format!("{}-child-{}-{}", parent_session_id, timestamp, short_id)
}

/// Check if an Agent process is running for the given session
pub async fn is_agent_running(session_id: &str) -> bool {
    crate::ipc::is_agent_alive(session_id).await
}

/// Get the socket path for an existing agent (doesn't spawn)
pub fn get_agent_socket(session_id: &str) -> Result<PathBuf> {
    get_agent_socket_path(session_id)
}

/// Clean up after an agent exits
///
/// This removes the socket, token, and metadata files, and optionally notifies children.
pub async fn cleanup_agent(session_id: &str, notify_children: bool) -> Result<()> {
    // If requested, notify children to exit
    if notify_children {
        let children = crate::ipc::get_child_agents(session_id)?;
        for child in children {
            // Try to send shutdown signal to each child
            if crate::ipc::is_agent_alive(&child.session_id).await {
                // Read child's session token for authenticated connection
                let child_token = match crate::ipc::read_session_token(&child.session_id) {
                    Ok(Some(token)) => token,
                    _ => {
                        tracing::warn!("No session token for child {}", child.session_id);
                        continue;
                    }
                };

                // Connect and send ParentSignal with authentication
                if let Ok(mut conn) = crate::ipc::connect_to_agent(&child.session_id).await {
                    use brainwires::agent_network::ipc::{ViewerMessage, ParentSignalType, Handshake, HandshakeResponse};

                    // Perform authenticated handshake
                    let handshake = Handshake::reattach(child.session_id.clone(), child_token);
                    if conn.writer.write(&handshake).await.is_err() {
                        continue;
                    }

                    // Wait for response
                    if let Ok(Some(response)) = conn.reader.read::<HandshakeResponse>().await {
                        if response.accepted {
                            let signal = ViewerMessage::ParentSignal {
                                signal: ParentSignalType::ParentExiting,
                                parent_session_id: session_id.to_string(),
                            };
                            let _ = conn.writer.write(&signal).await;
                        }
                    }
                }
            }
        }
    }

    // Clean up socket
    let socket_path = get_agent_socket_path(session_id)?;
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    // Clean up session token
    let _ = crate::ipc::delete_session_token(session_id);

    // Clean up metadata
    let _ = crate::ipc::delete_agent_metadata(session_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_is_agent_running_no_agent() {
        // Random session ID that doesn't exist
        let result = is_agent_running("nonexistent-test-session-12345").await;
        assert!(!result);
    }
}
