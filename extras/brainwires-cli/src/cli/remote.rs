//! Remote Bridge CLI Commands
//!
//! Commands for managing the remote bridge connection to brainwires-studio.

use std::process::Stdio;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::config::ConfigManager;
use crate::remote::{create_bridge_manager, try_auto_start};
use crate::utils::logger::Logger;

#[derive(Subcommand)]
pub enum RemoteCommands {
    /// Start the remote bridge connection (runs as background daemon)
    Start {
        /// Force start even if already running
        #[arg(long)]
        force: bool,

        /// Run in foreground instead of as daemon (for debugging)
        #[arg(long, short = 'f')]
        foreground: bool,
    },

    /// Internal: Run the bridge (called by daemon spawn, not for direct use)
    #[clap(hide = true)]
    Daemon,

    /// Stop the remote bridge connection
    Stop,

    /// Check remote bridge status
    Status,

    /// View remote bridge logs
    Log {
        /// Follow the log output (like tail -f)
        #[arg(long, short = 'f')]
        follow: bool,

        /// Number of lines to show (default: 50)
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,

        /// Clear the log file
        #[arg(long)]
        clear: bool,
    },

    /// Pair this device with your Brainwires Studio account (no API key needed)
    Pair,

    /// Configure remote bridge settings
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Enable or disable remote bridge
        #[arg(long)]
        enabled: Option<bool>,

        /// Set the backend URL
        #[arg(long)]
        url: Option<String>,

        /// Set the API key
        #[arg(long)]
        api_key: Option<String>,

        /// Set heartbeat interval in seconds
        #[arg(long)]
        heartbeat: Option<u32>,
    },
}

pub async fn handle_remote(cmd: RemoteCommands) -> Result<()> {
    match cmd {
        RemoteCommands::Start { force, foreground } => handle_start(force, foreground).await,
        RemoteCommands::Daemon => handle_daemon().await,
        RemoteCommands::Pair => handle_pair().await,
        RemoteCommands::Stop => handle_stop().await,
        RemoteCommands::Status => handle_status().await,
        RemoteCommands::Log { follow, lines, clear } => handle_log(follow, lines, clear).await,
        RemoteCommands::Config { show, enabled, url, api_key, heartbeat } => {
            handle_config(show, enabled, url, api_key, heartbeat).await
        }
    }
}

async fn handle_start(force: bool, foreground: bool) -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let config = config_manager.get();
    let settings = &config.remote;

    if !settings.enabled {
        Logger::warn("Remote bridge is disabled in config. Enable it first:");
        Logger::info("  brainwires remote config --enabled true");
        return Ok(());
    }

    if settings.api_key.is_none() {
        Logger::error("No API key configured. Set one first:");
        Logger::info("  brainwires remote config --api-key <your-key>");
        return Ok(());
    }

    // Check if already running via PID file
    if !force && is_bridge_running() {
        Logger::warn("Remote bridge is already running");
        Logger::info("Use --force to restart, or 'brainwires remote stop' first");
        return Ok(());
    }

    if foreground {
        // Run directly in this process (for debugging)
        Logger::info("Starting remote bridge in foreground...");
        return handle_daemon().await;
    }

    // Spawn as a background daemon process
    Logger::info("Starting remote bridge daemon...");
    spawn_bridge_daemon().await?;

    // Wait a moment for it to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    if is_bridge_running() {
        Logger::success("Remote bridge daemon started");
        Logger::info(&format!("Backend URL: {}", settings.backend_url));
    } else {
        Logger::error("Failed to start remote bridge daemon");
        Logger::info("Check logs at ~/.brainwires/remote-bridge.log");
    }

    Ok(())
}

/// Spawn the bridge as a detached daemon process
async fn spawn_bridge_daemon() -> Result<()> {
    let exe_path = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Log file for daemon output
    let log_dir = crate::utils::paths::PlatformPaths::data_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let log_path = log_dir.join("remote-bridge.log");
    let log_file = std::fs::File::create(&log_path)
        .context("Failed to create bridge log file")?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let mut cmd = std::process::Command::new(&exe_path);
        cmd.args(["remote", "daemon"])
            .stdin(Stdio::null())
            .stdout(log_file.try_clone()?)
            .stderr(log_file);

        // Create new session so it survives parent exit
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        cmd.spawn().context("Failed to spawn bridge daemon")?;
    }

    #[cfg(not(unix))]
    {
        std::process::Command::new(&exe_path)
            .args(["remote", "daemon"])
            .stdin(Stdio::null())
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .spawn()
            .context("Failed to spawn bridge daemon")?;
    }

    Ok(())
}

