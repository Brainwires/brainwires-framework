//! Terminal User Interface Module
//!
//! This module provides a full-screen TUI for interactive chat sessions
//! using the ratatui framework.

mod app;
mod console;
mod events;
mod exec_overlay;
mod help_content;
pub(crate) mod hotkey_content;
pub mod question_parser;
mod ui;

pub use app::{App, AppMode, LogLevel};
pub use console::ConsoleBuffer;
pub use events::{Event, EventHandler};
pub use exec_overlay::execute_command_overlay;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use crate::mdap::MdapConfig;

/// Initialize the TUI terminal
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste // Enable bracketed paste to detect paste vs typing
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    println!(); // Ensure prompt starts on a new line
    Ok(())
}

/// Emergency restore of terminal without needing Terminal object
/// Used when we need to restore before Terminal is fully set up
fn emergency_restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        crossterm::cursor::Show
    );
    println!(); // Ensure prompt starts on a new line
}

/// Result of running the TUI app loop
enum TuiLoopResult {
    /// Normal exit
    Exit,
}

/// Run the TUI application
///
/// The TUI always operates in IPC mode, connecting to a Session (Agent) process.
/// The Session handles all AI/tools/MCP, while the TUI is a pure viewer.
pub async fn run_tui(
    session_id: Option<String>,
    model: Option<String>,
    mdap_config: Option<MdapConfig>,
    pty_session: bool,
) -> Result<()> {
    // Ignore signals that could kill us during attach/reattach
    // IMPORTANT: We ignore SIGINT so Ctrl+C is captured as a keyboard event by crossterm
    // rather than being handled by the default signal handler. This ensures consistent
    // quit behavior whether starting fresh or reattaching.
    #[cfg(unix)]
    unsafe {
        // SIGINT - We handle Ctrl+C as a keyboard event, not a signal
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        // SIGHUP - sent when controlling terminal closes
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        // SIGPIPE - sent when writing to a broken pipe/socket
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
        // SIGTTOU/SIGTTIN - sent when background process tries to access terminal
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
    }

    // Set up panic hook to write to file for debugging
    std::panic::set_hook(Box::new(|panic_info| {
        let _ = std::fs::write("/tmp/brainwires_panic.log", format!("{:?}", panic_info));
        emergency_restore_terminal();
    }));

    // Disable tracing for TUI mode
    crate::utils::logger::init_with_output(false);

    // Generate or use provided session ID
    let session_id = session_id
        .unwrap_or_else(|| format!("session-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")));

    // Always spawn or connect to a Session (Agent) process
    // The Session is the "brain" that handles AI/tools/MCP
    use crate::agent::spawn::spawn_agent_process;
    use brainwires::agent_network::ipc::{AgentMessage, Handshake};

    eprintln!("Connecting to session: {}", session_id);

    // Spawn agent if not already running
    let socket_path =
        spawn_agent_process(&session_id, model.as_deref(), mdap_config.as_ref()).await?;
    eprintln!("Session ready at: {}", socket_path.display());

    // Connect to the Session via IPC
    let mut conn = crate::ipc::connect_to_agent(&session_id)
        .await
        .context("Failed to connect to session")?;

    // Send handshake (new session - agent will return the token)
    let handshake = Handshake::new_session();
    conn.writer.write(&handshake).await?;

    // Wait for handshake response
    use brainwires::agent_network::ipc::HandshakeResponse;
    let response: HandshakeResponse = conn
        .reader
        .read()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session closed during handshake"))?;

    if !response.accepted {
        anyhow::bail!(
            "Session rejected connection: {}",
            response.error.unwrap_or_default()
        );
    }

    // The session token is returned in the response and also saved to disk
    // by the agent - the TUI will read it from disk when reattaching
    if response.session_token.is_some() {
        tracing::debug!("Received session token for secure reattachment");
    }

    // Wait for ConversationSync to get initial state
    let initial_sync: AgentMessage = conn
        .reader
        .read()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session closed before sending state"))?;

    // Create TUI App in viewer mode (no AI provider)
    let mut app = App::new_viewer(session_id.clone(), model).await?;
    app.is_ipc_mode = true;
    app.is_pty_session = pty_session;

    // Refresh models cache in background on startup
    // This ensures the autocomplete always has fresh models from the backend
    tokio::spawn(async {
        use crate::config::ModelRegistry;
        if let Err(e) = ModelRegistry::fetch_models().await {
            tracing::warn!("Failed to refresh models cache on startup: {}", e);
        } else {
            tracing::debug!("Models cache refreshed on startup");
        }
    });

    // Populate state from Session's initial sync
    if let AgentMessage::ConversationSync {
        messages,
        status,
        model: session_model,
        tool_mode,
        mcp_servers,
        ..
    } = initial_sync
    {
        app.messages = messages
            .iter()
            .map(|m| crate::tui::app::TuiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                created_at: m.created_at,
            })
            .collect();
        app.set_status(LogLevel::Info, status);
        app.model = session_model;
        app.tool_mode = tool_mode;
        app.mcp_connected_servers = mcp_servers;
        eprintln!("Loaded {} messages from session", app.messages.len());
    }

    // Store the IPC connection writer for sending messages to Session
    // The reader is handled by the EventHandler's IPC reader task
    app.ipc_writer = Some(conn.writer);

    // Store flag for IPC mode
    app.is_ipc_mode = true;

    // Store reader for the event handler to use (consumed on first loop iteration)
    let mut ipc_reader = Some(conn.reader);

    if pty_session {
        // Set scroll_to_bottom_on_resize for PTY mode
        // because the first render happens before the client sends window size
        app.scroll_to_bottom_on_resize = true;
    }

    // Main TUI loop - handles backgrounding and reattachment
    #[allow(clippy::never_loop)]
    loop {
        // Initialize terminal and enter alternate screen
        let mut terminal = init_terminal()?;

        // Set up debug handler to capture eprintln-like output
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        crate::utils::debug::set_debug_handler(move |msg| {
            let _ = tx.send(msg);
        });

        // Create event handler
        let mut events = EventHandler::new(250);

        // Start IPC reader task if we have a connection (event-driven, no polling)
        // Note: ipc_reader is consumed here; subsequent loops won't have it
        // This is fine because we only need to start it once
        if let Some(reader) = ipc_reader.take() {
            events.start_ipc_reader(reader);
        }

        // Main event loop
        let result = run_app(&mut terminal, &mut app, events, rx).await;

        // Clear debug handler
        crate::utils::debug::clear_debug_handler();

        match result {
            Ok(TuiLoopResult::Exit) => {
                // Normal exit - send Exit to Session to shut it down
                if let Some(mut writer) = app.ipc_writer.take() {
                    use brainwires::agent_network::ipc::ViewerMessage;
                    let _ = writer.write(&ViewerMessage::Exit).await;
                }
                // Restore terminal and return
                restore_terminal(terminal)?;
                return Ok(());
            }
            Err(e) => {
                // Error exit - still try to shut down Session
                if let Some(mut writer) = app.ipc_writer.take() {
                    use brainwires::agent_network::ipc::ViewerMessage;
                    let _ = writer.write(&ViewerMessage::Exit).await;
                }
                // Restore terminal and return error
                let _ = restore_terminal(terminal);
                return Err(e);
            }
        }
    }
}

