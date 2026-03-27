//! Remote Bridge - Backend communication client
//!
//! Maintains communication with the brainwires-studio backend using either:
//! 1. **Supabase Realtime** (preferred) - Bidirectional WebSocket for instant commands
//! 2. **HTTP Polling** (fallback) - For environments where Realtime isn't available
//!
//! All CLI-specific dependencies have been removed:
//! - `PlatformPaths` → `BridgeConfig.sessions_dir` / `BridgeConfig.attachment_dir`
//! - `crate::build_info::VERSION` → `BridgeConfig.version`
//! - `spawn_agent_process_with_options` → `AgentSpawner` trait object
//! - `crate::ipc::*` → bridge-internal `crate::ipc::*`

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const REMOTE_BRIDGE_TIMEOUT_SECS: u64 = 30;

use anyhow::{Context, Result, bail};
use tokio::sync::RwLock;

use super::attachments::AttachmentReceiver;
use super::heartbeat::HeartbeatCollector;
use super::protocol::{
    BackendCommand, NegotiatedProtocol, ProtocolCapability, ProtocolHello, RemoteMessage,
    StreamChunkType,
};
use super::realtime::{RealtimeClient, RealtimeConfig};
use crate::ipc::{AgentMessage, Handshake, HandshakeResponse, IpcConnection, ViewerMessage};
use crate::traits::AgentSpawner;

/// Remote bridge configuration
///
/// All platform-specific values (version, paths) are injected via this config
/// instead of being read from CLI globals.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Backend base URL (https://...)
    pub backend_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Heartbeat/poll interval in seconds
    pub heartbeat_interval_secs: u32,
    /// Reconnect delay on disconnect
    pub reconnect_delay_secs: u32,
    /// Maximum reconnect attempts (0 = unlimited)
    pub max_reconnect_attempts: u32,
    /// CLI version string (injected, replaces build_info::VERSION)
    pub version: String,
    /// Sessions directory for IPC discovery (injected, replaces PlatformPaths)
    pub sessions_dir: PathBuf,
    /// Attachment storage directory (injected, replaces PlatformPaths::data_dir())
    pub attachment_dir: PathBuf,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            backend_url: "https://brainwires.studio".to_string(),
            api_key: String::new(),
            heartbeat_interval_secs: 5,
            reconnect_delay_secs: 5,
            max_reconnect_attempts: 0,
            version: "unknown".to_string(),
            sessions_dir: PathBuf::from("/tmp/brainwires-sessions"),
            attachment_dir: PathBuf::from("/tmp/brainwires-attachments"),
        }
    }
}

/// Bridge state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeState {
    /// Not connected to the backend.
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Connected but not yet authenticated.
    Connected,
    /// Successfully authenticated with the backend.
    Authenticated,
    /// Gracefully shutting down.
    ShuttingDown,
}

/// Handle for an active agent subscription reader task
struct AgentSubscription {
    /// Cancel token to stop the reader task
    cancel_tx: tokio::sync::oneshot::Sender<()>,
    /// Task handle
    task_handle: tokio::task::JoinHandle<()>,
    /// Writer for sending messages to this agent
    writer_tx: tokio::sync::mpsc::Sender<ViewerMessage>,
}

/// Connection mode (Realtime or Polling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    /// Using Supabase Realtime WebSocket (preferred)
    Realtime,
    /// Using HTTP polling (fallback)
    Polling,
}

/// Realtime credentials returned by backend
#[derive(Debug, Clone)]
pub struct RealtimeCredentials {
    /// JWT token for Supabase Realtime authentication.
    pub realtime_token: String,
    /// WebSocket URL for Supabase Realtime.
    pub realtime_url: String,
    /// Channel name to subscribe to.
    pub channel_name: String,
    /// Supabase anonymous key for Kong auth.
    pub supabase_anon_key: String,
}

/// Remote control bridge
///
/// Maintains communication with the backend using either Supabase Realtime
/// (preferred) or HTTP polling (fallback).
#[derive(Clone)]
pub struct RemoteBridge {
    config: BridgeConfig,
    http_client: reqwest::Client,
    /// Current bridge connection state.
    pub state: Arc<RwLock<BridgeState>>,
    connection_mode: Arc<RwLock<ConnectionMode>>,
    session_token: Arc<RwLock<Option<String>>>,
    user_id: Arc<RwLock<Option<String>>>,
    realtime_credentials: Arc<RwLock<Option<RealtimeCredentials>>>,
    subscriptions: Arc<RwLock<HashSet<String>>>,
    subscription_tasks: Arc<RwLock<HashMap<String, AgentSubscription>>>,
    heartbeat_collector: Arc<RwLock<HeartbeatCollector>>,
    command_result_queue: Arc<RwLock<Vec<RemoteMessage>>>,
    #[allow(clippy::type_complexity)]
    stream_tx: Arc<RwLock<Option<tokio::sync::mpsc::Sender<(String, StreamChunkType, String)>>>>,
    sync_trigger_tx: Arc<RwLock<Option<tokio::sync::mpsc::Sender<()>>>>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
    negotiated_protocol: Arc<RwLock<NegotiatedProtocol>>,
    attachment_receiver: AttachmentReceiver,
    /// Agent spawner for creating new agent processes (injected trait)
    agent_spawner: Option<Arc<dyn AgentSpawner>>,
    /// Device allowlist status from last authentication.
    pub device_status: Arc<RwLock<Option<super::protocol::DeviceStatus>>>,
    /// Organization policies from last authentication.
    pub org_policies: Arc<RwLock<Option<super::protocol::OrgPolicies>>>,
}