/// Handle the daemon subcommand (runs the actual bridge)
async fn handle_daemon() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let config = config_manager.get();
    let settings = &config.remote;

    if !settings.enabled || settings.api_key.is_none() {
        anyhow::bail!("Remote bridge not properly configured");
    }

    // Write PID file
    write_pid_file()?;

    // Set up cleanup on exit
    let _guard = PidFileGuard;

    tracing::info!("Remote bridge daemon starting...");

    let manager = create_bridge_manager()?;

    match manager.start_from_config().await {
        Ok(Some(handle)) => {
            tracing::info!("Remote bridge connected to {}", settings.backend_url);

            // Wait for shutdown signal or task completion
            let shutdown = tokio::signal::ctrl_c();

            tokio::select! {
                _ = shutdown => {
                    tracing::info!("Received shutdown signal");
                    manager.stop().await;
                }
                result = handle => {
                    match result {
                        Ok(()) => tracing::info!("Bridge task completed"),
                        Err(e) => tracing::error!("Bridge task error: {}", e),
                    }
                }
            }
        }
        Ok(None) => {
            tracing::warn!("Bridge already running or not configured");
        }
        Err(e) => {
            tracing::error!("Failed to start bridge: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Get the PID file path
fn get_pid_file_path() -> Result<std::path::PathBuf> {
    let data_dir = crate::utils::paths::PlatformPaths::data_dir()?;
    Ok(data_dir.join("remote-bridge.pid"))
}

/// Write the current process PID to the PID file
fn write_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path()?;
    std::fs::write(&pid_path, std::process::id().to_string())
        .context("Failed to write PID file")?;
    Ok(())
}

/// Check if bridge is running by checking PID file
fn is_bridge_running() -> bool {
    let pid_path = match get_pid_file_path() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if !pid_path.exists() {
        return false;
    }

    let pid_str = match std::fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check if process is running
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(not(unix))]
    {
        // On Windows, just check if PID file exists (less accurate)
        true
    }
}

/// Get the running bridge PID
fn get_bridge_pid() -> Option<u32> {
    let pid_path = get_pid_file_path().ok()?;
    let pid_str = std::fs::read_to_string(&pid_path).ok()?;
    pid_str.trim().parse().ok()
}

/// Guard that removes PID file on drop
struct PidFileGuard;

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        if let Ok(pid_path) = get_pid_file_path() {
            let _ = std::fs::remove_file(pid_path);
        }
    }
}

async fn handle_stop() -> Result<()> {
    Logger::info("Stopping remote bridge...");

    if let Some(pid) = get_bridge_pid() {
        #[cfg(unix)]
        {
            // Send SIGTERM to gracefully stop
            unsafe {
                if libc::kill(pid as i32, libc::SIGTERM) == 0 {
                    Logger::success(&format!("Sent stop signal to bridge (PID {})", pid));

                    // Wait for process to exit
                    for _ in 0..20 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        if libc::kill(pid as i32, 0) != 0 {
                            // Clean up PID file
                            if let Ok(pid_path) = get_pid_file_path() {
                                let _ = std::fs::remove_file(pid_path);
                            }
                            Logger::success("Remote bridge stopped");
                            return Ok(());
                        }
                    }

                    // Force kill if still running
                    Logger::warn("Bridge didn't stop gracefully, forcing...");
                    libc::kill(pid as i32, libc::SIGKILL);
                    // Clean up PID file
                    if let Ok(pid_path) = get_pid_file_path() {
                        let _ = std::fs::remove_file(pid_path);
                    }
                    Logger::success("Remote bridge force stopped");
                } else {
                    Logger::warn("Bridge process not found (stale PID file)");
                    // Clean up stale PID file
                    if let Ok(pid_path) = get_pid_file_path() {
                        let _ = std::fs::remove_file(pid_path);
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            Logger::warn("Cannot stop bridge on this platform - please kill manually");
        }
    } else {
        Logger::info("Remote bridge is not running");
    }

    Ok(())
}

async fn handle_status() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let config = config_manager.get();
    let settings = &config.remote;

    Logger::info("Remote Bridge Status:");
    Logger::info("---------------------");

    // Config status
    Logger::info(&format!("  Enabled: {}", settings.enabled));
    Logger::info(&format!("  Backend URL: {}", settings.backend_url));
    Logger::info(&format!("  API Key: {}", if settings.api_key.is_some() { "configured" } else { "not set" }));
    Logger::info(&format!("  Heartbeat: {}s", settings.heartbeat_interval_secs));
    Logger::info(&format!("  Auto-start: {}", settings.auto_start));

    // Daemon status
    if let Some(pid) = get_bridge_pid() {
        if is_bridge_running() {
            Logger::success(&format!("  Status: Running (PID {})", pid));
        } else {
            Logger::warn("  Status: Stale PID file (not running)");
        }
    } else {
        Logger::info("  Status: Not running");
    }

    Ok(())
}

/// Get the log file path
fn get_log_file_path() -> Result<std::path::PathBuf> {
    let data_dir = crate::utils::paths::PlatformPaths::data_dir()?;
    Ok(data_dir.join("brainwires").join("remote-bridge.log"))
}

async fn handle_log(follow: bool, lines: usize, clear: bool) -> Result<()> {
    let log_path = get_log_file_path()?;

    if clear {
        if log_path.exists() {
            std::fs::write(&log_path, "")?;
            Logger::success("Log file cleared");
        } else {
            Logger::info("No log file to clear");
        }
        return Ok(());
    }

    if !log_path.exists() {
        Logger::info("No log file found. The bridge may not have been started yet.");
        Logger::info(&format!("Log path: {}", log_path.display()));
        return Ok(());
    }

    if follow {
        // Follow mode - like tail -f
        Logger::info(&format!("Following log: {} (Ctrl+C to stop)", log_path.display()));
        Logger::info("---");

        // First, show the last N lines
        show_last_lines(&log_path, lines)?;

        // Then follow new content
        follow_log_file(&log_path).await?;
    } else {
        // Just show the last N lines
        Logger::info(&format!("Remote Bridge Log (last {} lines):", lines));
        Logger::info(&format!("Log path: {}", log_path.display()));
        Logger::info("---");
        show_last_lines(&log_path, lines)?;
    }

    Ok(())
}

/// Show the last N lines of a file
fn show_last_lines(path: &std::path::Path, n: usize) -> Result<()> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    // Collect all lines and take the last N
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    let start = if all_lines.len() > n {
        all_lines.len() - n
    } else {
        0
    };

    for line in &all_lines[start..] {
        println!("{}", line);
    }

    Ok(())
}

