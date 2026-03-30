//! Agent Hibernate/Resume System
//!
//! Save and restore agent states across reboots/restarts.
//! This is useful for integration with system startup/shutdown scripts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use brainwires::agent_network::ipc::AgentMetadata;
use crate::ipc::{list_agent_sessions_with_metadata, is_agent_alive};
use crate::utils::paths::PlatformPaths;

/// Hibernate manifest - list of sessions to restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HibernateManifest {
    /// Timestamp when hibernation occurred
    pub hibernated_at: i64,
    /// List of hibernated session IDs
    pub sessions: Vec<HibernatedSession>,
    /// Version for forward compatibility
    pub version: u32,
}

/// A hibernated session's metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HibernatedSession {
    /// Session ID
    pub session_id: String,
    /// Model used
    pub model: String,
    /// Working directory
    pub working_directory: String,
    /// Parent agent ID (for hierarchy restoration)
    pub parent_agent_id: Option<String>,
    /// Reason for spawning (if child)
    pub spawn_reason: Option<String>,
    /// Whether the agent was busy when hibernated
    pub was_busy: bool,
}

impl HibernateManifest {
    /// Current manifest version
    pub const VERSION: u32 = 1;

    /// Create a new manifest
    pub fn new(sessions: Vec<HibernatedSession>) -> Self {
        Self {
            hibernated_at: chrono::Utc::now().timestamp(),
            sessions,
            version: Self::VERSION,
        }
    }

    /// Get the manifest file path
    pub fn manifest_path() -> Result<PathBuf> {
        let hibernate_dir = PlatformPaths::sessions_dir()?.join("hibernate");
        Ok(hibernate_dir.join("manifest.json"))
    }

    /// Load manifest from disk
    pub fn load() -> Result<Option<Self>> {
        let path = Self::manifest_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest from {}", path.display()))?;

        let manifest: Self = serde_json::from_str(&content)
            .with_context(|| "Failed to parse hibernate manifest")?;

        Ok(Some(manifest))
    }

    /// Save manifest to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::manifest_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize manifest")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write manifest to {}", path.display()))?;

        tracing::info!("Saved hibernate manifest to {}", path.display());
        Ok(())
    }

    /// Delete manifest from disk
    pub fn delete() -> Result<()> {
        let path = Self::manifest_path()?;
        if path.exists() {
            std::fs::remove_file(&path)?;
            tracing::info!("Deleted hibernate manifest");
        }
        Ok(())
    }
}

