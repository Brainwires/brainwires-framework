//! Agent Process
//!
//! The main agent process that runs in the background, maintains session state,
//! and communicates with TUI viewers via IPC.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, RwLock};

use crate::auth::SessionManager;
use crate::commands::executor::{CommandAction, CommandResult};
use crate::config::ConfigManager;
use futures::StreamExt as FuturesStreamExt;
use serde_json::json;
use brainwires::agent_network::ipc::{
    AgentMessage, Handshake, HandshakeResponse, IpcConnection, LockChangeType, LockInfo,
    ResourceLockType, ViewerMessage,
};
use crate::ipc::get_agent_socket_path;
use crate::mdap::MdapConfig;
use crate::types::tool::{ToolContext, ToolUse};

use super::AgentState;

/// Agent process that manages a single session
pub struct AgentProcess {
    /// Agent state
    state: Arc<RwLock<AgentState>>,
    /// Socket path for IPC
    socket_path: PathBuf,
    /// Broadcast channel for sending updates to viewer
    update_tx: broadcast::Sender<AgentMessage>,
}

impl AgentProcess {
    /// Create a new agent process
    pub async fn new(
        session_id: Option<String>,
        model: Option<String>,
        mdap_config: Option<MdapConfig>,
    ) -> Result<Self> {
        let state = AgentState::new(session_id, model, mdap_config).await?;
        let socket_path = get_agent_socket_path(&state.session_id)?;

        // Create broadcast channel for updates
        let (update_tx, _) = broadcast::channel(256);

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            socket_path,
            update_tx,
        })
    }

    /// Get the session ID
    pub async fn session_id(&self) -> String {
        self.state.read().await.session_id.clone()
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Run the agent process
    ///
    /// This listens for viewer connections and handles IPC messages.
    /// The process runs until it receives an Exit command or exit_when_done triggers.
    pub async fn run(self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Remove stale socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Generate and store session token for secure reattachment
        let session_id = self.state.read().await.session_id.clone();
        let session_token = brainwires::agent_network::ipc::socket::generate_session_token();
        crate::ipc::write_session_token(&session_id, &session_token)?;
        tracing::info!("Session token generated and saved with 0600 permissions");

        // Bind to socket
        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind to socket: {}", self.socket_path.display()))?;

        // Set socket permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o600))?;
            tracing::debug!("Socket permissions set to 0600");
        }

        tracing::info!("Agent listening on: {}", self.socket_path.display());

        // Initialize MCP servers
        {
            let mut state = self.state.write().await;
            state.initialize_mcp().await;
        }

        // Check for pending request and process it
        // This handles the case where user backgrounded before AI could respond
        {
            let should_process = {
                let state = self.state.read().await;
                state.has_pending_request
            };

            if should_process {
                tracing::info!("Processing pending user request...");
                let state_clone = Arc::clone(&self.state);
                let update_tx = self.update_tx.clone();

                // Process the pending request in background
                tokio::spawn(async move {
                    if let Err(e) = process_pending_request(state_clone, update_tx).await {
                        tracing::error!("Failed to process pending request: {}", e);
                    }
                });
            }
        }

        // Main event loop
        loop {
            tokio::select! {
                // Accept new viewer connections
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _addr)) => {
                            let conn = IpcConnection::from_stream(stream);
                            let state = Arc::clone(&self.state);
                            let update_tx = self.update_tx.clone();

                            // Handle this viewer connection
                            tokio::spawn(async move {
                                if let Err(e) = handle_viewer_connection(conn, state, update_tx).await {
                                    tracing::error!("Viewer connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }

                // Check for exit_when_done condition
                _ = check_exit_condition(Arc::clone(&self.state)) => {
                    tracing::info!("Exit condition met, shutting down agent");
                    break;
                }
            }
        }

        // Phase 5: Smart cascade shutdown - notify children before exit
        let session_id = {
            let state = self.state.read().await;
            state.session_id.clone()
        };

        // Notify children that we're exiting (ShutdownIfIdle behavior)
        if let Ok(children) = crate::ipc::get_child_agents(&session_id) {
            for child in children {
                if crate::ipc::is_agent_alive(&child.session_id).await {
                    // Read child's session token for authenticated connection
                    let child_token = match crate::ipc::read_session_token(&child.session_id) {
                        Ok(Some(token)) => token,
                        _ => {
                            tracing::warn!("No session token for child {}", child.session_id);
                            continue;
                        }
                    };

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
                                let signal_msg = ViewerMessage::ParentSignal {
                                    signal: ParentSignalType::ParentExiting,
                                    parent_session_id: session_id.clone(),
                                };
                                let _ = conn.writer.write(&signal_msg).await;
                                tracing::info!("Sent ParentExiting signal to child {}", child.session_id);
                            }
                        }
                    }
                }
            }
        }

        // Use cleanup_agent helper which handles socket and metadata
        let _ = crate::agent::cleanup_agent(&session_id, false).await;

        Ok(())
    }
}

/// Check if the agent should exit (exit_when_done is true and not busy)
async fn check_exit_condition(state: Arc<RwLock<AgentState>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let state = state.read().await;
        if state.exit_when_done && !state.is_busy {
            return;
        }
    }
}