/// Follow the log file for new content (like tail -f)
async fn follow_log_file(path: &std::path::Path) -> Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    use tokio::signal;

    let mut file = std::fs::File::open(path)?;
    // Seek to end of file
    file.seek(SeekFrom::End(0))?;

    let mut buffer = [0u8; 4096];
    let mut partial_line = String::new();

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                Logger::info("\nStopped following log");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Check for new content
                match file.read(&mut buffer) {
                    Ok(0) => {
                        // No new data, continue waiting
                    }
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buffer[..n]);
                        partial_line.push_str(&text);

                        // Print complete lines
                        while let Some(newline_pos) = partial_line.find('\n') {
                            let line = &partial_line[..newline_pos];
                            println!("{}", line);
                            partial_line = partial_line[newline_pos + 1..].to_string();
                        }
                    }
                    Err(e) => {
                        Logger::error(&format!("Error reading log: {}", e));
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_config(
    show: bool,
    enabled: Option<bool>,
    url: Option<String>,
    api_key: Option<String>,
    heartbeat: Option<u32>,
) -> Result<()> {
    let mut config_manager = ConfigManager::new()?;

    // If no options provided or show requested, just display config
    if show || (enabled.is_none() && url.is_none() && api_key.is_none() && heartbeat.is_none()) {
        let config = config_manager.get();
        Logger::info("Remote Bridge Configuration:");
        Logger::info("----------------------------");
        Logger::info(&format!("  enabled: {}", config.remote.enabled));
        Logger::info(&format!("  backend_url: {}", config.remote.backend_url));
        Logger::info(&format!("  api_key: {}", if config.remote.api_key.is_some() { "<configured>" } else { "<not set>" }));
        Logger::info(&format!("  heartbeat_interval_secs: {}", config.remote.heartbeat_interval_secs));
        Logger::info(&format!("  reconnect_delay_secs: {}", config.remote.reconnect_delay_secs));
        Logger::info(&format!("  max_reconnect_attempts: {}", config.remote.max_reconnect_attempts));
        Logger::info(&format!("  auto_start: {}", config.remote.auto_start));
        return Ok(());
    }

    // Build remote settings updates
    let mut remote = config_manager.get().remote.clone();
    let mut changed = false;

    if let Some(val) = enabled {
        remote.enabled = val;
        Logger::info(&format!("Set remote.enabled = {}", val));
        changed = true;
    }

    if let Some(val) = url {
        remote.backend_url = val.clone();
        Logger::info(&format!("Set remote.backend_url = {}", val));
        changed = true;
    }

    if let Some(val) = api_key {
        remote.api_key = Some(val);
        Logger::info("Set remote.api_key = <hidden>");
        changed = true;
    }

    if let Some(val) = heartbeat {
        remote.heartbeat_interval_secs = val;
        Logger::info(&format!("Set remote.heartbeat_interval_secs = {}", val));
        changed = true;
    }

    if changed {
        use crate::config::manager::ConfigUpdates;
        config_manager.update(ConfigUpdates {
            remote: Some(remote),
            ..Default::default()
        });
        config_manager.save()?;
        Logger::success("Configuration saved");
    }

    Ok(())
}

// ── Pairing ─────────────────────────────────────────────────────────────

async fn handle_pair() -> Result<()> {
    Logger::info("Starting device pairing...");

    let mut config_manager = ConfigManager::new()?;
    let config = config_manager.get();
    let backend_url = config.remote.backend_url.clone();

    let client = reqwest::Client::new();

    // Compute device fingerprint for auto-registration
    let device_fingerprint = brainwires::agent_network::remote::compute_device_fingerprint();

    // Step 1: Initiate pairing
    let initiate_url = format!("{}/api/remote/pair/initiate", backend_url);
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let os_name = std::env::consts::OS.to_string();
    let version = crate::build_info::VERSION.to_string();

    let initiate_body = serde_json::json!({
        "hostname": hostname,
        "os": os_name,
        "cli_version": version,
        "device_fingerprint": device_fingerprint,
    });

    let res = client
        .post(&initiate_url)
        .json(&initiate_body)
        .send()
        .await
        .context("Failed to connect to backend")?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Pairing initiation failed: {}", body);
    }

    let initiate_response: serde_json::Value = res.json().await?;
    let request_id = initiate_response
        .get("request_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing request_id in response"))?;
    let pairing_code = initiate_response
        .get("pairing_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing pairing_code in response"))?;
    let expires_at = initiate_response
        .get("expires_at")
        .and_then(|v| v.as_str())
        .unwrap_or("5 minutes");

    // Step 2: Show code to user
    println!();
    Logger::success(&format!("Pairing code: {}", pairing_code));
    println!();
    Logger::info(&format!(
        "Go to {}/pair and enter this code to pair your device.",
        backend_url
    ));
    Logger::info(&format!("Code expires at: {}", expires_at));
    println!();
    Logger::info("Waiting for confirmation...");

    // Step 3: Poll for confirmation
    let status_url = format!(
        "{}/api/remote/pair/status/{}",
        backend_url, request_id
    );
    let poll_interval = std::time::Duration::from_secs(2);
    let max_polls = 150; // 5 minutes at 2-second intervals

    for _ in 0..max_polls {
        tokio::time::sleep(poll_interval).await;

        let res = match client.get(&status_url).send().await {
            Ok(r) => r,
            Err(_) => continue, // Network hiccup, retry
        };

        if !res.status().is_success() {
            continue;
        }

        let status_response: serde_json::Value = match res.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let status = status_response
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");

        match status {
            "confirmed" => {
                if let Some(api_key) = status_response.get("api_key").and_then(|v| v.as_str()) {
                    // Save API key to config and authenticate
                    Logger::success("Device paired successfully!");

                    // Save API key in config
                    {
                        let config = config_manager.get_mut();
                        config.remote.api_key = Some(api_key.to_string());
                        config.remote.enabled = true;
                    }
                    config_manager.save()?;

                    // Authenticate with the new key
                    let auth_client = crate::auth::AuthClient::new(backend_url.clone());
                    match auth_client.authenticate(api_key).await {
                        Ok(session) => {
                            Logger::success(&format!(
                                "Authenticated as {}",
                                session.user.display_name
                            ));
                        }
                        Err(e) => {
                            Logger::warn(&format!(
                                "API key saved but authentication failed: {}. You can still use 'brainwires remote start'.",
                                e
                            ));
                        }
                    }

                    Logger::info("Remote bridge is now configured. Start it with:");
                    Logger::info("  brainwires remote start");
                    return Ok(());
                } else if status_response.get("api_key_retrieved").is_some() {
                    Logger::error("Pairing was confirmed but API key was already retrieved.");
                    Logger::info("Please try pairing again: brainwires remote pair");
                    return Ok(());
                }
            }
            "expired" => {
                Logger::error("Pairing code expired. Please try again.");
                return Ok(());
            }
            "rejected" => {
                Logger::error("Pairing was rejected.");
                return Ok(());
            }
            "pending" => {
                // Still waiting
            }
            other => {
                Logger::warn(&format!("Unexpected status: {}", other));
            }
        }
    }

    Logger::error("Pairing timed out. Please try again.");
    Ok(())
}

/// Try to auto-start the remote bridge if configured
/// Call this at application startup
pub async fn maybe_auto_start() {
    match try_auto_start().await {
        Ok(true) => {
            Logger::info("Remote bridge auto-started");
        }
        Ok(false) => {
            // Not enabled or not configured, that's fine
        }
        Err(e) => {
            Logger::warn(&format!("Failed to auto-start remote bridge: {}", e));
        }
    }
}