impl RemoteBridge {
    /// Create a new remote bridge
    ///
    /// # Arguments
    /// * `config` - Bridge configuration with all injected platform values
    /// * `agent_spawner` - Optional agent spawner for remote agent creation
    pub fn new(config: BridgeConfig, agent_spawner: Option<Arc<dyn AgentSpawner>>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REMOTE_BRIDGE_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        let heartbeat_collector =
            HeartbeatCollector::new(config.sessions_dir.clone(), config.version.clone());

        let attachment_receiver = AttachmentReceiver::new(config.attachment_dir.clone());

        Self {
            config,
            http_client,
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            connection_mode: Arc::new(RwLock::new(ConnectionMode::Polling)),
            session_token: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            realtime_credentials: Arc::new(RwLock::new(None)),
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            subscription_tasks: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_collector: Arc::new(RwLock::new(heartbeat_collector)),
            command_result_queue: Arc::new(RwLock::new(Vec::new())),
            stream_tx: Arc::new(RwLock::new(None)),
            sync_trigger_tx: Arc::new(RwLock::new(None)),
            shutdown_tx: None,
            negotiated_protocol: Arc::new(RwLock::new(NegotiatedProtocol::default())),
            attachment_receiver,
            agent_spawner,
            device_status: Arc::new(RwLock::new(None)),
            org_policies: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current connection mode
    pub async fn connection_mode(&self) -> ConnectionMode {
        *self.connection_mode.read().await
    }

    /// Get current bridge state
    pub async fn state(&self) -> BridgeState {
        *self.state.read().await
    }

    /// Check if bridge is connected and authenticated
    pub async fn is_ready(&self) -> bool {
        *self.state.read().await == BridgeState::Authenticated
    }

    /// Get the user ID (if authenticated)
    pub async fn user_id(&self) -> Option<String> {
        self.user_id.read().await.clone()
    }

    /// Get the negotiated protocol version
    pub async fn protocol_version(&self) -> String {
        self.negotiated_protocol.read().await.version.clone()
    }

    /// Check if a capability is enabled in the negotiated protocol
    pub async fn has_capability(&self, cap: ProtocolCapability) -> bool {
        self.negotiated_protocol.read().await.has_capability(cap)
    }

    /// Get all enabled capabilities
    pub async fn enabled_capabilities(&self) -> Vec<ProtocolCapability> {
        self.negotiated_protocol.read().await.capabilities.clone()
    }

    /// Set the shutdown signal sender (for external shutdown control)
    pub fn set_shutdown_tx(&mut self, tx: tokio::sync::broadcast::Sender<()>) {
        self.shutdown_tx = Some(tx);
    }

    /// Connect to the backend and run the main communication loop
    pub async fn run(&mut self) -> Result<()> {
        let shutdown_tx = self.shutdown_tx.clone().unwrap_or_else(|| {
            let (tx, _) = tokio::sync::broadcast::channel(1);
            self.shutdown_tx = Some(tx.clone());
            tx
        });

        let mut reconnect_attempts = 0;

        loop {
            if *self.state.read().await == BridgeState::ShuttingDown {
                tracing::info!("Remote bridge shutting down");
                break;
            }

            *self.state.write().await = BridgeState::Connecting;

            match self.register_with_backend().await {
                Ok(()) => {
                    reconnect_attempts = 0;
                    *self.state.write().await = BridgeState::Authenticated;

                    let realtime_creds = self.realtime_credentials.read().await.clone();

                    if let Some(creds) = realtime_creds {
                        *self.connection_mode.write().await = ConnectionMode::Realtime;
                        tracing::info!("Using Supabase Realtime for communication");

                        if let Err(e) = self.run_realtime_loop(shutdown_tx.subscribe(), creds).await
                        {
                            tracing::error!("Remote bridge Realtime error: {:?}", e);
                        }
                    } else {
                        *self.connection_mode.write().await = ConnectionMode::Polling;
                        tracing::info!(
                            "Using HTTP polling for communication (Realtime not available)"
                        );

                        if let Err(e) = self.run_polling_loop(shutdown_tx.subscribe()).await {
                            tracing::error!("Remote bridge polling error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to register with backend: {}", e);
                    reconnect_attempts += 1;

                    if self.config.max_reconnect_attempts > 0
                        && reconnect_attempts >= self.config.max_reconnect_attempts
                    {
                        bail!(
                            "Max reconnect attempts ({}) reached",
                            self.config.max_reconnect_attempts
                        );
                    }
                }
            }

            // Clean up state
            *self.state.write().await = BridgeState::Disconnected;
            *self.connection_mode.write().await = ConnectionMode::Polling;
            *self.session_token.write().await = None;
            *self.realtime_credentials.write().await = None;
            self.subscriptions.write().await.clear();
            self.command_result_queue.write().await.clear();

            // Wait before reconnecting
            if *self.state.read().await != BridgeState::ShuttingDown {
                tracing::info!(
                    "Reconnecting in {} seconds...",
                    self.config.reconnect_delay_secs
                );
                tokio::time::sleep(Duration::from_secs(self.config.reconnect_delay_secs as u64))
                    .await;
            }
        }

        Ok(())
    }

    /// Shutdown the bridge
    pub async fn shutdown(&mut self) {
        *self.state.write().await = BridgeState::ShuttingDown;

        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
    }

    /// Queue a command result to send with the next heartbeat
    async fn queue_command_result_msg(&self, msg: RemoteMessage) -> Result<()> {
        self.command_result_queue.write().await.push(msg);
        Ok(())
    }

    /// Register with the backend via HTTP POST
    async fn register_with_backend(&mut self) -> Result<()> {
        let url = format!("{}/api/remote/connect", self.config.backend_url);
        tracing::info!("Registering with backend: {}", url);

        let protocol_hello = ProtocolHello::default();
        let device_fingerprint = super::protocol::compute_device_fingerprint();
        let register_body = serde_json::json!({
            "hostname": gethostname::gethostname().to_string_lossy().to_string(),
            "os": std::env::consts::OS.to_string(),
            "version": self.config.version.clone(),
            "protocol": protocol_hello,
            "device_fingerprint": device_fingerprint,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&register_body)
            .send()
            .await
            .context("Failed to connect to backend")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Registration failed: {} - {}", status, body);
        }

        let auth_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse registration response")?;

        if let Some(error) = auth_response.get("error") {
            bail!("Authentication failed: {}", error);
        }

        let session_token = auth_response
            .get("session_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing session_token in response"))?;

        let user_id = auth_response
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing user_id in response"))?;

        tracing::info!("Authenticated as user: {}", user_id);
        *self.session_token.write().await = Some(session_token.to_string());
        *self.user_id.write().await = Some(user_id.to_string());

        // Handle protocol negotiation response
        if let Some(protocol_value) = auth_response.get("protocol") {
            match serde_json::from_value::<super::protocol::ProtocolAccept>(protocol_value.clone())
            {
                Ok(accept) => {
                    tracing::info!(
                        "Protocol negotiated: version={}, capabilities={:?}",
                        accept.selected_version,
                        accept.enabled_capabilities
                    );
                    *self.negotiated_protocol.write().await =
                        NegotiatedProtocol::from_accept(accept);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse protocol accept: {}, using defaults", e);
                    *self.negotiated_protocol.write().await = NegotiatedProtocol::default();
                }
            }
        } else {
            tracing::debug!("Backend did not return protocol, using defaults");
            *self.negotiated_protocol.write().await = NegotiatedProtocol::default();
        }

        // Handle device allowlist status
        if let Some(ds) = auth_response.get("device_status") {
            match serde_json::from_value::<super::protocol::DeviceStatus>(ds.clone()) {
                Ok(status) => {
                    tracing::info!("Device status: {:?}", status);
                    if matches!(status, super::protocol::DeviceStatus::Blocked) {
                        bail!("Device is blocked by the user's device allowlist");
                    }
                    *self.device_status.write().await = Some(status);
                }
                Err(e) => tracing::warn!("Failed to parse device_status: {}", e),
            }
        }

        // Handle organization policies
        if let Some(op) = auth_response.get("org_policies") {
            match serde_json::from_value::<super::protocol::OrgPolicies>(op.clone()) {
                Ok(policies) => {
                    tracing::info!(
                        "Org policies: blocked_tools={:?}, permission_relay_required={}",
                        policies.blocked_tools,
                        policies.permission_relay_required
                    );
                    *self.org_policies.write().await = Some(policies);
                }
                Err(e) => tracing::warn!("Failed to parse org_policies: {}", e),
            }
        }

        // Check for Realtime credentials
        let use_realtime = auth_response
            .get("use_realtime")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if use_realtime {
            let realtime_token = auth_response.get("realtime_token").and_then(|v| v.as_str());
            let realtime_url = auth_response.get("realtime_url").and_then(|v| v.as_str());
            let channel_name = auth_response.get("channel_name").and_then(|v| v.as_str());
            let supabase_anon_key = auth_response
                .get("supabase_anon_key")
                .and_then(|v| v.as_str());

            if let (Some(token), Some(url), Some(channel), Some(anon_key)) = (
                realtime_token,
                realtime_url,
                channel_name,
                supabase_anon_key,
            ) {
                tracing::info!("Realtime credentials received, channel: {}", channel);
                *self.realtime_credentials.write().await = Some(RealtimeCredentials {
                    realtime_token: token.to_string(),
                    realtime_url: url.to_string(),
                    channel_name: channel.to_string(),
                    supabase_anon_key: anon_key.to_string(),
                });
            } else {
                tracing::warn!(
                    "use_realtime=true but missing Realtime credentials (token={}, url={}, channel={}, anon_key={})",
                    realtime_token.is_some(),
                    realtime_url.is_some(),
                    channel_name.is_some(),
                    supabase_anon_key.is_some()
                );
            }
        }

        Ok(())
    }

    /// Main Realtime WebSocket loop (preferred mode)
    async fn run_realtime_loop(
        &mut self,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
        creds: RealtimeCredentials,
    ) -> Result<()> {
        let user_id = self.user_id.read().await.clone().unwrap_or_default();
        let session_token = self.session_token.read().await.clone().unwrap_or_default();

        let config = RealtimeConfig {
            realtime_url: creds.realtime_url,
            realtime_token: creds.realtime_token,
            channel_name: creds.channel_name,
            user_id: user_id.clone(),
            session_token,
            supabase_anon_key: creds.supabase_anon_key,
            heartbeat_interval_secs: self.config.heartbeat_interval_secs as u64,
            sessions_dir: self.config.sessions_dir.clone(),
            version: self.config.version.clone(),
        };

        let mut client = RealtimeClient::new(config);

        // Create heartbeat channel
        let (heartbeat_tx, heartbeat_rx) =
            tokio::sync::mpsc::channel::<super::heartbeat::HeartbeatData>(10);

        // Create stream channel for agent output
        let (stream_tx, stream_rx) =
            tokio::sync::mpsc::channel::<(String, StreamChunkType, String)>(100);

        // Create sync trigger channel
        let (sync_trigger_tx, mut sync_trigger_rx) = tokio::sync::mpsc::channel::<()>(10);

        // Store channels for command handlers
        *self.stream_tx.write().await = Some(stream_tx);
        *self.sync_trigger_tx.write().await = Some(sync_trigger_tx);

        // Create command channel
        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel::<BackendCommand>(100);

        // Spawn command processor task
        let self_clone = self.clone();
        let command_handle = tokio::spawn(async move {
            tracing::info!("Command processor task started");
            while let Some(cmd) = command_rx.recv().await {
                tracing::info!("Processing Realtime command: {:?}", cmd);
                if let Err(e) = self_clone.handle_backend_command(cmd).await {
                    tracing::error!("Error handling backend command: {}", e);
                }
            }
            tracing::info!("Command processor task ended");
        });

        // Spawn heartbeat collector task
        let heartbeat_collector = Arc::clone(&self.heartbeat_collector);
        let heartbeat_interval = Duration::from_secs(self.config.heartbeat_interval_secs as u64);

        let heartbeat_handle = tokio::spawn(async move {
            tracing::info!(
                "Heartbeat collector task started, interval: {}s",
                heartbeat_interval.as_secs()
            );

            // Send initial heartbeat immediately
            if let Ok(data) = heartbeat_collector.write().await.collect().await {
                tracing::info!(
                    "Sending initial heartbeat with {} agents to frontend",
                    data.agents.len()
                );
                let _ = heartbeat_tx.send(data).await;
            }

            let mut interval = tokio::time::interval(heartbeat_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Ok(data) = heartbeat_collector.write().await.collect().await {
                            tracing::info!("Sending heartbeat with {} agents to frontend", data.agents.len());
                            let _ = heartbeat_tx.send(data).await;
                        }
                    }
                    Some(()) = sync_trigger_rx.recv() => {
                        tracing::info!("Sync trigger received, sending immediate heartbeat");
                        if let Ok(data) = heartbeat_collector.write().await.collect().await {
                            tracing::info!("Sending sync heartbeat with {} agents to frontend", data.agents.len());
                            let _ = heartbeat_tx.send(data).await;
                        }
                    }
                }
            }
        });

        // Connect and run
        client
            .connect(shutdown_rx, heartbeat_rx, stream_rx, command_tx)
            .await?;

        // Clean up
        *self.stream_tx.write().await = None;
        *self.sync_trigger_tx.write().await = None;
        heartbeat_handle.abort();
        command_handle.abort();

        Ok(())
    }

    /// Main polling loop
    async fn run_polling_loop(
        &mut self,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<()> {
        let heartbeat_interval = Duration::from_secs(self.config.heartbeat_interval_secs as u64);
        let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);

        // Initial heartbeat
        self.send_heartbeat_and_process_commands().await?;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Received shutdown signal");
                    break;
                }
                _ = heartbeat_timer.tick() => {
                    if let Err(e) = self.send_heartbeat_and_process_commands().await {
                        tracing::error!("Heartbeat failed: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Send heartbeat and process any commands returned
    async fn send_heartbeat_and_process_commands(&self) -> Result<()> {
        let session_token = self.session_token.read().await.clone().unwrap_or_default();

        let heartbeat_data = self.heartbeat_collector.write().await.collect().await?;

        // Drain queued command results
        let command_results: Vec<RemoteMessage> = {
            let mut queue = self.command_result_queue.write().await;
            let msgs = std::mem::take(&mut *queue);
            if !msgs.is_empty() {
                tracing::info!("Sending {} command results in heartbeat", msgs.len());
            }
            msgs
        };

        let heartbeat_body = serde_json::json!({
            "session_token": session_token,
            "agents": heartbeat_data.agents,
            "system_load": heartbeat_data.system_load,
            "messages": command_results,
            "hostname": gethostname::gethostname().to_string_lossy().to_string(),
            "os": std::env::consts::OS.to_string(),
            "version": self.config.version.clone(),
        });

        let url = format!("{}/api/remote/heartbeat", self.config.backend_url);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&heartbeat_body)
            .send()
            .await
            .context("Heartbeat request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Heartbeat failed: {} - {}", status, body);
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse heartbeat response")?;

        // Process any commands from the response
        if let Some(commands) = response_body.get("commands").and_then(|v| v.as_array()) {
            if !commands.is_empty() {
                tracing::info!(
                    "Received {} commands from backend: {:?}",
                    commands.len(),
                    commands
                );
            }
            for cmd_value in commands {
                match serde_json::from_value::<BackendCommand>(cmd_value.clone()) {
                    Ok(cmd) => {
                        tracing::info!("Processing backend command: {:?}", cmd);
                        if let Err(e) = self.handle_backend_command(cmd).await {
                            tracing::error!("Error handling backend command: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse backend command {:?}: {}", cmd_value, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a command from the backend
    async fn handle_backend_command(&self, cmd: BackendCommand) -> Result<()> {
        match cmd {
            BackendCommand::Ping { timestamp } => {
                self.queue_command_result_msg(RemoteMessage::Pong { timestamp })
                    .await?;
            }

            BackendCommand::RequestSync => {
                tracing::info!("Backend requested sync, triggering immediate heartbeat");
                if let Some(tx) = self.sync_trigger_tx.read().await.as_ref()
                    && let Err(e) = tx.send(()).await
                {
                    tracing::error!("Failed to trigger sync: {}", e);
                }
            }

            BackendCommand::Subscribe { agent_id } => {
                tracing::info!("Web client subscribed to agent: {}", agent_id);
                self.subscriptions.write().await.insert(agent_id.clone());
                self.start_agent_reader(&agent_id).await;
                self.request_history_sync(&agent_id).await;
            }

            BackendCommand::Unsubscribe { agent_id } => {
                tracing::info!("Web client unsubscribed from agent: {}", agent_id);
                self.subscriptions.write().await.remove(&agent_id);
                self.stop_agent_reader(&agent_id).await;
            }

            BackendCommand::SendInput {
                command_id,
                agent_id,
                content,
            } => {
                let result = self.relay_input_to_agent(&agent_id, &content).await;
                self.queue_command_result(&command_id, result).await?;
            }

            BackendCommand::SlashCommand {
                command_id,
                agent_id,
                command,
                args,
            } => {
                let result = self
                    .relay_slash_command_to_agent(&agent_id, &command, &args)
                    .await;
                self.queue_command_result(&command_id, result).await?;
            }

            BackendCommand::CancelOperation {
                command_id,
                agent_id,
            } => {
                let result = self.relay_cancel_to_agent(&agent_id).await;
                self.queue_command_result(&command_id, result).await?;
            }

            BackendCommand::SpawnAgent {
                command_id,
                model,
                working_directory,
            } => {
                let result = self.spawn_new_agent(model, working_directory).await;
                self.queue_command_result(&command_id, result).await?;
            }

            BackendCommand::Disconnect { reason } => {
                tracing::info!("Backend requested disconnect: {}", reason);
            }

            // Attachment Commands
            BackendCommand::AttachmentUpload {
                command_id,
                agent_id,
                attachment_id,
                filename,
                mime_type,
                size,
                compressed,
                compression_algorithm,
                chunks_total,
            } => {
                tracing::info!(
                    "Starting attachment upload: {} ({} bytes, {} chunks)",
                    filename,
                    size,
                    chunks_total
                );

                let result = self
                    .attachment_receiver
                    .start_upload(
                        command_id.clone(),
                        agent_id,
                        attachment_id.clone(),
                        filename,
                        mime_type,
                        size,
                        compressed,
                        compression_algorithm,
                        chunks_total,
                    )
                    .await;

                match result {
                    Ok(()) => {
                        self.queue_command_result(
                            &command_id,
                            Ok(serde_json::json!({
                                "attachment_id": attachment_id,
                                "status": "started"
                            })),
                        )
                        .await?;
                    }
                    Err(e) => {
                        self.queue_command_result(&command_id, Err(e)).await?;
                    }
                }
            }

            BackendCommand::AttachmentChunk {
                attachment_id,
                chunk_index,
                data,
                is_final,
            } => {
                tracing::debug!(
                    "Receiving attachment chunk: {} (index {})",
                    attachment_id,
                    chunk_index
                );

                match self
                    .attachment_receiver
                    .receive_chunk(&attachment_id, chunk_index, &data, is_final)
                    .await
                {
                    Ok(all_received) => {
                        if all_received {
                            tracing::info!("All chunks received for attachment: {}", attachment_id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to receive chunk for {}: {}", attachment_id, e);
                        self.attachment_receiver.cancel_upload(&attachment_id).await;
                    }
                }
            }

            BackendCommand::AttachmentComplete {
                attachment_id,
                checksum,
            } => {
                tracing::info!(
                    "Completing attachment upload: {} (checksum: {})",
                    attachment_id,
                    checksum
                );

                match self
                    .attachment_receiver
                    .complete_upload(&attachment_id, &checksum)
                    .await
                {
                    Ok(file_path) => {
                        let path_str = file_path.display().to_string();
                        tracing::info!("Attachment saved to: {}", path_str);

                        self.queue_command_result_msg(RemoteMessage::AttachmentReceived {
                            attachment_id: attachment_id.clone(),
                            success: true,
                            file_path: Some(path_str),
                            error: None,
                        })
                        .await?;
                    }
                    Err(e) => {
                        tracing::error!("Failed to complete attachment: {}", e);

                        self.queue_command_result_msg(RemoteMessage::AttachmentReceived {
                            attachment_id: attachment_id.clone(),
                            success: false,
                            file_path: None,
                            error: Some(e.to_string()),
                        })
                        .await?;
                    }
                }
            }

            BackendCommand::Authenticated { .. } | BackendCommand::AuthenticationFailed { .. } => {
                tracing::warn!("Unexpected auth message after authentication");
            }
        }

        Ok(())
    }

    /// Queue a command result to send with the next heartbeat
    async fn queue_command_result(
        &self,
        command_id: &str,
        result: Result<serde_json::Value>,
    ) -> Result<()> {
        let msg = match result {
            Ok(value) => RemoteMessage::CommandResult {
                command_id: command_id.to_string(),
                success: true,
                result: Some(value),
                error: None,
            },
            Err(e) => RemoteMessage::CommandResult {
                command_id: command_id.to_string(),
                success: false,
                result: None,
                error: Some(e.to_string()),
            },
        };

        self.queue_command_result_msg(msg).await
    }

    /// Start an agent reader task to stream output back to backend
    async fn start_agent_reader(&self, agent_id: &str) {
        tracing::info!("start_agent_reader called for agent: {}", agent_id);

        // Check if already reading
        if self.subscription_tasks.read().await.contains_key(agent_id) {
            tracing::debug!("Agent reader already running for {}", agent_id);
            return;
        }

        let sessions_dir = &self.config.sessions_dir;

        // Connect using bridge-internal IPC with injected sessions_dir
        tracing::info!("Connecting to agent {} via IPC...", agent_id);
        let mut conn = match IpcConnection::connect_to_agent(sessions_dir, agent_id).await {
            Ok(c) => {
                tracing::info!("Successfully connected to agent {}", agent_id);
                c
            }
            Err(e) => {
                tracing::error!(
                    "Failed to connect to agent {} for streaming: {}",
                    agent_id,
                    e
                );
                return;
            }
        };

        // Read session token using bridge-internal parameterized function
        let session_token = match crate::ipc::socket::read_session_token(sessions_dir, agent_id) {
            Ok(Some(token)) => token,
            Ok(None) => {
                tracing::error!("No session token found for agent {}", agent_id);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to read session token for agent {}: {}", agent_id, e);
                return;
            }
        };

        // Perform handshake
        tracing::info!("Sending handshake with token to agent {}", agent_id);
        let handshake = Handshake::reattach(agent_id.to_string(), session_token);
        if let Err(e) = conn.writer.write(&handshake).await {
            tracing::error!("Failed to send handshake to agent {}: {}", agent_id, e);
            return;
        }

        // Wait for handshake response
        tracing::info!("Waiting for handshake response from agent {}", agent_id);
        let response: HandshakeResponse = match conn.reader.read().await {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::error!("Agent {} closed connection during handshake", agent_id);
                return;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read handshake response from agent {}: {}",
                    agent_id,
                    e
                );
                return;
            }
        };

        if !response.accepted {
            tracing::error!(
                "Handshake rejected by agent {}: {:?}",
                agent_id,
                response.error
            );
            return;
        }
        tracing::info!("Handshake accepted by agent {}", agent_id);

        // Request conversation sync
        tracing::info!("Sending SyncRequest to agent {}", agent_id);
        if let Err(e) = conn.writer.write(&ViewerMessage::SyncRequest).await {
            tracing::error!("Failed to send SyncRequest to agent {}: {}", agent_id, e);
        } else {
            tracing::info!("SyncRequest sent successfully to agent {}", agent_id);
        }

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let agent_id_owned = agent_id.to_string();
        let subscriptions = Arc::clone(&self.subscriptions);
        let stream_tx = Arc::clone(&self.stream_tx);

        // Create channel for sending messages to this agent
        let (writer_tx, mut writer_rx) = tokio::sync::mpsc::channel::<ViewerMessage>(32);

        // Spawn reader/writer task
        tracing::info!("Spawning reader task for agent {}", agent_id);
        let task_handle = tokio::spawn(async move {
            tracing::info!("Agent reader task started for {}", agent_id_owned);
            let (mut reader, mut writer) = (conn.reader, conn.writer);
            let mut cancel_rx = cancel_rx;

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        tracing::debug!("Agent reader for {} cancelled", agent_id_owned);
                        break;
                    }
                    Some(msg) = writer_rx.recv() => {
                        tracing::info!("Sending ViewerMessage to agent {}: {:?}", agent_id_owned, std::mem::discriminant(&msg));
                        if let Err(e) = writer.write(&msg).await {
                            tracing::error!("Failed to send message to agent {}: {}", agent_id_owned, e);
                            break;
                        }
                    }
                    result = reader.read::<AgentMessage>() => {
                        match result {
                            Ok(Some(msg)) => {
                                tracing::info!("Received AgentMessage from {}: {:?}", agent_id_owned, std::mem::discriminant(&msg));

                                if !subscriptions.read().await.contains(&agent_id_owned) {
                                    tracing::debug!("Agent {} no longer subscribed, stopping reader", agent_id_owned);
                                    break;
                                }

                                if let Some((chunk_type, content)) = convert_agent_message_to_stream(&msg) {
                                    tracing::info!("Sending stream message for {}: type={:?}, content_len={}",
                                        agent_id_owned, chunk_type, content.len());

                                    if let Some(tx) = stream_tx.read().await.as_ref() {
                                        if let Err(e) = tx.send((agent_id_owned.clone(), chunk_type, content)).await {
                                            tracing::error!("Failed to send stream via Realtime: {}", e);
                                        } else {
                                            tracing::debug!("Stream message sent via Realtime");
                                        }
                                    } else {
                                        tracing::warn!("Realtime stream channel not available, dropping message");
                                    }
                                } else {
                                    tracing::debug!("AgentMessage not converted to stream chunk (filtered out)");
                                }
                            }
                            Ok(None) => {
                                tracing::info!("Agent {} disconnected", agent_id_owned);
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Error reading from agent {}: {}", agent_id_owned, e);
                                break;
                            }
                        }
                    }
                }
            }
            tracing::info!("Agent reader task ended for {}", agent_id_owned);
        });

        // Store subscription with writer channel
        self.subscription_tasks.write().await.insert(
            agent_id.to_string(),
            AgentSubscription {
                cancel_tx,
                task_handle,
                writer_tx,
            },
        );

        tracing::info!("Started agent reader for {}", agent_id);
    }

    /// Stop an agent reader task
    async fn stop_agent_reader(&self, agent_id: &str) {
        if let Some(sub) = self.subscription_tasks.write().await.remove(agent_id) {
            let _ = sub.cancel_tx.send(());
            sub.task_handle.abort();
            tracing::info!("Stopped agent reader for {}", agent_id);
        }
    }

    /// Request history sync from an agent
    async fn request_history_sync(&self, agent_id: &str) {
        tracing::info!("Requesting history sync for agent: {}", agent_id);

        let writer_tx = {
            let tasks = self.subscription_tasks.read().await;
            tasks.get(agent_id).map(|sub| sub.writer_tx.clone())
        };

        let writer_tx = match writer_tx {
            Some(tx) => tx,
            None => {
                tracing::warn!(
                    "No active subscription for agent {}, cannot request history sync",
                    agent_id
                );
                return;
            }
        };

        if let Err(e) = writer_tx.send(ViewerMessage::SyncRequest).await {
            tracing::error!("Failed to send SyncRequest to agent {}: {}", agent_id, e);
            return;
        }

        tracing::info!(
            "SyncRequest sent to agent {} via persistent connection",
            agent_id
        );
    }

    /// Relay user input to an agent
    async fn relay_input_to_agent(
        &self,
        agent_id: &str,
        content: &str,
    ) -> Result<serde_json::Value> {
        let writer_tx = {
            let tasks = self.subscription_tasks.read().await;
            tasks.get(agent_id).map(|sub| sub.writer_tx.clone())
        };

        let writer_tx = writer_tx
            .ok_or_else(|| anyhow::anyhow!("No active subscription for agent {}", agent_id))?;

        let msg = ViewerMessage::UserInput {
            content: content.to_string(),
            context_files: vec![],
        };

        writer_tx
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send input to agent: {}", e))?;

        Ok(serde_json::json!({
            "agent_id": agent_id,
            "input_sent": true,
        }))
    }

    /// Relay slash command to an agent
    async fn relay_slash_command_to_agent(
        &self,
        agent_id: &str,
        command: &str,
        args: &[String],
    ) -> Result<serde_json::Value> {
        let writer_tx = {
            let tasks = self.subscription_tasks.read().await;
            tasks.get(agent_id).map(|sub| sub.writer_tx.clone())
        };

        let writer_tx = writer_tx
            .ok_or_else(|| anyhow::anyhow!("No active subscription for agent {}", agent_id))?;

        let msg = ViewerMessage::SlashCommand {
            command: command.to_string(),
            args: args.to_vec(),
        };

        writer_tx
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send command to agent: {}", e))?;

        Ok(serde_json::json!({
            "agent_id": agent_id,
            "command": command,
            "command_sent": true,
        }))
    }

    /// Relay cancel to an agent
    async fn relay_cancel_to_agent(&self, agent_id: &str) -> Result<serde_json::Value> {
        let writer_tx = {
            let tasks = self.subscription_tasks.read().await;
            tasks.get(agent_id).map(|sub| sub.writer_tx.clone())
        };

        let writer_tx = writer_tx
            .ok_or_else(|| anyhow::anyhow!("No active subscription for agent {}", agent_id))?;

        let msg = ViewerMessage::Cancel;
        writer_tx
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send cancel to agent: {}", e))?;

        Ok(serde_json::json!({
            "agent_id": agent_id,
            "cancel_sent": true,
        }))
    }

    /// Spawn a new agent (session)
    ///
    /// Delegates to the injected `AgentSpawner` trait for actual process creation.
    /// The bridge handles session ID generation and basic path validation.
    async fn spawn_new_agent(
        &self,
        model: Option<String>,
        working_directory: Option<String>,
    ) -> Result<serde_json::Value> {
        let agent_spawner = self
            .agent_spawner
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No agent spawner configured"))?;

        // Validate and canonicalize working_directory
        let validated_working_dir = if let Some(ref dir) = working_directory {
            let path = PathBuf::from(dir);

            if !path.exists() {
                anyhow::bail!("Working directory does not exist: {}", dir);
            }
            if !path.is_dir() {
                anyhow::bail!("Working directory is not a directory: {}", dir);
            }

            let canonical = path
                .canonicalize()
                .context(format!("Failed to canonicalize working directory: {}", dir))?;

            Some(canonical)
        } else {
            None
        };

        // Generate a cryptographically secure session ID
        use rand::Rng;
        let mut random_bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut random_bytes);
        let session_id = format!("session-{}", hex::encode(random_bytes));

        // Delegate to spawner
        tracing::info!("Spawning new session via remote: {}", session_id);
        let socket_path = agent_spawner
            .spawn_agent(&session_id, model, validated_working_dir)
            .await?;

        Ok(serde_json::json!({
            "session_id": session_id,
            "socket_path": socket_path.to_string_lossy(),
            "status": "spawned",
        }))
    }
}

/// Convert an AgentMessage to a stream chunk (chunk_type, content)
fn convert_agent_message_to_stream(msg: &AgentMessage) -> Option<(StreamChunkType, String)> {
    match msg {
        AgentMessage::StreamChunk { text } => Some((StreamChunkType::Text, text.clone())),
        AgentMessage::StreamEnd { .. } => Some((StreamChunkType::Complete, String::new())),
        AgentMessage::ToolCallStart { name, input, .. } => {
            let content = format!(
                "Tool: {} - {}",
                name,
                serde_json::to_string(input).unwrap_or_default()
            );
            Some((StreamChunkType::ToolCall, content))
        }
        AgentMessage::ToolProgress { name, message, .. } => {
            Some((StreamChunkType::Text, format!("[{}] {}", name, message)))
        }
        AgentMessage::ToolResult {
            name,
            output,
            error,
            ..
        } => {
            let content = if let Some(err) = error {
                format!("{}: Error: {}", name, err)
            } else if let Some(out) = output {
                format!("{}: {}", name, out)
            } else {
                format!("{}: (no output)", name)
            };
            Some((StreamChunkType::ToolResult, content))
        }
        AgentMessage::Error { message, .. } => Some((StreamChunkType::Error, message.clone())),
        AgentMessage::StatusUpdate { status } => Some((StreamChunkType::System, status.clone())),
        AgentMessage::MessageAdded { message } => {
            if message.role == "user" {
                Some((StreamChunkType::UserInput, message.content.clone()))
            } else {
                None
            }
        }
        AgentMessage::ConversationSync { messages, .. } => {
            let history_json = serde_json::to_string(messages).unwrap_or_else(|_| "[]".to_string());
            Some((StreamChunkType::History, history_json))
        }
        AgentMessage::SlashCommandResult {
            command,
            success,
            output,
            action_taken,
            error,
            blocked,
            ..
        } => {
            let content = if *blocked {
                format!(
                    "Command /{} blocked: {}",
                    command,
                    error.as_deref().unwrap_or("security policy")
                )
            } else if *success {
                if let Some(out) = output {
                    format!("/{}: {}", command, out)
                } else if let Some(action) = action_taken {
                    format!("/{}: {}", command, action)
                } else {
                    format!("/{}: done", command)
                }
            } else {
                format!(
                    "/{} failed: {}",
                    command,
                    error.as_deref().unwrap_or("unknown error")
                )
            };
            Some((StreamChunkType::System, content))
        }
        // Not exposed to remote bridge
        AgentMessage::TaskUpdate { .. }
        | AgentMessage::Toast { .. }
        | AgentMessage::SealStatus { .. }
        | AgentMessage::LockResult { .. }
        | AgentMessage::LockReleased { .. }
        | AgentMessage::LockStatus { .. }
        | AgentMessage::LockChanged { .. }
        | AgentMessage::Ack { .. }
        | AgentMessage::Exiting { .. }
        | AgentMessage::AgentSpawned { .. }
        | AgentMessage::AgentList { .. }
        | AgentMessage::AgentExiting { .. }
        | AgentMessage::ParentSignalReceived { .. }
        | AgentMessage::PlanModeEntered { .. }
        | AgentMessage::PlanModeExited { .. }
        | AgentMessage::PlanModeSync { .. }
        | AgentMessage::PlanModeMessageAdded { .. }
        | AgentMessage::PlanModeStreamChunk { .. }
        | AgentMessage::PlanModeStreamEnd { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_config_default() {
        let config = BridgeConfig::default();
        assert!(config.backend_url.starts_with("https://"));
        assert_eq!(config.heartbeat_interval_secs, 5);
        assert_eq!(config.version, "unknown");
    }

    #[tokio::test]
    async fn test_bridge_state() {
        let config = BridgeConfig::default();
        let bridge = RemoteBridge::new(config, None);

        assert_eq!(bridge.state().await, BridgeState::Disconnected);
        assert!(!bridge.is_ready().await);
        assert!(bridge.user_id().await.is_none());
    }

    #[tokio::test]
    async fn test_bridge_command_result_queue() {
        let config = BridgeConfig::default();
        let bridge = RemoteBridge::new(config, None);

        // Queue should start empty
        assert!(bridge.command_result_queue.read().await.is_empty());

        // Queue a command result message
        bridge
            .queue_command_result_msg(RemoteMessage::Pong { timestamp: 12345 })
            .await
            .unwrap();

        // Queue should have one message
        assert_eq!(bridge.command_result_queue.read().await.len(), 1);
    }
}