/// Handle a viewer connection
async fn handle_viewer_connection(
    conn: IpcConnection,
    state: Arc<RwLock<AgentState>>,
    update_tx: broadcast::Sender<AgentMessage>,
) -> Result<()> {
    let (mut reader, mut writer) = conn.split();

    // Perform handshake
    let handshake: Handshake = reader.read().await?
        .ok_or_else(|| anyhow::anyhow!("Connection closed during handshake"))?;

    let session_id = state.read().await.session_id.clone();

    // Validate protocol version (allow version 1 for backwards compatibility during transition)
    if handshake.version != Handshake::PROTOCOL_VERSION && handshake.version != 1 {
        let response = HandshakeResponse {
            accepted: false,
            session_id: String::new(),
            session_token: None,
            error: Some(format!(
                "Protocol version mismatch: expected {}, got {}",
                Handshake::PROTOCOL_VERSION,
                handshake.version
            )),
        };
        writer.write(&response).await?;
        return Ok(());
    }

    // For reattach requests, validate the session token
    if handshake.is_reattach {
        let provided_token = handshake.session_token.as_deref().unwrap_or("");

        if provided_token.is_empty() {
            tracing::warn!("Reattach attempt without session token from {}",
                handshake.session_id.as_deref().unwrap_or("unknown"));
            let response = HandshakeResponse {
                accepted: false,
                session_id: String::new(),
                session_token: None,
                error: Some("Session token required for reattachment".to_string()),
            };
            writer.write(&response).await?;
            return Ok(());
        }

        if !crate::ipc::validate_session_token(&session_id, provided_token) {
            tracing::warn!("Invalid session token for reattach to session {}", session_id);
            let response = HandshakeResponse {
                accepted: false,
                session_id: String::new(),
                session_token: None,
                error: Some("Invalid session token".to_string()),
            };
            writer.write(&response).await?;
            return Ok(());
        }

        tracing::info!("Reattach authentication successful for session {}", session_id);
    }

    // Read the stored token to return to new sessions
    let stored_token = crate::ipc::read_session_token(&session_id).ok().flatten();

    // Send handshake response with session token for new sessions
    let response = HandshakeResponse {
        accepted: true,
        session_id: session_id.clone(),
        // Only return the token for new sessions (not reattach)
        session_token: if !handshake.is_reattach { stored_token } else { None },
        error: None,
    };
    writer.write(&response).await?;

    // Send initial sync
    {
        let state = state.read().await;
        let sync_msg = state.create_sync_message();
        writer.write(&sync_msg).await?;

        // Send task update
        let task_msg = state.create_task_update_message();
        writer.write(&task_msg).await?;

        // Send SEAL status
        let seal_msg = state.get_seal_status();
        writer.write(&seal_msg).await?;
    }

    // Subscribe to updates
    let mut update_rx = update_tx.subscribe();

    // Message handling loop
    loop {
        tokio::select! {
            // Handle incoming messages from viewer
            msg = reader.read::<ViewerMessage>() => {
                match msg {
                    Ok(Some(viewer_msg)) => {
                        match handle_viewer_message(viewer_msg, Arc::clone(&state), &update_tx).await {
                            Ok(should_exit) => {
                                if should_exit {
                                    // Send exiting message
                                    let exit_msg = AgentMessage::Exiting {
                                        reason: "Exit requested".to_string(),
                                    };
                                    let _ = writer.write(&exit_msg).await;
                                    return Ok(());
                                }
                            }
                            Err(e) => {
                                let error_msg = AgentMessage::Error {
                                    message: e.to_string(),
                                    fatal: false,
                                };
                                let _ = writer.write(&error_msg).await;
                            }
                        }
                    }
                    Ok(None) => {
                        // Connection closed
                        tracing::info!("Viewer disconnected");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("Read error: {}", e);
                        return Err(e);
                    }
                }
            }

            // Forward updates to viewer
            update = update_rx.recv() => {
                match update {
                    Ok(msg) => {
                        if let Err(e) = writer.write(&msg).await {
                            tracing::error!("Write error: {}", e);
                            return Err(e);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Viewer lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Channel closed, agent is shutting down
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Handle a message from the viewer
///
/// Returns Ok(true) if the agent should exit, Ok(false) to continue.
async fn handle_viewer_message(
    msg: ViewerMessage,
    state: Arc<RwLock<AgentState>>,
    update_tx: &broadcast::Sender<AgentMessage>,
) -> Result<bool> {
    match msg {
        ViewerMessage::UserInput { content, context_files } => {
            // Preprocess through SEAL and add user message
            let (_resolved_content, user_message, provider, conversation_history, tools, model, tool_executor, working_directory) = {
                let mut state = state.write().await;

                // Process context files if provided
                let mut context_content = String::new();
                for file_path in &context_files {
                    match std::fs::read_to_string(file_path) {
                        Ok(file_content) => {
                            context_content.push_str(&format!(
                                "\n--- Context File: {} ---\n{}\n--- End of {} ---\n",
                                file_path,
                                file_content,
                                file_path
                            ));
                            tracing::info!("Loaded context file: {}", file_path);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read context file {}: {}", file_path, e);
                        }
                    }
                }

                // Combine content with context files
                let full_content = if context_content.is_empty() {
                    content.clone()
                } else {
                    format!("{}\n\n# Referenced Files\n{}", content, context_content)
                };

                let resolved_content = state.seal_preprocess(&full_content);
                state.add_user_message(resolved_content.clone());
                state.is_busy = true;
                state.status = "Streaming response...".to_string();

                // Combine core tools with MCP tools
                let mut all_tools = state.tools.clone();
                all_tools.extend(state.mcp_tools.clone());

                // Get user message we just added for broadcasting
                let user_msg = state.messages.last().cloned();

                (
                    resolved_content,
                    user_msg,
                    state.provider.clone(),
                    state.conversation_history.clone(),
                    all_tools,
                    state.model.clone(),
                    state.tool_executor.clone(),
                    state.working_directory.clone(),
                )
            };

            // Send acknowledgment and status update
            let _ = update_tx.send(AgentMessage::Ack {
                command: "user_input".to_string(),
            });

            // Broadcast user message to all viewers (so GUI input shows in TUI)
            if let Some(msg) = user_message {
                let _ = update_tx.send(AgentMessage::MessageAdded { message: msg });
            }

            let _ = update_tx.send(AgentMessage::StatusUpdate {
                status: "Streaming response...".to_string(),
            });

            // Stream AI response in background with tool execution support
            let state_clone = Arc::clone(&state);
            let update_tx_clone = update_tx.clone();

            tokio::spawn(async move {
                let result = stream_with_tool_execution(
                    provider,
                    conversation_history,
                    tools,
                    model.clone(),
                    tool_executor,
                    working_directory,
                    update_tx_clone.clone(),
                    state_clone.clone(),
                ).await;

                match result {
                    Ok(full_response) => {
                        // Finalize: add assistant message and update state
                        let mut state = state_clone.write().await;
                        state.add_assistant_message(full_response.clone());
                        state.is_busy = false;
                        state.status = format!("Ready - Model: {}", model);

                        // Send message added notification
                        if let Some(last_msg) = state.messages.last() {
                            let _ = update_tx_clone.send(AgentMessage::MessageAdded {
                                message: last_msg.clone(),
                            });
                        }

                        let _ = update_tx_clone.send(AgentMessage::StatusUpdate {
                            status: state.status.clone(),
                        });

                        // Process any queued messages
                        let queued_count = state.message_queue.len();
                        if queued_count > 0 {
                            tracing::info!("Processing {} queued message(s)", queued_count);
                            drop(state); // Release lock before spawning

                            // Process queued messages sequentially
                            process_queued_messages(
                                state_clone.clone(),
                                update_tx_clone.clone(),
                            ).await;
                        }
                    }
                    Err(e) => {
                        let mut state = state_clone.write().await;
                        state.is_busy = false;
                        state.status = format!("Error: {}", e);

                        let _ = update_tx_clone.send(AgentMessage::Error {
                            message: e.to_string(),
                            fatal: false,
                        });
                        let _ = update_tx_clone.send(AgentMessage::StatusUpdate {
                            status: state.status.clone(),
                        });
                    }
                }
            });

            Ok(false)
        }

        ViewerMessage::Cancel => {
            let mut state = state.write().await;
            if let Some(token) = state.cancellation_token.take() {
                token.cancel();
            }
            state.is_busy = false;
            state.status = "Cancelled".to_string();

            let ack = AgentMessage::Ack {
                command: "cancel".to_string(),
            };
            let _ = update_tx.send(ack);

            Ok(false)
        }

        ViewerMessage::SyncRequest => {
            let state = state.read().await;
            let sync_msg = state.create_sync_message();
            let _ = update_tx.send(sync_msg);

            let task_msg = state.create_task_update_message();
            let _ = update_tx.send(task_msg);

            Ok(false)
        }

        ViewerMessage::Detach { exit_when_done } => {
            {
                let mut state = state.write().await;
                state.exit_when_done = exit_when_done;
            }

            let ack = AgentMessage::Ack {
                command: "detach".to_string(),
            };
            let _ = update_tx.send(ack);

            // Connection will close but agent continues
            Ok(false)
        }

        ViewerMessage::Exit => {
            let ack = AgentMessage::Ack {
                command: "exit".to_string(),
            };
            let _ = update_tx.send(ack);

            // Signal agent to exit by setting exit_when_done
            // This will cause the main run() loop to break once the agent is not busy
            {
                let mut state_guard = state.write().await;
                state_guard.exit_when_done = true;
                // Force is_busy to false since this is an explicit exit request
                state_guard.is_busy = false;
            }

            // Also signal this connection to close
            Ok(true)
        }

        ViewerMessage::SlashCommand { command, args } => {
            tracing::info!("Slash command from remote: /{} {:?}", command, args);

            // Check if command is blocked by remote settings
            let remote_settings = ConfigManager::new()
                .map(|cm| cm.get().remote.clone())
                .unwrap_or_default();

            if remote_settings.is_command_blocked(&command) {
                tracing::warn!("Blocked remote command: /{}", command);
                let result_msg = AgentMessage::SlashCommandResult {
                    command: command.clone(),
                    success: false,
                    output: None,
                    action_taken: None,
                    error: Some(format!("Command '{}' is blocked for remote execution", command)),
                    blocked: true,
                };
                let _ = update_tx.send(result_msg);
                return Ok(false);
            }

            // Log warning for warned commands but continue execution
            if remote_settings.is_command_warned(&command) {
                tracing::warn!("Executing warned remote command: /{}", command);
            }

            // Execute the command using the command executor
            let result = {
                let state_guard = state.read().await;
                state_guard.command_executor.execute(&command, &args)
            };

            let result_msg = match result {
                Ok(CommandResult::Message(msg)) => {
                    tracing::info!("Slash command produced message: {}", &msg[..msg.len().min(100)]);
                    // Add message to conversation as user input (will trigger AI response)
                    {
                        let mut state_guard = state.write().await;
                        let expanded_message = crate::types::message::Message {
                            role: crate::types::message::Role::User,
                            content: crate::types::message::MessageContent::Text(msg.clone()),
                            name: None,
                            metadata: None,
                        };
                        state_guard.conversation_history.push(expanded_message);
                    }
                    AgentMessage::SlashCommandResult {
                        command: command.clone(),
                        success: true,
                        output: Some(msg),
                        action_taken: Some("message_added".to_string()),
                        error: None,
                        blocked: false,
                    }
                }
                Ok(CommandResult::Help(lines)) => {
                    let output = lines.join("\n");
                    AgentMessage::SlashCommandResult {
                        command: command.clone(),
                        success: true,
                        output: Some(output),
                        action_taken: Some("help_displayed".to_string()),
                        error: None,
                        blocked: false,
                    }
                }
                Ok(CommandResult::ActionWithMessage(action, msg)) => {
                    // Handle action then add message
                    let action_name = format!("{:?}", action);
                    tracing::info!("Slash command action with message: {}", action_name);
                    {
                        let mut state_guard = state.write().await;
                        let expanded_message = crate::types::message::Message {
                            role: crate::types::message::Role::User,
                            content: crate::types::message::MessageContent::Text(msg.clone()),
                            name: None,
                            metadata: None,
                        };
                        state_guard.conversation_history.push(expanded_message);
                    }
                    AgentMessage::SlashCommandResult {
                        command: command.clone(),
                        success: true,
                        output: Some(msg),
                        action_taken: Some(format!("action_with_message: {}", action_name)),
                        error: None,
                        blocked: false,
                    }
                }
                Ok(CommandResult::Action(action)) => {
                    // Handle the action and return result
                    let action_name = format!("{:?}", action);
                    tracing::info!("Slash command action: {}", action_name);

                    // Execute certain safe actions, block dangerous ones
                    let (success, action_taken, error) = match &action {
                        // Conversation actions that are safe
                        CommandAction::ClearHistory => {
                            let mut state_guard = state.write().await;
                            state_guard.messages.clear();
                            state_guard.conversation_history.clear();
                            (true, Some("history_cleared".to_string()), None)
                        }
                        CommandAction::ShowStatus => {
                            let state_guard = state.read().await;
                            let status = format!(
                                "Session: {}\nModel: {}\nMessages: {}\nStatus: {}",
                                state_guard.session_id,
                                state_guard.model,
                                state_guard.messages.len(),
                                state_guard.status
                            );
                            let _ = update_tx.send(AgentMessage::StatusUpdate { status: status.clone() });
                            (true, Some(format!("status_shown: {}", status)), None)
                        }
                        CommandAction::Exit => {
                            // Exit is allowed but flagged as warned
                            let mut state_guard = state.write().await;
                            state_guard.exit_when_done = true;
                            state_guard.is_busy = false;
                            (true, Some("exit_requested".to_string()), None)
                        }
                        CommandAction::SwitchModel(model_name) => {
                            // Model switch - update state
                            let mut state_guard = state.write().await;
                            state_guard.model = model_name.clone();
                            (true, Some(format!("model_switched: {}", model_name)), None)
                        }
                        // Agent commands
                        CommandAction::ListAgents |
                        CommandAction::AgentTree => {
                            (true, Some(action_name.clone()), None)
                        }
                        // Blocked actions for remote
                        CommandAction::ExecCommand(_) => {
                            (false, None, Some("ExecCommand is blocked for remote execution".to_string()))
                        }
                        // All other actions - attempt but may not have full effect
                        _ => {
                            tracing::info!("Unhandled action for remote: {:?}", action);
                            (true, Some(action_name.clone()), None)
                        }
                    };

                    AgentMessage::SlashCommandResult {
                        command: command.clone(),
                        success,
                        output: None,
                        action_taken,
                        error,
                        blocked: false,
                    }
                }
                Err(e) => {
                    tracing::error!("Slash command failed: {}", e);
                    AgentMessage::SlashCommandResult {
                        command: command.clone(),
                        success: false,
                        output: None,
                        action_taken: None,
                        error: Some(e.to_string()),
                        blocked: false,
                    }
                }
            };

            let _ = update_tx.send(result_msg);
            Ok(false)
        }

        ViewerMessage::SetToolMode { mode } => {
            {
                let mut state = state.write().await;
                state.tool_mode = mode.clone();
                state.status = format!("Tool mode: {}", mode.display_name());
            }

            let ack = AgentMessage::Ack {
                command: "set_tool_mode".to_string(),
            };
            let _ = update_tx.send(ack);

            let state = state.read().await;
            let status_msg = AgentMessage::StatusUpdate {
                status: state.status.clone(),
            };
            let _ = update_tx.send(status_msg);

            Ok(false)
        }

        ViewerMessage::QueueMessage { content } => {
            // Add message to queue
            {
                let mut state = state.write().await;
                match state.message_queue.push_content(content.clone()) {
                    Ok(_) => {
                        let queue_len = state.message_queue.len();
                        tracing::info!("Queued message (queue size: {}): {}",
                            queue_len,
                            &content[..content.len().min(50)]);
                    }
                    Err(e) => {
                        tracing::error!("Failed to queue message: {}", e);
                        let error = AgentMessage::Error {
                            message: format!("Failed to queue message: {}", e),
                            fatal: false,
                        };
                        let _ = update_tx.send(error);
                        return Ok(false);
                    }
                }
            }

            let ack = AgentMessage::Ack {
                command: "queue_message".to_string(),
            };
            let _ = update_tx.send(ack);

            Ok(false)
        }

        ViewerMessage::AcquireLock { resource_type, scope, description } => {
            tracing::info!("Lock request: {:?} scope={} desc={}", resource_type, scope, description);

            // Get lock store and session_id from state
            let (lock_store, session_id) = {
                let state = state.read().await;
                (state.lock_store.clone(), state.session_id.clone())
            };

            // Try to acquire the lock via LockStore
            let lock_type_str = resource_type.as_lock_type_str();
            match lock_store.try_acquire(lock_type_str, &scope, &session_id, None).await {
                Ok(acquired) => {
                    if acquired {
                        tracing::info!("Lock acquired: {:?} scope={}", resource_type, scope);

                        // Broadcast lock change to all viewers
                        let lock_info = LockInfo {
                            agent_id: session_id,
                            resource_type,
                            scope: scope.clone(),
                            description: description.clone(),
                            status: "active".to_string(),
                            held_for_secs: 0,
                        };
                        let change_msg = AgentMessage::LockChanged {
                            change: LockChangeType::Acquired,
                            lock: lock_info,
                        };
                        let _ = update_tx.send(change_msg);

                        let result = AgentMessage::LockResult {
                            success: true,
                            resource_type,
                            scope,
                            error: None,
                            blocking_agent: None,
                        };
                        let _ = update_tx.send(result);
                    } else {
                        // Lock is held by another agent
                        tracing::info!("Lock denied: {:?} scope={} (held by another)", resource_type, scope);

                        // Get info about who holds the lock
                        let blocking_agent = if let Ok(Some(lock_record)) =
                            lock_store.is_locked(lock_type_str, &scope).await
                        {
                            Some(lock_record.agent_id)
                        } else {
                            None
                        };

                        let result = AgentMessage::LockResult {
                            success: false,
                            resource_type,
                            scope,
                            error: Some("Lock held by another agent".to_string()),
                            blocking_agent,
                        };
                        let _ = update_tx.send(result);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to acquire lock: {:?}", e);
                    let result = AgentMessage::LockResult {
                        success: false,
                        resource_type,
                        scope,
                        error: Some(format!("Lock acquisition failed: {}", e)),
                        blocking_agent: None,
                    };
                    let _ = update_tx.send(result);
                }
            }

            Ok(false)
        }

        ViewerMessage::ReleaseLock { resource_type, scope } => {
            tracing::info!("Lock release: {:?} scope={}", resource_type, scope);

            // Get lock store and session_id from state
            let (lock_store, session_id) = {
                let state = state.read().await;
                (state.lock_store.clone(), state.session_id.clone())
            };

            // Release the lock via LockStore
            let lock_type_str = resource_type.as_lock_type_str();
            match lock_store.release(lock_type_str, &scope, &session_id).await {
                Ok(released) => {
                    if released {
                        tracing::info!("Lock released: {:?} scope={}", resource_type, scope);

                        // Broadcast lock change to all viewers
                        let lock_info = LockInfo {
                            agent_id: session_id,
                            resource_type,
                            scope: scope.clone(),
                            description: String::new(),
                            status: "released".to_string(),
                            held_for_secs: 0,
                        };
                        let change_msg = AgentMessage::LockChanged {
                            change: LockChangeType::Released,
                            lock: lock_info,
                        };
                        let _ = update_tx.send(change_msg);
                    } else {
                        tracing::warn!(
                            "Lock release failed (not owned): {:?} scope={}",
                            resource_type, scope
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to release lock: {:?}", e);
                }
            }

            let result = AgentMessage::LockReleased {
                resource_type,
                scope,
            };
            let _ = update_tx.send(result);

            Ok(false)
        }

        ViewerMessage::QueryLocks { scope } => {
            tracing::info!("Lock query: scope={:?}", scope);

            // Get lock store from state
            let lock_store = {
                let state = state.read().await;
                state.lock_store.clone()
            };

            // Query all locks from LockStore
            let locks = match lock_store.list_locks().await {
                Ok(records) => {
                    let now = chrono::Utc::now().timestamp_millis();
                    records
                        .into_iter()
                        .filter(|r| {
                            // Filter by scope if specified
                            scope.as_ref().map_or(true, |s| r.resource_path.starts_with(s))
                        })
                        .filter_map(|r| {
                            // Convert lock record to LockInfo
                            let resource_type = ResourceLockType::from_lock_type_str(&r.lock_type)?;
                            let held_for_secs = ((now - r.acquired_at) / 1000).max(0) as u64;
                            Some(LockInfo {
                                agent_id: r.agent_id,
                                resource_type,
                                scope: r.resource_path,
                                description: String::new(), // LockStore doesn't store description
                                status: "active".to_string(),
                                held_for_secs,
                            })
                        })
                        .collect()
                }
                Err(e) => {
                    tracing::error!("Failed to query locks: {:?}", e);
                    Vec::new()
                }
            };

            let result = AgentMessage::LockStatus { locks };
            let _ = update_tx.send(result);

            Ok(false)
        }

        ViewerMessage::UpdateLockStatus { resource_type, scope, status } => {
            // TODO: Update lock status
            tracing::info!("Lock status update: {:?} scope={} status={}", resource_type, scope, status);

            let ack = AgentMessage::Ack {
                command: "update_lock_status".to_string(),
            };
            let _ = update_tx.send(ack);

            Ok(false)
        }

        // ====================================================================
        // Multi-Agent Messages (Phase 3+ implementation)
        // ====================================================================

        ViewerMessage::ListAgents => {
            tracing::info!("ListAgents request received");

            // Read all agent metadata files
            let agents = match crate::ipc::list_agent_sessions_with_metadata() {
                Ok(agents) => agents,
                Err(e) => {
                    tracing::error!("Failed to list agents: {}", e);
                    Vec::new()
                }
            };

            let result = AgentMessage::AgentList { agents };
            let _ = update_tx.send(result);

            Ok(false)
        }

        ViewerMessage::SpawnAgent { model, reason, working_directory } => {
            tracing::info!(
                "SpawnAgent request: model={:?} reason={:?} wd={:?}",
                model, reason, working_directory
            );

            // Get parent session ID
            let parent_session_id = {
                let state = state.read().await;
                state.session_id.clone()
            };

            // Spawn child agent
            let working_dir = working_directory.map(std::path::PathBuf::from);
            let spawn_reason = reason.unwrap_or_else(|| "Child agent".to_string());

            match crate::agent::spawn::spawn_child_agent(
                &parent_session_id,
                &spawn_reason,
                model.clone(),
                working_dir,
            ).await {
                Ok((child_session_id, _socket_path)) => {
                    tracing::info!("Spawned child agent: {}", child_session_id);

                    // Notify TUI about the new agent
                    let msg = AgentMessage::AgentSpawned {
                        new_session_id: child_session_id,
                        parent_session_id,
                        spawn_reason,
                        model: model.unwrap_or_else(|| "default".to_string()),
                    };
                    let _ = update_tx.send(msg);
                }
                Err(e) => {
                    tracing::error!("Failed to spawn child agent: {}", e);
                    let error_msg = AgentMessage::Error {
                        message: format!("Failed to spawn child agent: {}", e),
                        fatal: false,
                    };
                    let _ = update_tx.send(error_msg);
                }
            }

            Ok(false)
        }

        ViewerMessage::NotifyChildren { action } => {
            // Phase 5: Smart cascade shutdown - notify child agents on exit
            tracing::info!("NotifyChildren request: action={:?}", action);

            // Get our session_id
            let session_id = {
                let state = state.read().await;
                state.session_id.clone()
            };

            // Get all child agents
            let children = match crate::ipc::get_child_agents(&session_id) {
                Ok(children) => children,
                Err(e) => {
                    tracing::error!("Failed to get child agents: {:?}", e);
                    let ack = AgentMessage::Ack {
                        command: "notify_children".to_string(),
                    };
                    let _ = update_tx.send(ack);
                    return Ok(false);
                }
            };

            tracing::info!("Found {} child agent(s) to notify", children.len());

            let mut notified_children = Vec::new();
            use brainwires::agent_network::ipc::{ChildNotifyAction, ParentSignalType};

            for child in &children {
                // Check if child is alive
                if !crate::ipc::is_agent_alive(&child.session_id).await {
                    tracing::info!("Child {} is not alive, skipping", child.session_id);
                    continue;
                }

                // Determine signal based on action and child's busy state
                let signal = match &action {
                    ChildNotifyAction::ShutdownIfIdle => {
                        // Smart cascade: if idle, shutdown; if busy, let them finish
                        ParentSignalType::ParentExiting
                    }
                    ChildNotifyAction::ForceShutdown => {
                        ParentSignalType::Shutdown
                    }
                    ChildNotifyAction::Detach => {
                        ParentSignalType::Detached
                    }
                };

                // Read child's session token for authenticated connection
                let child_token = match crate::ipc::read_session_token(&child.session_id) {
                    Ok(Some(token)) => token,
                    _ => {
                        tracing::warn!("No session token for child {}", child.session_id);
                        continue;
                    }
                };

                // Send signal to child via authenticated IPC
                if let Ok(mut conn) = crate::ipc::connect_to_agent(&child.session_id).await {
                    use brainwires::agent_network::ipc::{Handshake, HandshakeResponse};

                    // Perform authenticated handshake
                    let handshake = Handshake::reattach(child.session_id.clone(), child_token);
                    if conn.writer.write(&handshake).await.is_err() {
                        tracing::warn!("Failed to send handshake to child {}", child.session_id);
                        continue;
                    }

                    // Wait for response
                    match conn.reader.read::<HandshakeResponse>().await {
                        Ok(Some(response)) if response.accepted => {
                            let signal_msg = ViewerMessage::ParentSignal {
                                signal: signal.clone(),
                                parent_session_id: session_id.clone(),
                            };
                            if let Err(e) = conn.writer.write(&signal_msg).await {
                                tracing::error!(
                                    "Failed to send signal to child {}: {:?}",
                                    child.session_id, e
                                );
                            } else {
                                tracing::info!(
                                    "Sent {:?} signal to child {}",
                                    signal, child.session_id
                                );
                                notified_children.push(child.session_id.clone());
                            }
                        }
                        _ => {
                            tracing::warn!(
                                "Child {} rejected connection during notify",
                                child.session_id
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "Could not connect to child {} to send signal",
                        child.session_id
                    );
                }
            }

            // Send AgentExiting message to TUI with list of notified children
            let exit_msg = AgentMessage::AgentExiting {
                session_id: session_id.clone(),
                reason: format!("NotifyChildren action: {:?}", action),
                children_notified: notified_children,
            };
            let _ = update_tx.send(exit_msg);

            let ack = AgentMessage::Ack {
                command: "notify_children".to_string(),
            };
            let _ = update_tx.send(ack);

            Ok(false)
        }

        ViewerMessage::ParentSignal { signal, parent_session_id } => {
            // Handle signal from parent agent
            tracing::info!(
                "ParentSignal received: signal={:?} from parent={}",
                signal, parent_session_id
            );

            use brainwires::agent_network::ipc::ParentSignalType;
            match signal {
                ParentSignalType::ParentExiting => {
                    // Check if we're busy - if idle, set exit_when_done
                    let is_busy = {
                        let state = state.read().await;
                        state.is_busy
                    };

                    if !is_busy {
                        // Idle - exit gracefully
                        let mut state = state.write().await;
                        state.exit_when_done = true;
                        tracing::info!("Parent exiting, we're idle - will exit when done");
                    } else {
                        // Busy - finish current work first
                        let mut state = state.write().await;
                        state.exit_when_done = true;
                        tracing::info!("Parent exiting, we're busy - will exit after current work");
                    }
                }
                ParentSignalType::Shutdown => {
                    // Immediate shutdown requested
                    tracing::info!("Shutdown signal from parent - exiting immediately");
                    return Ok(true);
                }
                ParentSignalType::Detached => {
                    // Parent detached - we're now an orphan, just keep running
                    tracing::info!("Parent detached - continuing as orphan");
                }
            }

            // Notify TUI of the signal
            let _ = update_tx.send(AgentMessage::ParentSignalReceived {
                signal,
                parent_session_id,
            });

            Ok(false)
        }

        ViewerMessage::Disconnect => {
            // Viewer is gracefully disconnecting (different from Detach)
            // This is a clean close, not a background operation
            tracing::info!("Viewer disconnecting gracefully");

            // Just acknowledge - don't set exit_when_done or anything special
            let ack = AgentMessage::Ack {
                command: "disconnect".to_string(),
            };
            let _ = update_tx.send(ack);

            // Return false to indicate this isn't an exit signal
            // The viewer will close the connection after this
            Ok(false)
        }

        // ====================================================================
        // Plan Mode Messages
        // ====================================================================

        ViewerMessage::EnterPlanMode { focus } => {
            tracing::info!("EnterPlanMode request: focus={:?}", focus);

            let result = {
                let mut state = state.write().await;
                state.enter_plan_mode(focus).await
            };

            match result {
                Ok(msg) => {
                    let _ = update_tx.send(msg);
                }
                Err(e) => {
                    tracing::error!("Failed to enter plan mode: {}", e);
                    let _ = update_tx.send(AgentMessage::Error {
                        message: format!("Failed to enter plan mode: {}", e),
                        fatal: false,
                    });
                }
            }

            Ok(false)
        }

        ViewerMessage::ExitPlanMode => {
            tracing::info!("ExitPlanMode request");

            let result = {
                let mut state = state.write().await;
                state.exit_plan_mode().await
            };

            match result {
                Ok(msg) => {
                    let _ = update_tx.send(msg);
                }
                Err(e) => {
                    tracing::error!("Failed to exit plan mode: {}", e);
                    let _ = update_tx.send(AgentMessage::Error {
                        message: format!("Failed to exit plan mode: {}", e),
                        fatal: false,
                    });
                }
            }

            Ok(false)
        }

        ViewerMessage::PlanModeUserInput { content, context_files } => {
            tracing::info!("PlanModeUserInput: {} chars", content.len());

            // Process context files if provided
            let mut full_content = content;
            if !context_files.is_empty() {
                let mut context_content = String::new();
                for file_path in &context_files {
                    match std::fs::read_to_string(file_path) {
                        Ok(file_content) => {
                            context_content.push_str(&format!(
                                "\n--- Context File: {} ---\n{}\n--- End of {} ---\n",
                                file_path,
                                file_content,
                                file_path
                            ));
                            tracing::info!("Loaded context file for plan mode: {}", file_path);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read context file {}: {}", file_path, e);
                        }
                    }
                }
                if !context_content.is_empty() {
                    full_content = format!("{}\n\n# Referenced Files\n{}", full_content, context_content);
                }
            }

            // Process plan mode input in background
            let state_clone = Arc::clone(&state);
            let update_tx_clone = update_tx.clone();

            tokio::spawn(async move {
                let result = {
                    let mut state = state_clone.write().await;
                    state.process_plan_mode_input(full_content, &update_tx_clone).await
                };

                if let Err(e) = result {
                    tracing::error!("Failed to process plan mode input: {}", e);
                    let _ = update_tx_clone.send(AgentMessage::Error {
                        message: format!("Plan mode error: {}", e),
                        fatal: false,
                    });
                }
            });

            Ok(false)
        }

        ViewerMessage::PlanModeSyncRequest => {
            let state = state.read().await;
            if state.is_plan_mode {
                let sync_msg = state.create_plan_mode_sync_message();
                let _ = update_tx.send(sync_msg);
            } else {
                // Not in plan mode, send regular sync
                let sync_msg = state.create_sync_message();
                let _ = update_tx.send(sync_msg);
            }

            Ok(false)
        }
    }
}

/// Process a pending user request that was loaded from storage
///
/// This is called when the Agent starts and finds that the last message
/// was from the user (meaning AI never got to respond before backgrounding).
async fn process_pending_request(
    state: Arc<RwLock<AgentState>>,
    update_tx: broadcast::Sender<AgentMessage>,
) -> Result<()> {
    // Get the necessary data from state
    let (provider, conversation_history, tools, model, tool_executor, working_directory, session_id) = {
        let mut state = state.write().await;
        state.has_pending_request = false; // Clear the flag
        state.is_busy = true;
        state.status = "Processing pending request...".to_string();

        // Combine core tools with MCP tools
        let mut all_tools = state.tools.clone();
        all_tools.extend(state.mcp_tools.clone());

        (
            state.provider.clone(),
            state.conversation_history.clone(),
            all_tools,
            state.model.clone(),
            state.tool_executor.clone(),
            state.working_directory.clone(),
            state.session_id.clone(),
        )
    };

    // Send status update
    let _ = update_tx.send(AgentMessage::StatusUpdate {
        status: "Processing pending request...".to_string(),
    });

    // Stream the AI response with tool execution support
    let full_response = stream_with_tool_execution(
        provider,
        conversation_history,
        tools,
        model.clone(),
        tool_executor,
        working_directory,
        update_tx.clone(),
        state.clone(),
    ).await?;

    // Finalize: add assistant message and update state
    {
        let mut state = state.write().await;
        state.add_assistant_message(full_response.clone());
        state.is_busy = false;
        state.status = format!("Ready - Model: {}", model);

        // Save response to storage
        let assistant_msg = crate::storage::MessageMetadata {
            message_id: uuid::Uuid::new_v4().to_string(),
            conversation_id: session_id,
            role: "assistant".to_string(),
            content: full_response,
            token_count: None,
            model_id: Some(model.clone()),
            images: None,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        };
        if let Err(e) = state.message_store.add(assistant_msg).await {
            tracing::warn!("Failed to save assistant message: {}", e);
        }

        // Send message added notification
        if let Some(last_msg) = state.messages.last() {
            let _ = update_tx.send(AgentMessage::MessageAdded {
                message: last_msg.clone(),
            });
        }

        let _ = update_tx.send(AgentMessage::StatusUpdate {
            status: state.status.clone(),
        });
    }

    Ok(())
}

/// Spawn an agent process in the background
pub async fn spawn_agent(
    session_id: Option<String>,
    model: Option<String>,
    mdap_config: Option<MdapConfig>,
) -> Result<String> {
    let agent = AgentProcess::new(session_id, model, mdap_config).await?;
    let session_id = agent.session_id().await;
    let socket_path = agent.socket_path().clone();

    // Spawn the agent in a background task
    tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("Agent error: {}", e);
        }
    });

    // Wait for socket to be ready
    for _ in 0..50 {
        if socket_path.exists() {
            return Ok(session_id);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    bail!("Agent failed to start (socket not created)");
}

/// Process queued messages sequentially
///
/// This drains the message queue and processes each message as a user input,
/// allowing for injecting automated messages into the conversation flow.
async fn process_queued_messages(
    state: Arc<RwLock<AgentState>>,
    update_tx: broadcast::Sender<AgentMessage>,
) {
    loop {
        // Get next message from queue
        let queued_msg = {
            let mut state = state.write().await;
            state.message_queue.pop()
        };

        let msg = match queued_msg {
            Some(m) => m,
            None => break, // Queue is empty
        };

        tracing::info!(
            "Processing queued message {}: {}",
            msg.id,
            &msg.content[..msg.content.len().min(50)]
        );

        // Get required state for processing
        let (_resolved_content, provider, conversation_history, tools, model, tool_executor, working_directory) = {
            let mut state = state.write().await;
            let resolved_content = state.seal_preprocess(&msg.content);
            state.add_user_message(resolved_content.clone());
            state.is_busy = true;
            state.status = "Processing queued message...".to_string();

            let mut all_tools = state.tools.clone();
            all_tools.extend(state.mcp_tools.clone());

            (
                resolved_content,
                state.provider.clone(),
                state.conversation_history.clone(),
                all_tools,
                state.model.clone(),
                state.tool_executor.clone(),
                state.working_directory.clone(),
            )
        };

        // Notify viewers
        let _ = update_tx.send(AgentMessage::StatusUpdate {
            status: "Processing queued message...".to_string(),
        });

        // Process this message
        let result = stream_with_tool_execution(
            provider,
            conversation_history,
            tools,
            model.clone(),
            tool_executor,
            working_directory,
            update_tx.clone(),
            state.clone(),
        ).await;

        match result {
            Ok(full_response) => {
                let mut state = state.write().await;
                state.add_assistant_message(full_response);
                state.is_busy = false;
                state.status = format!("Ready - Model: {}", model);

                if let Some(last_msg) = state.messages.last() {
                    let _ = update_tx.send(AgentMessage::MessageAdded {
                        message: last_msg.clone(),
                    });
                }

                let _ = update_tx.send(AgentMessage::StatusUpdate {
                    status: state.status.clone(),
                });
            }
            Err(e) => {
                tracing::error!("Error processing queued message {}: {}", msg.id, e);

                // Requeue for retry if retries remaining
                let mut state = state.write().await;
                if let Err(requeue_err) = state.message_queue.requeue(msg.clone()) {
                    tracing::error!("Failed to requeue message: {}", requeue_err);
                }

                state.is_busy = false;
                state.status = format!("Error processing queued message: {}", e);

                let _ = update_tx.send(AgentMessage::Error {
                    message: e.to_string(),
                    fatal: false,
                });
                let _ = update_tx.send(AgentMessage::StatusUpdate {
                    status: state.status.clone(),
                });

                // Don't process more messages after an error
                break;
            }
        }
    }

    tracing::info!("Finished processing queued messages");
}

/// Stream AI response with tool execution support
///
/// This function implements the agentic loop:
/// 1. Stream AI response
/// 2. If tool call received, execute tool locally
/// 3. Send continuation request with tool result
/// 4. Repeat until AI finishes (no more tool calls)
async fn stream_with_tool_execution(
    provider: Arc<dyn crate::providers::Provider>,
    conversation_history: Vec<crate::types::message::Message>,
    tools: Vec<crate::types::tool::Tool>,
    model: String,
    tool_executor: Arc<crate::tools::ToolExecutor>,
    working_directory: String,
    update_tx: broadcast::Sender<AgentMessage>,
    state: Arc<RwLock<AgentState>>,
) -> Result<String> {
    use crate::types::message::StreamChunk;
    use crate::types::provider::ChatOptions;
    use futures::StreamExt;

    // Debug: Log tools being sent
    tracing::info!("🔧 Agent streaming with {} tools:", tools.len());
    for tool in &tools {
        tracing::info!("  - {}: {}", tool.name, tool.description.chars().take(50).collect::<String>());
    }
    tracing::info!("🔧 Working directory: {}", working_directory);

    // Extract system prompt from conversation history (first message if it's a System message)
    let system_prompt = conversation_history.iter()
        .find(|m| m.role == crate::types::message::Role::System)
        .and_then(|m| m.text().map(|s| s.to_string()));

    if let Some(ref prompt) = system_prompt {
        tracing::info!("🔧 System prompt: {} chars", prompt.len());
    } else {
        tracing::warn!("🔧 No system prompt found in conversation history!");
    }

    // Build ChatOptions with system prompt
    let mut options = ChatOptions::default();
    options.system = system_prompt;

    let mut stream = provider.stream_chat(&conversation_history, Some(&tools), &options);
    let mut full_response = String::new();

    let mut chunk_count = 0;
    while let Some(chunk) = stream.next().await {
        chunk_count += 1;
        match chunk {
            Ok(StreamChunk::Text(text)) => {
                full_response.push_str(&text);
                let _ = update_tx.send(AgentMessage::StreamChunk { text });
            }
            Ok(StreamChunk::ToolCall {
                call_id,
                response_id: _,
                chat_id,
                tool_name,
                server,
                parameters
            }) => {
                tracing::info!("🔧 TOOL CALL RECEIVED: {} from server: {}", tool_name, server);

                // Only execute cli-local tools
                if server != "cli-local" {
                    tracing::warn!("Ignoring tool from non-local server: {}", server);
                    continue;
                }

                // Notify TUI about tool call start
                let _ = update_tx.send(AgentMessage::ToolCallStart {
                    id: call_id.clone(),
                    name: tool_name.clone(),
                    server: Some(server.clone()),
                    input: parameters.clone(),
                });
                let _ = update_tx.send(AgentMessage::StatusUpdate {
                    status: format!("⚙️ Executing: {}...", tool_name),
                });

                tracing::info!("🔧 Executing tool: {} (call_id: {})", tool_name, call_id);

                // Execute the tool
                let tool_use = ToolUse {
                    id: call_id.clone(),
                    name: tool_name.clone(),
                    input: parameters.clone(),
                };

                let tool_context = ToolContext {
                    working_directory: working_directory.clone(),
                    // Use full_access for agent mode - agents need unrestricted file access
                    capabilities: serde_json::to_value(&brainwires::permissions::AgentCapabilities::full_access()).ok(),
                    ..Default::default()
                };

                let tool_result = match tool_executor.execute(&tool_use, &tool_context).await {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("Tool execution failed: {}", e);
                        // Record failed tool outcome for implicit learning
                        {
                            let mut state_guard = state.write().await;
                            state_guard.record_tool_outcome(
                                &tool_name,
                                &parameters.to_string(),
                                false,
                                Some(&format!("{}", e)),
                                0,
                            );
                        }
                        let _ = update_tx.send(AgentMessage::ToolResult {
                            id: call_id.clone(),
                            name: tool_name.clone(),
                            output: None,
                            error: Some(format!("Error: {}", e)),
                        });
                        // Continue streaming - tool error is not fatal
                        continue;
                    }
                };

                // Limit tool output to prevent context overflow
                const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;
                let truncated_output = if tool_result.content.len() > MAX_TOOL_OUTPUT_CHARS {
                    let truncated = &tool_result.content[..MAX_TOOL_OUTPUT_CHARS];
                    format!(
                        "{}\n\n[Output truncated: {} of {} characters]",
                        truncated,
                        MAX_TOOL_OUTPUT_CHARS,
                        tool_result.content.len()
                    )
                } else {
                    tool_result.content.clone()
                };

                // Notify TUI about tool completion
                let _ = update_tx.send(AgentMessage::ToolResult {
                    id: call_id.clone(),
                    name: tool_name.clone(),
                    output: if tool_result.is_error {
                        None
                    } else {
                        Some(if truncated_output.len() > 200 {
                            format!("{}...", &truncated_output[..200])
                        } else {
                            truncated_output.clone()
                        })
                    },
                    error: if tool_result.is_error {
                        Some(truncated_output.clone())
                    } else {
                        None
                    },
                });

                if tool_result.is_error {
                    tracing::warn!("Tool {} returned error: {}", tool_name, truncated_output);
                } else {
                    tracing::info!("Tool {} completed successfully", tool_name);
                }

                // Record tool outcome for implicit learning
                {
                    let mut state_guard = state.write().await;
                    state_guard.record_tool_outcome(
                        &tool_name,
                        &parameters.to_string(),
                        !tool_result.is_error,
                        if tool_result.is_error { Some(&truncated_output) } else { None },
                        0, // TODO: Track actual execution time
                    );
                }

                // Send streaming continuation request with tool result
                let _ = update_tx.send(AgentMessage::StatusUpdate {
                    status: "Processing tool result...".to_string(),
                });

                // Stream the continuation response
                let continuation_result = stream_continuation_with_tool_result(
                    &conversation_history,
                    &tools,
                    &model,
                    chat_id.clone(),
                    &call_id,
                    &tool_name,
                    &parameters,
                    &truncated_output,
                    &tool_executor,
                    &working_directory,
                    &update_tx,
                    &state,
                ).await;

                match continuation_result {
                    Ok(continuation_text) => {
                        tracing::info!("🔧 Continuation returned {} chars of text", continuation_text.len());
                        if continuation_text.len() > 100 {
                            tracing::info!("🔧 Continuation preview: {}...", &continuation_text[..100]);
                        } else {
                            tracing::info!("🔧 Continuation text: {}", continuation_text);
                        }
                        full_response.push_str(&continuation_text);
                    }
                    Err(e) => {
                        tracing::error!("Continuation request failed: {}", e);
                        let _ = update_tx.send(AgentMessage::Error {
                            message: format!("Continuation failed: {}", e),
                            fatal: false,
                        });
                    }
                }

                // Tool execution and continuation complete, stop reading original stream
                break;
            }
            Ok(StreamChunk::Done) => {
                tracing::info!("🔧 Stream completed after {} chunks, response length: {} chars", chunk_count, full_response.len());
                let _ = update_tx.send(AgentMessage::StreamEnd {
                    finish_reason: Some("stop".to_string()),
                });
                break;
            }
            Ok(other) => {
                tracing::debug!("🔧 Received other chunk type: {:?}", other);
                continue;
            }
            Err(e) => {
                tracing::error!("🔧 Stream error: {}", e);
                let _ = update_tx.send(AgentMessage::Error {
                    message: e.to_string(),
                    fatal: false,
                });
                return Err(e);
            }
        }
    }

    tracing::info!("🔧 Stream finished, total response: {} chars", full_response.len());

    // Send stream end if we haven't already (tool execution path)
    let _ = update_tx.send(AgentMessage::StreamEnd {
        finish_reason: Some("stop".to_string()),
    });

    Ok(full_response)
}

/// Stream continuation response with tool result - with real-time streaming to TUI
///
/// This function:
/// 1. Sends a continuation request to the backend with the tool result
/// 2. Streams the response text in real-time to the TUI
/// 3. Handles chained tool calls recursively
async fn stream_continuation_with_tool_result(
    conversation_history: &[crate::types::message::Message],
    tools: &[crate::types::tool::Tool],
    model: &str,
    chat_id: Option<String>,
    call_id: &str,
    tool_name: &str,
    tool_parameters: &serde_json::Value,
    tool_output: &str,
    tool_executor: &Arc<crate::tools::ToolExecutor>,
    working_directory: &str,
    update_tx: &broadcast::Sender<AgentMessage>,
    state: &Arc<RwLock<AgentState>>,
) -> Result<String> {
    use crate::types::message::Role;

    // Get session for backend URL
    let session = SessionManager::load()?
        .context("No active session found")?;

    // Get API key from secure storage (keyring or fallback)
    let api_key = SessionManager::get_api_key()?
        .context("No API key found. Please re-authenticate with: brainwires auth")?;

    let http_client = reqwest::Client::new();
    let url = format!("{}/api/chat/stream", session.backend);

    // Build conversation history using shared helper that properly serializes
    // tool calls and tool results (not just text content)
    let mut conv_history = crate::types::message::serialize_messages_to_stateless_history(conversation_history);

    // Add the function_call (AI's request to call the tool)
    conv_history.push(json!({
        "role": "function_call",
        "call_id": call_id,
        "name": tool_name,
        "arguments": tool_parameters.to_string()
    }));

    // Add the tool result
    conv_history.push(json!({
        "role": "tool",
        "call_id": call_id,
        "name": tool_name,
        "content": tool_output
    }));

    // Convert tools to MCP format
    let mcp_tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "server": "cli-local",
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect();

    // Extract system prompt from conversation history
    let system_prompt = conversation_history.iter()
        .find(|m| m.role == Role::System)
        .and_then(|m| m.text().map(|s| s.to_string()));

    // Build request
    let mut request_body = json!({
        "chatId": chat_id,
        "content": "",
        "model": model,
        "timezone": "UTC",
        "conversationHistory": conv_history
    });

    // Add system prompt if present
    if let Some(ref prompt) = system_prompt {
        request_body["systemPrompt"] = json!(prompt);
    }

    if !mcp_tools.is_empty() {
        request_body["selectedMCPTools"] = json!(mcp_tools);
    }

    tracing::info!("🔧 Continuation request: {} tools, {} history msgs, system_prompt: {}",
        mcp_tools.len(),
        conv_history.len(),
        system_prompt.as_ref().map(|s| s.len()).unwrap_or(0)
    );

    // Send request
    let response = http_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key.as_str()))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send continuation request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Continuation request failed ({}): {}",
            status,
            error_text
        ));
    }

    // Stream the SSE response with real-time output
    let mut full_text = String::new();
    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = chunk_result.context("Failed to read stream chunk")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE events (delimited by \n\n)
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            // Parse SSE event block
            let mut event_type = None;
            let mut event_data = None;

            for line in event_block.lines() {
                if let Some(evt) = line.strip_prefix("event: ") {
                    event_type = Some(evt.to_string());
                } else if let Some(data) = line.strip_prefix("data: ") {
                    event_data = Some(data.to_string());
                }
            }

            if let (Some(evt_type), Some(data)) = (event_type, event_data) {
                tracing::debug!("🔧 Continuation SSE event: type={}, data_len={}", evt_type, data.len());
                match evt_type.as_str() {
                    "delta" => {
                        if let Ok(delta_data) = serde_json::from_str::<serde_json::Value>(&data) {
                            if let Some(text) = delta_data.get("delta").and_then(|t| t.as_str()) {
                                // Stream each chunk in real-time!
                                full_text.push_str(text);
                                let _ = update_tx.send(AgentMessage::StreamChunk {
                                    text: text.to_string()
                                });
                            }
                        }
                    }
                    "toolCall" => {
                        tracing::info!("🔧 Continuation received chained toolCall event: {}", data);
                        // Handle chained tool calls
                        if let Ok(tool_data) = serde_json::from_str::<serde_json::Value>(&data) {
                            let next_call_id = tool_data.get("callId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let next_tool_name = tool_data.get("toolName")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let next_server = tool_data.get("server")
                                .and_then(|v| v.as_str())
                                .unwrap_or("cli-local")
                                .to_string();
                            let next_parameters = tool_data.get("parameters")
                                .cloned()
                                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                            let next_chat_id = tool_data.get("chatId")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Only execute cli-local tools
                            if next_server != "cli-local" {
                                tracing::warn!("Ignoring chained tool from non-local server: {}", next_server);
                                continue;
                            }

                            tracing::info!("Chained tool call: {} (call_id: {})", next_tool_name, next_call_id);

                            // Notify TUI about tool call start
                            let _ = update_tx.send(AgentMessage::ToolCallStart {
                                id: next_call_id.clone(),
                                name: next_tool_name.clone(),
                                server: Some(next_server.clone()),
                                input: next_parameters.clone(),
                            });
                            let _ = update_tx.send(AgentMessage::StatusUpdate {
                                status: format!("Executing chained tool: {}...", next_tool_name),
                            });

                            // Execute the chained tool
                            let tool_use = ToolUse {
                                id: next_call_id.clone(),
                                name: next_tool_name.clone(),
                                input: next_parameters.clone(),
                            };

                            let tool_context = ToolContext {
                                working_directory: working_directory.to_string(),
                                capabilities: serde_json::to_value(&brainwires::permissions::AgentCapabilities::full_access()).ok(),
                                ..Default::default()
                            };

                            let chained_result = match tool_executor.execute(&tool_use, &tool_context).await {
                                Ok(result) => result,
                                Err(e) => {
                                    tracing::error!("Chained tool execution failed: {}", e);
                                    // Record failed tool outcome for implicit learning
                                    {
                                        let mut state_guard = state.write().await;
                                        state_guard.record_tool_outcome(
                                            &next_tool_name,
                                            &next_parameters.to_string(),
                                            false,
                                            Some(&format!("{}", e)),
                                            0,
                                        );
                                    }
                                    let _ = update_tx.send(AgentMessage::ToolResult {
                                        id: next_call_id.clone(),
                                        name: next_tool_name.clone(),
                                        output: None,
                                        error: Some(format!("Error: {}", e)),
                                    });
                                    continue;
                                }
                            };

                            // Truncate output
                            const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;
                            let chained_output = if chained_result.content.len() > MAX_TOOL_OUTPUT_CHARS {
                                format!(
                                    "{}\n\n[Output truncated: {} of {} characters]",
                                    &chained_result.content[..MAX_TOOL_OUTPUT_CHARS],
                                    MAX_TOOL_OUTPUT_CHARS,
                                    chained_result.content.len()
                                )
                            } else {
                                chained_result.content.clone()
                            };

                            // Notify TUI
                            let _ = update_tx.send(AgentMessage::ToolResult {
                                id: next_call_id.clone(),
                                name: next_tool_name.clone(),
                                output: if chained_result.is_error { None } else { Some(chained_output.clone()) },
                                error: if chained_result.is_error { Some(chained_output.clone()) } else { None },
                            });

                            // Record tool outcome for implicit learning
                            {
                                let mut state_guard = state.write().await;
                                state_guard.record_tool_outcome(
                                    &next_tool_name,
                                    &next_parameters.to_string(),
                                    !chained_result.is_error,
                                    if chained_result.is_error { Some(&chained_output) } else { None },
                                    0,
                                );
                            }

                            // Recursively continue with chained tool result
                            // Build updated conversation history including ALL prior tool interactions
                            let mut updated_history = conversation_history.to_vec();
                            // Add the assistant text so far (before the tool call)
                            if !full_text.is_empty() {
                                updated_history.push(crate::types::message::Message::assistant(&full_text));
                            }
                            // Add the previous tool call (the one that triggered this continuation) as a ToolUse block
                            updated_history.push(crate::types::message::Message {
                                role: crate::types::message::Role::Assistant,
                                content: crate::types::message::MessageContent::Blocks(vec![
                                    crate::types::message::ContentBlock::ToolUse {
                                        id: call_id.to_string(),
                                        name: tool_name.to_string(),
                                        input: tool_parameters.clone(),
                                    },
                                ]),
                                name: None,
                                metadata: None,
                            });
                            // Add the previous tool result
                            updated_history.push(crate::types::message::Message::tool_result(
                                call_id,
                                tool_output,
                            ));

                            let chained_text = Box::pin(stream_continuation_with_tool_result(
                                &updated_history,
                                tools,
                                model,
                                next_chat_id.or(chat_id.clone()),
                                &next_call_id,
                                &next_tool_name,
                                &next_parameters,
                                &chained_output,
                                tool_executor,
                                working_directory,
                                update_tx,
                                state,
                            )).await?;

                            full_text.push_str(&chained_text);
                            return Ok(full_text);
                        }
                    }
                    "complete" => {
                        tracing::info!("🔧 Continuation stream completed with {} chars of text (no more tool calls)", full_text.len());
                        return Ok(full_text);
                    }
                    "error" => {
                        let error_msg = if let Ok(error_data) =
                            serde_json::from_str::<serde_json::Value>(&data)
                        {
                            error_data
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown error")
                                .to_string()
                        } else {
                            "Unknown error".to_string()
                        };
                        return Err(anyhow::anyhow!("Continuation stream error: {}", error_msg));
                    }
                    _ => {
                        // Ignore other event types
                    }
                }
            }
        }
    }

    Ok(full_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
    // They need to be careful about socket cleanup
}