/// Main application event loop
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mut events: EventHandler,
    debug_rx: std::sync::mpsc::Receiver<String>,
) -> Result<TuiLoopResult> {
    use crate::tui::app::session_management::SessionManagement;
    use tokio::time::{Duration, timeout};

    // Track mouse capture state to detect changes
    let mut mouse_capture_enabled = true;

    loop {
        // Check for debug messages
        while let Ok(msg) = debug_rx.try_recv() {
            app.add_console_message(msg);
        }

        // Check for pending background action
        if app.pending_background {
            app.pending_background = false;
            app.mode = AppMode::Normal;
            events.pause();

            // Save conversation to storage before backgrounding so Agent can restore it
            app.force_save_conversation().await;

            let session_id = app.session_id.clone();
            let model = app.model.clone();

            // background_process spawns an Agent and exits - it never returns
            // If it fails, we restore the terminal and show the error
            if let Err(e) = background_process(terminal, &session_id, &model) {
                reinit_terminal(terminal)?;
                events.resume();
                app.set_status(LogLevel::Error, format!("Background failed: {}", e));
            }
            // Note: If background_process succeeds, it calls exit(0), so we never reach here
            continue;
        }

        // Check for pending suspend action
        if app.pending_suspend {
            app.pending_suspend = false;
            app.mode = AppMode::Normal;
            events.pause();
            suspend_process(terminal)?;
            reinit_terminal(terminal)?;
            events.resume();
            continue;
        }

        // Check for pending AI resume (after reattach with user message waiting for response)
        if app.pending_resume_ai {
            app.pending_resume_ai = false;
            // Resume AI response for the last user message
            app.set_status(LogLevel::Info, "Resuming AI response...");
            if let Err(e) = app.call_ai_provider().await {
                app.set_status(LogLevel::Error, format!("Failed to resume AI: {}", e));
            }
        }

        // Poll stream events if streaming is active (non-blocking)
        // This updates the UI content as chunks arrive
        let is_streaming = app.stream_rx.is_some();
        let is_tool_executing = app.tool_rx.is_some();
        let mut content_updated = false;

        if is_streaming {
            // Track if we received new content (for auto-scroll after render)
            let prev_len = app.streaming_content.len();
            app.poll_stream_events().await;
            content_updated = app.streaming_content.len() > prev_len;
        }

        // Poll tool execution events if a tool is running in background
        if is_tool_executing {
            let prev_len = app.streaming_content.len();
            app.poll_tool_events().await;
            content_updated = content_updated || app.streaming_content.len() > prev_len;
        }

        // IPC events now come through the unified Event::Ipc in handle_event
        // No polling needed - fully event-driven!

        // Poll approval requests from tool executor (non-blocking)
        // This shows the approval dialog when a tool needs user approval
        if let Some(ref mut approval_rx) = app.approval_rx
            && let Ok(request) = approval_rx.try_recv()
        {
            // Create approval dialog state and set the pending request
            let mut dialog_state = crate::tui::app::approval_dialog::ApprovalDialogState::new();
            dialog_state.set_request(request);
            app.approval_dialog_state = Some(dialog_state);
            app.mode = AppMode::ApprovalDialog;
            app.add_console_message(format!(
                "⚠️ Tool approval requested: {}",
                app.approval_dialog_state
                    .as_ref()
                    .and_then(|s| s.current_request.as_ref())
                    .map(|r| r.tool_name.as_str())
                    .unwrap_or("unknown")
            ));
        }

        // Poll sudo password requests from tool executor (non-blocking)
        // This shows the sudo password dialog when a bash command needs sudo
        if let Some(ref mut sudo_rx) = app.sudo_password_rx
            && let Ok(request) = sudo_rx.try_recv()
        {
            let command_display = request.command.clone();
            let mut dialog_state = crate::tui::app::sudo_dialog::SudoDialogState::new();
            dialog_state.set_request(request);
            app.sudo_dialog_state = Some(dialog_state);
            app.mode = AppMode::SudoPasswordDialog;
            app.add_console_message(format!(
                "🔒 Sudo password requested for: {}",
                if command_display.len() > 40 {
                    format!("{}...", &command_display[..40])
                } else {
                    command_display
                }
            ));
        }

        // Handle mouse capture toggle
        let should_capture_mouse = !app.mouse_capture_disabled;
        if should_capture_mouse != mouse_capture_enabled {
            if should_capture_mouse {
                let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
            } else {
                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
            }
            mouse_capture_enabled = should_capture_mouse;
        }

        // Check for pending exec command
        if let Some(command) = app.pending_exec_command.take() {
            // Pause event handler while overlay handles events directly
            events.pause();

            // Execute command in full-screen overlay
            let timestamp = chrono::Utc::now().timestamp();
            let result = execute_command_overlay(terminal, &command);

            // Resume event handler
            events.resume();

            match result {
                Ok((output, exit_code)) => {
                    // Store in shell history
                    app.shell_history.push(crate::tui::app::ShellExecution {
                        command: command.clone(),
                        output: output.clone(),
                        exit_code,
                        executed_at: timestamp,
                    });

                    let result_msg = format!(
                        "Command: {}\nExit code: {}\n\nOutput:\n{}",
                        command, exit_code, output
                    );
                    app.messages.push(crate::tui::app::TuiMessage {
                        role: "system".to_string(),
                        content: result_msg,
                        created_at: chrono::Utc::now().timestamp(),
                    });
                    app.set_status(LogLevel::Info, format!("Command executed (exit code: {})", exit_code));
                }
                Err(e) => {
                    // Store failed execution in history too
                    app.shell_history.push(crate::tui::app::ShellExecution {
                        command: command.clone(),
                        output: format!("Error: {}", e),
                        exit_code: -1,
                        executed_at: timestamp,
                    });

                    app.messages.push(crate::tui::app::TuiMessage {
                        role: "system".to_string(),
                        content: format!("Failed to execute command: {}", e),
                        created_at: chrono::Utc::now().timestamp(),
                    });
                    app.set_status(LogLevel::Error, "Command execution failed");
                }
            }
        }

        // Check for pending agent switch
        if let Some(target_session_id) = app.pending_agent_switch.take() {
            use crate::ipc::{is_agent_alive, read_session_token};
            use brainwires::agent_network::ipc::{AgentMessage, Handshake, ViewerMessage};

            // 1. Disconnect from current agent (if connected via IPC)
            if let Some(mut writer) = app.ipc_writer.take() {
                // Send disconnect notification (best effort)
                let _ = writer.write(&ViewerMessage::Disconnect).await;
            }

            // 2. Check if target agent exists and connect
            if !is_agent_alive(&target_session_id).await {
                app.add_console_message(format!(
                    "❌ Agent '{}' is not running. Use /agents to see active agents.",
                    target_session_id
                ));
            } else {
                // Read session token from disk for secure reattachment
                let session_token = match read_session_token(&target_session_id) {
                    Ok(Some(token)) => token,
                    Ok(None) => {
                        app.add_console_message(format!(
                            "❌ No session token found for '{}'. Cannot reattach securely.",
                            target_session_id
                        ));
                        continue;
                    }
                    Err(e) => {
                        app.add_console_message(format!("❌ Failed to read session token: {}", e));
                        continue;
                    }
                };

                match crate::ipc::connect_to_agent(&target_session_id).await {
                    Ok(mut conn) => {
                        // 3. Send handshake with session token
                        let handshake =
                            Handshake::reattach(target_session_id.clone(), session_token);
                        if let Ok(()) = conn.writer.write(&handshake).await {
                            // 4. Receive HandshakeResponse first
                            use brainwires::agent_network::ipc::HandshakeResponse;
                            match conn.reader.read::<HandshakeResponse>().await {
                                Ok(Some(response)) if !response.accepted => {
                                    app.add_console_message(format!(
                                        "❌ Agent rejected reattach: {}",
                                        response
                                            .error
                                            .unwrap_or_else(|| "Unknown error".to_string())
                                    ));
                                    continue;
                                }
                                Ok(None) => {
                                    app.add_console_message(
                                        "❌ Agent connection closed during handshake".to_string(),
                                    );
                                    continue;
                                }
                                Err(e) => {
                                    app.add_console_message(format!(
                                        "❌ Failed to receive handshake response: {}",
                                        e
                                    ));
                                    continue;
                                }
                                Ok(Some(_)) => {
                                    // Handshake accepted, continue to receive ConversationSync
                                }
                            }

                            // 5. Receive ConversationSync
                            match conn.reader.read::<AgentMessage>().await {
                                Ok(Some(AgentMessage::ConversationSync {
                                    messages,
                                    status,
                                    model,
                                    tool_mode,
                                    mcp_servers,
                                    ..
                                })) => {
                                    // 6. Update App state from agent
                                    app.messages = messages
                                        .iter()
                                        .map(|m| crate::tui::app::TuiMessage {
                                            role: m.role.clone(),
                                            content: m.content.clone(),
                                            created_at: m.created_at,
                                        })
                                        .collect();
                                    app.set_status(LogLevel::Info, status);
                                    app.model = model;
                                    app.tool_mode = tool_mode;
                                    app.mcp_connected_servers = mcp_servers;
                                    app.session_id = target_session_id.clone();

                                    // 7. Split connection - store writer, start reader task
                                    let (reader, writer) = conn.split();
                                    app.ipc_writer = Some(writer);
                                    app.is_ipc_mode = true;
                                    app.pending_scroll_to_bottom = true;

                                    // Start new IPC reader task
                                    events.start_ipc_reader(reader);

                                    app.add_console_message(format!(
                                        "✅ Switched to agent: {} ({} messages)",
                                        target_session_id,
                                        app.messages.len()
                                    ));
                                }
                                Ok(Some(other)) => {
                                    app.add_console_message(format!(
                                        "❌ Unexpected response from agent: {:?}",
                                        other
                                    ));
                                }
                                Ok(None) => {
                                    app.add_console_message(
                                        "❌ Agent connection closed after handshake".to_string(),
                                    );
                                }
                                Err(e) => {
                                    app.add_console_message(format!(
                                        "❌ Failed to receive conversation sync: {}",
                                        e
                                    ));
                                }
                            }
                        } else {
                            app.add_console_message(
                                "❌ Failed to send handshake to agent".to_string(),
                            );
                        }
                    }
                    Err(e) => {
                        app.add_console_message(format!(
                            "❌ Failed to connect to agent '{}': {}",
                            target_session_id, e
                        ));
                    }
                }
            }
        }

        // Check for pending agent spawn
        if let Some((model, reason)) = app.pending_agent_spawn.take() {
            use crate::agent::spawn_child_agent;

            let reason_str = reason.as_deref().unwrap_or("user-requested child agent");
            let working_dir = Some(std::path::PathBuf::from(&app.working_directory));

            match spawn_child_agent(&app.session_id, reason_str, model.clone(), working_dir).await {
                Ok((child_session_id, socket_path)) => {
                    app.add_console_message(format!(
                        "✅ Spawned new agent: {}\n   Socket: {}\n   Use /switch {} to connect.",
                        child_session_id,
                        socket_path.display(),
                        child_session_id
                    ));
                }
                Err(e) => {
                    app.add_console_message(format!("❌ Failed to spawn agent: {}", e));
                }
            }
        }

        // Render UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Auto-scroll to bottom AFTER draw, so conversation_area is set correctly
        if content_updated {
            app.scroll_to_bottom();
        }

        // Handle pending scroll to bottom (set when loading a session)
        // This must happen after render so conversation_line_count is available
        // We then re-render immediately so the user sees the scrolled view
        if app.pending_scroll_to_bottom {
            app.pending_scroll_to_bottom = false;
            // First render to get accurate line_count for current terminal size
            terminal.draw(|f| ui::draw(f, app))?;
            // For PTY attach, deduct visible_height to avoid over-scroll
            // For normal session loading, just scroll to bottom normally
            if app.is_pty_session {
                let visible_height = app
                    .conversation_area
                    .map(|a| a.height.saturating_sub(1)) // KEEP AT 1 NOT 2
                    .unwrap_or(20);
                app.scroll = app.max_scroll().saturating_sub(visible_height);
            } else {
                app.scroll_to_bottom();
            }
            // Re-render with scroll applied
            terminal.draw(|f| ui::draw(f, app))?;
        }

        // Handle events with adaptive timeout
        // During streaming or tool execution, use a short timeout to allow frequent re-renders
        // Otherwise, wait normally for the next event
        let needs_fast_polling = is_streaming || is_tool_executing;
        if needs_fast_polling {
            // Short timeout during streaming/tool execution - 16ms (~60fps) for smooth updates
            match timeout(Duration::from_millis(16), events.next()).await {
                Ok(Ok(Some(event))) => {
                    if !app.handle_event(event).await? {
                        break; // Exit requested
                    }
                }
                Ok(Ok(None)) => break, // Channel closed
                Ok(Err(e)) => return Err(e),
                Err(_) => {} // Timeout - just continue the loop to re-render
            }
        } else {
            // Normal mode - wait for event
            match events.next().await {
                Ok(Some(event)) => {
                    if !app.handle_event(event).await? {
                        break; // Exit requested
                    }
                }
                Ok(None) => break, // Channel closed
                Err(e) => return Err(e),
            }
        }

        // Process queued messages if the agent just finished responding
        if app.mode == AppMode::Normal && app.queued_message_count() > 0 {
            let _ = app.process_queued_message().await;
        }

        // Handle Session respawn if needed (IPC disconnected)
        if app.ipc_needs_respawn {
            match app.respawn_session().await {
                Ok(reader) => {
                    // Restart the IPC reader task with the new connection
                    events.start_ipc_reader(reader);
                    app.add_console_message("✅ IPC reader restarted".to_string());
                }
                Err(e) => {
                    app.add_console_message(format!("❌ Session respawn failed: {}", e));
                    app.set_status(LogLevel::Error, format!("Respawn failed: {}", e));
                    app.ipc_needs_respawn = false; // Don't keep trying
                }
            }
        }
    }

    // Stop the event handler and wait for EventStream to be properly dropped.
    // This is critical - EventStream has an internal reader thread that must be
    // stopped before we disable mouse capture, otherwise escape codes leak to stdin.
    events.stop_and_wait().await;

    // Disable mouse capture to tell the terminal to stop sending mouse events.
    let _ = execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste
    );

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);

    // Drain any remaining mouse events after disabling raw mode
    for _ in 0..3 {
        while crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            let _ = crossterm::event::read();
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // If preserve_chat_on_exit is enabled, print the conversation to the console
    if app.preserve_chat_on_exit {
        print_conversation_on_exit(app);
    }

    Ok(TuiLoopResult::Exit)
}

