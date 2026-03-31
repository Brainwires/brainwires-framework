//! Remote Control Bridge - CLI adapter
//!
//! Thin adapter over `brainwires::agent_network::remote`. All types are re-exported
//! from the bridge crate; this module adds CLI-specific trait implementations
//! (`CliAgentSpawner`, `CliBridgeConfigProvider`) and convenience functions.
//!
//! # Security Model
//!
//! - **Outbound-only**: CLI initiates all connections (no inbound ports)
//! - **API Key Auth**: Uses existing `bw_*` API key system
//! - **Session Tokens**: Short-lived tokens for authenticated sessions
//! - **TLS Required**: All communication over secure WebSocket (wss://)

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::agent::spawn::{SpawnOptions, spawn_agent_process_with_options};
use crate::auth::SessionManager;
use crate::config::ConfigManager;
use crate::config::manager::RemoteSettings;
use crate::utils::paths::PlatformPaths;

// ── Private imports from bridge ──────────────────────────────────────────

use brainwires::agent_network::remote::manager::RemoteBridgeManager;
use brainwires::agent_network::traits::{AgentSpawner, BridgeConfigProvider, RemoteBridgeConfig};

// ============================================================================
// CLI Agent Spawner — implements AgentSpawner trait
// ============================================================================

/// CLI implementation of `AgentSpawner`.
///
/// Wraps `spawn_agent_process_with_options` with working-directory security
/// validation (home-dir check) since that's CLI policy.
pub struct CliAgentSpawner;

#[async_trait]
impl AgentSpawner for CliAgentSpawner {
    async fn spawn_agent(
        &self,
        session_id: &str,
        model: Option<String>,
        working_directory: Option<PathBuf>,
    ) -> Result<PathBuf> {
        // Security: validate working directory is within allowed locations
        let validated_dir = if let Some(ref dir) = working_directory {
            if !dir.exists() {
                anyhow::bail!("Working directory does not exist: {}", dir.display());
            }
            if !dir.is_dir() {
                anyhow::bail!("Working directory is not a directory: {}", dir.display());
            }

            let canonical = dir.canonicalize().context(format!(
                "Failed to canonicalize working directory: {}",
                dir.display()
            ))?;

            let home_dir = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

            if !canonical.starts_with(&home_dir) && !canonical.starts_with("/tmp") {
                tracing::warn!(
                    "Working directory {} is outside user's home directory",
                    canonical.display()
                );
                anyhow::bail!(
                    "Working directory must be within your home directory: {}",
                    home_dir.display()
                );
            }

            Some(canonical)
        } else {
            None
        };

        let options = SpawnOptions {
            model,
            working_directory: validated_dir,
            ..Default::default()
        };

        spawn_agent_process_with_options(session_id, options).await
    }
}

// ============================================================================
// CLI Bridge Config Provider — implements BridgeConfigProvider trait
// ============================================================================

/// CLI implementation of `BridgeConfigProvider`.
///
/// Reads remote bridge configuration from `ConfigManager` and API keys
/// from `SessionManager`.
pub struct CliBridgeConfigProvider;

impl BridgeConfigProvider for CliBridgeConfigProvider {
    fn get_remote_config(&self) -> Result<Option<RemoteBridgeConfig>> {
        let config = ConfigManager::new()?.get().clone();

        if !config.remote.enabled {
            return Ok(None);
        }

        // Get API key from config or session fallback
        let api_key = config
            .remote
            .api_key
            .clone()
            .or_else(|| {
                SessionManager::get_api_key()
                    .ok()
                    .flatten()
                    .map(|k| k.to_string())
            })
            .unwrap_or_default();

        // Auto-select backend URL based on API key
        let backend_url = config.remote.auto_select_backend_url(&api_key);

        Ok(Some(RemoteBridgeConfig {
            backend_url,
            api_key,
            heartbeat_interval_secs: config.remote.heartbeat_interval_secs,
            reconnect_delay_secs: config.remote.reconnect_delay_secs,
            max_reconnect_attempts: config.remote.max_reconnect_attempts,
        }))
    }

    fn get_api_key(&self) -> Result<Option<Zeroizing<String>>> {
        SessionManager::get_api_key()
    }
}

// ============================================================================
// Convenience functions (CLI-specific)
// ============================================================================

/// Create a `RemoteBridgeManager` wired with CLI adapters.
///
/// Injects `PlatformPaths` for sessions/attachments, `build_info::VERSION`,
/// `CliAgentSpawner`, and `CliBridgeConfigProvider`.
pub fn create_bridge_manager() -> Result<RemoteBridgeManager> {
    let sessions_dir = PlatformPaths::sessions_dir()?;
    let version = crate::build_info::VERSION.to_string();
    let attachment_dir = PlatformPaths::data_dir()?.join("attachments");

    Ok(RemoteBridgeManager::new(
        Box::new(CliBridgeConfigProvider),
        Arc::new(CliAgentSpawner),
        sessions_dir,
        version,
        attachment_dir,
    ))
}

/// Check if remote bridge should auto-start based on the given settings.
pub fn should_auto_start_with(settings: &RemoteSettings) -> bool {
    settings.enabled && settings.auto_start
}

/// Check if remote bridge should auto-start based on config.
pub fn should_auto_start() -> bool {
    match ConfigManager::new() {
        Ok(manager) => {
            let config = manager.get();
            should_auto_start_with(&config.remote)
        }
        Err(_) => false,
    }
}

/// Try to start remote bridge if configured for auto-start.
///
/// Returns `Ok(true)` if started, `Ok(false)` if not configured, `Err` on failure.
pub async fn try_auto_start() -> Result<bool> {
    if !should_auto_start() {
        return Ok(false);
    }

    let manager = create_bridge_manager()?;
    let result = manager.start_from_config().await?;
    Ok(result.is_some())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_auto_start_default() {
        // Default RemoteSettings has enabled=false, auto_start=true
        let default_settings = RemoteSettings::default();
        assert!(!should_auto_start_with(&default_settings));

        // Explicitly enabled + auto_start should return true
        let enabled = RemoteSettings {
            enabled: true,
            auto_start: true,
            ..Default::default()
        };
        assert!(should_auto_start_with(&enabled));

        // Enabled but auto_start=false should return false
        let no_auto = RemoteSettings {
            enabled: true,
            auto_start: false,
            ..Default::default()
        };
        assert!(!should_auto_start_with(&no_auto));
    }
}