/// Hibernate all running agents
///
/// This saves the state of all running agents and shuts them down gracefully.
/// Use `resume_agents()` to restore them later.
pub async fn hibernate_agents() -> Result<Vec<String>> {
    let agents = list_agent_sessions_with_metadata()?;
    let mut hibernated = Vec::new();
    let mut sessions = Vec::new();

    tracing::info!("Hibernating {} agent(s)...", agents.len());

    for agent in agents {
        // Check if agent is alive
        if !is_agent_alive(&agent.session_id).await {
            tracing::info!("Agent {} is not alive, skipping", agent.session_id);
            continue;
        }

        // Create hibernated session record
        let session = HibernatedSession {
            session_id: agent.session_id.clone(),
            model: agent.model.clone(),
            working_directory: agent.working_directory.clone(),
            parent_agent_id: agent.parent_agent_id.clone(),
            spawn_reason: agent.spawn_reason.clone(),
            was_busy: agent.is_busy,
        };
        sessions.push(session);

        // Send shutdown signal via IPC with authentication
        use brainwires::agent_network::ipc::{ViewerMessage, Handshake, HandshakeResponse};
        use crate::ipc::read_session_token;

        // Read session token for authenticated connection
        let session_token = match read_session_token(&agent.session_id) {
            Ok(Some(token)) => token,
            Ok(None) => {
                tracing::warn!("No session token for agent {}, skipping", agent.session_id);
                continue;
            }
            Err(e) => {
                tracing::warn!("Failed to read session token for {}: {:?}", agent.session_id, e);
                continue;
            }
        };

        if let Ok(mut conn) = crate::ipc::connect_to_agent(&agent.session_id).await {
            // Perform authenticated handshake
            let handshake = Handshake::reattach(agent.session_id.clone(), session_token);
            if let Err(e) = conn.writer.write(&handshake).await {
                tracing::warn!("Failed to send handshake to {}: {:?}", agent.session_id, e);
                continue;
            }

            // Wait for handshake response
            match conn.reader.read::<HandshakeResponse>().await {
                Ok(Some(response)) if response.accepted => {
                    // Send detach command
                    let detach = ViewerMessage::Detach { exit_when_done: true };
                    if let Err(e) = conn.writer.write(&detach).await {
                        tracing::warn!("Failed to send detach to {}: {:?}", agent.session_id, e);
                    } else {
                        tracing::info!("Sent hibernate signal to agent {}", agent.session_id);
                        hibernated.push(agent.session_id.clone());
                    }
                }
                Ok(Some(response)) => {
                    tracing::warn!("Agent {} rejected connection: {:?}", agent.session_id, response.error);
                }
                Ok(None) => {
                    tracing::warn!("Agent {} closed connection during handshake", agent.session_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to read handshake response from {}: {:?}", agent.session_id, e);
                }
            }
        } else {
            tracing::warn!("Could not connect to agent {} for hibernate", agent.session_id);
        }
    }

    // Save manifest
    let manifest = HibernateManifest::new(sessions);
    manifest.save()?;

    tracing::info!("Hibernated {} agent(s), manifest saved", hibernated.len());
    Ok(hibernated)
}

/// Resume hibernated agents
///
/// This reads the hibernate manifest and restarts all previously hibernated agents.
pub async fn resume_agents() -> Result<Vec<String>> {
    let manifest = match HibernateManifest::load()? {
        Some(m) => m,
        None => {
            tracing::info!("No hibernate manifest found, nothing to resume");
            return Ok(Vec::new());
        }
    };

    tracing::info!("Resuming {} hibernated agent(s)...", manifest.sessions.len());

    let mut resumed = Vec::new();

    // Sort sessions by hierarchy (parents first)
    let mut sorted_sessions = manifest.sessions.clone();
    sorted_sessions.sort_by(|a, b| {
        // Agents without parents come first
        match (&a.parent_agent_id, &b.parent_agent_id) {
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });

    for session in sorted_sessions {
        // Check if already running
        if is_agent_alive(&session.session_id).await {
            tracing::info!("Agent {} is already running, skipping", session.session_id);
            resumed.push(session.session_id.clone());
            continue;
        }

        // Spawn the agent
        use crate::agent::SpawnOptions;

        let mut options = SpawnOptions::default();
        options.model = Some(session.model.clone());
        options.parent_agent_id = session.parent_agent_id.clone();
        options.spawn_reason = session.spawn_reason.clone();

        if !session.working_directory.is_empty() {
            options.working_directory = Some(PathBuf::from(&session.working_directory));
        }

        match crate::agent::spawn_agent_process_with_options(&session.session_id, options).await {
            Ok(_) => {
                tracing::info!("Resumed agent {}", session.session_id);
                resumed.push(session.session_id.clone());
            }
            Err(e) => {
                tracing::error!("Failed to resume agent {}: {:?}", session.session_id, e);
            }
        }
    }

    // Clear the manifest now that we've resumed
    HibernateManifest::delete()?;

    tracing::info!("Resumed {} agent(s)", resumed.len());
    Ok(resumed)
}

/// Check if there are hibernated agents to resume
pub fn has_hibernated_agents() -> Result<bool> {
    let path = HibernateManifest::manifest_path()?;
    Ok(path.exists())
}

/// Get list of hibernated sessions (for display)
pub fn list_hibernated_sessions() -> Result<Vec<HibernatedSession>> {
    match HibernateManifest::load()? {
        Some(manifest) => Ok(manifest.sessions),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let session = HibernatedSession {
            session_id: "test-123".to_string(),
            model: "gpt-4".to_string(),
            working_directory: "/home/user/project".to_string(),
            parent_agent_id: None,
            spawn_reason: None,
            was_busy: false,
        };

        let manifest = HibernateManifest::new(vec![session]);
        let json = serde_json::to_string(&manifest).unwrap();

        let parsed: HibernateManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.sessions[0].session_id, "test-123");
        assert_eq!(parsed.version, HibernateManifest::VERSION);
    }
}