/// Print the conversation to the terminal on exit (when preserve_chat_on_exit is enabled)
fn print_conversation_on_exit(app: &App) {
    use crossterm::style::{Color, ResetColor, SetForegroundColor};

    // Add a blank line before the conversation output
    println!();

    let msg_count = app.messages.len();
    for (i, msg) in app.messages.iter().enumerate() {
        let is_user = msg.role == "user";
        let role_color = match msg.role.as_str() {
            "user" => Color::Green,
            "assistant" => Color::Blue,
            _ => Color::Yellow,
        };

        if is_user {
            // Print "> " prefix only for user messages
            let _ = execute!(io::stdout(), SetForegroundColor(role_color));
            print!("> ");
            let _ = execute!(io::stdout(), ResetColor);
        }

        // Print content
        println!("{}", msg.content);

        // Add blank line between messages, but not after the last one
        if i < msg_count - 1 {
            println!();
        }
    }
}

/// Background the session - spawn a session server that runs the TUI in a PTY
///
/// This spawns a session server process that:
/// 1. Creates a PTY and runs the TUI inside it
/// 2. Listens on a Unix socket for client connections
/// 3. Proxies I/O between clients and the PTY
///
/// The current TUI process then exits, returning control to the shell.
/// Users can reconnect using 'brainwires attach'.
#[cfg(unix)]
fn background_process(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    session_id: &str,
    model: &str,
) -> Result<()> {
    use crate::session;
    use crossterm::{
        cursor::Show,
        event::{DisableBracketedPaste, DisableMouseCapture},
        terminal::disable_raw_mode,
    };

    // Restore terminal to normal state BEFORE spawning session server
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        Show
    )?;

    println!("\nSpawning background session server...");

    // First, save current session state so the new TUI can load it
    // (This is critical for state persistence when backgrounding mid-conversation)

    // Build TUI args to pass to the new TUI instance
    // Pass --session and --pty-session so the new TUI loads state from storage
    // without trying to connect to an IPC Agent
    let mut tui_args = vec![
        "--session".to_string(),
        session_id.to_string(),
        "--pty-session".to_string(),
    ];
    if !model.is_empty() {
        tui_args.push("--model".to_string());
        tui_args.push(model.to_string());
    }

    // Spawn the session server in the background
    // It will create a PTY and run a new TUI instance inside it
    match session::server::spawn_session(Some(session_id.to_string()), tui_args) {
        Ok((session_id, socket_path)) => {
            println!("[brainwires backgrounded]");
            println!("Session: {}", session_id);
            println!("Socket: {}", socket_path.display());
            println!("Use 'brainwires attach {}' to reconnect.\n", session_id);
            println!("Or just 'brainwires attach' to attach to the most recent session.\n");
        }
        Err(e) => {
            println!("Failed to spawn session server: {}", e);
            // Don't exit - return error so TUI can continue
            return Err(e);
        }
    }

    // Exit the TUI process - session server continues running in background
    std::process::exit(0);
}

/// Suspend the process - restore terminal and stop completely
#[cfg(unix)]
fn suspend_process(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    use crossterm::{
        cursor::Show,
        event::{DisableBracketedPaste, DisableMouseCapture},
        terminal::disable_raw_mode,
    };

    // Restore terminal to normal state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        Show
    )?;

    println!("\n[brainwires suspended]");
    println!("Use 'fg' to resume.\n");

    unsafe {
        // Restore default signal handlers so SIGTSTP actually stops the process
        libc::signal(libc::SIGTSTP, libc::SIG_DFL);
        libc::signal(libc::SIGCONT, libc::SIG_DFL);

        // Send SIGTSTP to self - process stops here until SIGCONT from shell's `fg`
        libc::raise(libc::SIGTSTP);

        // After SIGCONT (when user runs `fg`), we continue here
        // Re-ignore terminal signals for TUI operation
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
    }

    Ok(())
}

/// Stub for non-Unix platforms
#[cfg(not(unix))]
fn background_process(
    _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    _session_id: &str,
    _model: &str,
) -> Result<()> {
    anyhow::bail!("Background/suspend not supported on this platform")
}

/// Stub for non-Unix platforms
#[cfg(not(unix))]
fn suspend_process(_terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    anyhow::bail!("Background/suspend not supported on this platform")
}

/// Reinitialize terminal after resume from suspend/background
fn reinit_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    terminal.clear()?;

    Ok(())
}
