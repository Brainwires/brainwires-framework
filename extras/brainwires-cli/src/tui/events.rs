//! Event handling for TUI
//!
//! Handles keyboard and mouse events with async support.
//! Uses crossterm's async EventStream for proper cancellation on shutdown.
//! Supports integrated IPC message handling for event-driven Session communication.

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use brainwires::agent_network::ipc::AgentMessage;

/// TUI Event types
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick for redraws
    Tick,
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(event::MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Paste event (bracketed paste mode or detected rapid input)
    Paste(String),
    /// IPC message from Session (Agent)
    Ipc(AgentMessage),
    /// IPC connection lost - Session needs respawn
    IpcDisconnected,
}

/// Threshold for detecting paste via rapid input (characters arriving faster than this)
const PASTE_CHAR_THRESHOLD_MS: u64 = 15;
/// Minimum characters to consider as a paste (to avoid false positives)
const MIN_PASTE_CHARS: usize = 10;
/// Timeout to wait for more paste input before flushing buffer (accounts for SSH latency)
const PASTE_FLUSH_TIMEOUT_MS: u64 = 100;

/// Event handler with async support
/// Uses crossterm's async EventStream for proper cancellation behavior.
/// Optionally integrates IPC message handling for event-driven Session communication.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    tx: mpsc::UnboundedSender<Event>, // Keep sender alive and allow IPC integration
    paused: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    task_handle: JoinHandle<()>,
    ipc_task_handle: Option<JoinHandle<()>>,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate (ms)
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = Arc::clone(&paused);
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = Arc::clone(&stopped);

        // Spawn event polling task using async EventStream
        // This allows proper cancellation when abort() is called
        let task_handle = tokio::spawn(async move {
            let tick_duration = Duration::from_millis(tick_rate);
            let paste_threshold = Duration::from_millis(PASTE_CHAR_THRESHOLD_MS);

            // Create async event stream
            let mut event_stream = EventStream::new();

            // State for paste detection (when bracketed paste isn't supported)
            let mut paste_buffer = String::new();
            let mut last_char_time: Option<Instant> = None;

            // Interval for tick events and flush timeout
            let mut tick_interval = tokio::time::interval(tick_duration);
            tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                // Check if we should stop
                if stopped_clone.load(Ordering::SeqCst) {
                    break;
                }

                // Skip event polling while paused
                if paused_clone.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Calculate flush timeout for paste buffer
                let flush_timeout = if !paste_buffer.is_empty() {
                    Some(tokio::time::sleep(Duration::from_millis(PASTE_FLUSH_TIMEOUT_MS)))
                } else {
                    None
                };

                tokio::select! {
                    // Event from terminal (async - properly cancellable)
                    maybe_event = event_stream.next() => {
                        match maybe_event {
                            Some(Ok(CrosstermEvent::Key(key))) => {
                                // Check if this is a character we should buffer for paste detection
                                let char_to_buffer = match key.code {
                                    KeyCode::Char(c) => {
                                        // Only batch if no modifiers (except shift for uppercase)
                                        let dominated_by_modifiers = key.modifiers.intersects(
                                            KeyModifiers::CONTROL | KeyModifiers::ALT
                                        );
                                        if !dominated_by_modifiers {
                                            Some(c)
                                        } else {
                                            None
                                        }
                                    }
                                    // Also capture Enter as newline when buffering paste
                                    KeyCode::Enter => {
                                        if !paste_buffer.is_empty() {
                                            Some('\n')
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };

                                if let Some(c) = char_to_buffer {
                                    let now = Instant::now();
                                    let is_rapid = last_char_time
                                        .map(|t| now.duration_since(t) < paste_threshold)
                                        .unwrap_or(false);

                                    if is_rapid || !paste_buffer.is_empty() {
                                        // Continue buffering rapid input
                                        paste_buffer.push(c);
                                        last_char_time = Some(now);
                                        continue; // Don't send individual key event
                                    } else {
                                        // First character - start potential paste buffer
                                        paste_buffer.push(c);
                                        last_char_time = Some(now);
                                        continue;
                                    }
                                }

                                // Non-character key or modified key - flush any buffered paste first
                                if !paste_buffer.is_empty() {
                                    flush_paste_buffer(&event_tx, &mut paste_buffer);
                                    last_char_time = None;
                                }

                                // Now send the actual key event
                                if event_tx.send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(CrosstermEvent::Mouse(mouse))) => {
                                // Flush paste buffer before mouse event
                                if !paste_buffer.is_empty() {
                                    flush_paste_buffer(&event_tx, &mut paste_buffer);
                                    last_char_time = None;
                                }

                                if event_tx.send(Event::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(CrosstermEvent::Resize(width, height))) => {
                                if event_tx.send(Event::Resize(width, height)).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(CrosstermEvent::Paste(text))) => {
                                // Native bracketed paste - clear any buffer and use this
                                paste_buffer.clear();
                                last_char_time = None;
                                if event_tx.send(Event::Paste(text)).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(_)) => {}
                            Some(Err(_)) => {
                                // Event read error - continue
                            }
                            None => {
                                // Stream ended
                                break;
                            }
                        }
                    }

                    // Tick event for periodic redraws
                    _ = tick_interval.tick() => {
                        // Only send tick if paste buffer is empty (otherwise we're waiting for more input)
                        if paste_buffer.is_empty() {
                            if event_tx.send(Event::Tick).is_err() {
                                break;
                            }
                        }
                    }

                    // Flush timeout for paste buffer
                    _ = async {
                        if let Some(timeout) = flush_timeout {
                            timeout.await
                        } else {
                            // Never completes if no timeout
                            std::future::pending::<()>().await
                        }
                    } => {
                        // Timeout with pending paste buffer - flush it
                        if !paste_buffer.is_empty() {
                            flush_paste_buffer(&event_tx, &mut paste_buffer);
                            last_char_time = None;
                        }
                    }
                }
            }
        });

        Self { rx, tx, paused, stopped, task_handle, ipc_task_handle: None }
    }

    /// Start IPC reader task that integrates Session messages into the event stream
    ///
    /// This spawns a background task that reads from the IPC connection and sends
    /// messages as Event::Ipc variants. This makes IPC fully event-driven - no polling needed.
    pub fn start_ipc_reader(&mut self, mut reader: brainwires::agent_network::ipc::IpcReader) {
        let event_tx = self.tx.clone();
        let stopped = Arc::clone(&self.stopped);

        let handle = tokio::spawn(async move {
            loop {
                // Check if we should stop
                if stopped.load(Ordering::SeqCst) {
                    break;
                }

                // Read next message from IPC (this is async, waits for data)
                match reader.read::<AgentMessage>().await {
                    Ok(Some(msg)) => {
                        // Send as IPC event
                        if event_tx.send(Event::Ipc(msg)).is_err() {
                            break; // Event channel closed
                        }
                    }
                    Ok(None) => {
                        // Connection closed gracefully
                        let _ = event_tx.send(Event::IpcDisconnected);
                        break;
                    }
                    Err(_e) => {
                        // Connection error
                        let _ = event_tx.send(Event::IpcDisconnected);
                        break;
                    }
                }
            }
        });

        self.ipc_task_handle = Some(handle);
    }

    /// Get a clone of the event sender for external IPC integration
    ///
    /// This allows the IPC reader to send events without being part of EventHandler
    pub fn event_sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    /// Stop the event handler (terminates all background tasks)
    /// This sets the stopped flag which causes the async loop to exit gracefully,
    /// ensuring EventStream is properly dropped (which cleans up its internal reader thread).
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        // Don't abort - let the task exit gracefully so EventStream can clean up
        // The stopped flag check in the loop will cause it to break and drop EventStream

        // Abort the IPC reader task if running (it doesn't have cleanup concerns)
        if let Some(ref handle) = self.ipc_task_handle {
            handle.abort();
        }
    }

    /// Stop and wait for the event handler to fully terminate
    /// This ensures EventStream is properly dropped before returning.
    pub async fn stop_and_wait(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);

        // Abort the IPC reader task if running
        if let Some(ref handle) = self.ipc_task_handle {
            handle.abort();
        }

        // Wait briefly for the terminal event task to notice the stopped flag and exit
        // This gives EventStream a chance to be dropped properly
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Now abort if it hasn't exited yet
        self.task_handle.abort();

        // Wait for abort to complete
        let _ = (&mut self.task_handle).await;
    }

    /// Pause event polling (for overlays that handle events directly)
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    /// Resume event polling
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    /// Get the next event
    pub async fn next(&mut self) -> Result<Option<Event>> {
        Ok(self.rx.recv().await)
    }
}

/// Helper to flush paste buffer - sends as Paste event if long enough, otherwise individual keys
fn flush_paste_buffer(event_tx: &mpsc::UnboundedSender<Event>, paste_buffer: &mut String) {
    if paste_buffer.len() >= MIN_PASTE_CHARS {
        // Send as paste event
        let _ = event_tx.send(Event::Paste(paste_buffer.clone()));
    } else {
        // Too short, send as individual key events
        for ch in paste_buffer.chars() {
            let key_code = if ch == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(ch)
            };
            let char_key = KeyEvent::new(key_code, KeyModifiers::NONE);
            if event_tx.send(Event::Key(char_key)).is_err() {
                break;
            }
        }
    }
    paste_buffer.clear();
}

/// Key binding helpers
impl Event {
    /// Check if event is Ctrl+C
    pub fn is_quit(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+R (reverse search)
    pub fn is_reverse_search(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+L (session picker)
    pub fn is_session_picker(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+D (console view)
    pub fn is_console_view(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+T (task viewer)
    pub fn is_task_viewer(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('t'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+Z (suspend/background dialog)
    pub fn is_suspend(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('z'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+P (toggle plan mode)
    pub fn is_plan_mode_toggle(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is 'c' key (copy in console view)
    pub fn is_copy(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event is 'm' key (toggle mouse capture for text selection)
    pub fn is_toggle_mouse(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event is Enter (without Shift or Alt modifiers)
    pub fn is_enter(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers,
                ..
            }) if !modifiers.contains(KeyModifiers::SHIFT) && !modifiers.contains(KeyModifiers::ALT)
        )
    }

    /// Check if event is Shift+Enter, Alt+Enter, or Ctrl+J (for multi-line input)
    /// Note: Many terminals don't send SHIFT with Enter, so we support Alt+Enter and Ctrl+J as alternatives
    pub fn is_shift_enter(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::SHIFT) || modifiers.contains(KeyModifiers::ALT)
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Escape
    pub fn is_escape(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                ..
            })
        )
    }

    /// Check if event is Ctrl+Alt+F or F10 (for fullscreen toggle)
    pub fn is_fullscreen_toggle(&self) -> bool {
        // F10
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::F(10),
                ..
            })
        ) ||
        // Ctrl+Alt+F
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL) && modifiers.contains(KeyModifiers::ALT)
        )
    }

    /// Check if event is F9 (for conversation view style toggle)
    pub fn is_view_style_toggle(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::F(9),
                ..
            })
        )
    }

    /// Check if event is Tab (without Shift)
    pub fn is_tab(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Tab,
                modifiers,
                ..
            }) if !modifiers.contains(KeyModifiers::SHIFT)
        )
    }

    /// Check if event is Shift+Tab (BackTab)
    pub fn is_shift_tab(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::BackTab,
                ..
            })
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Tab,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::SHIFT)
        )
    }

    /// Check if event is Backspace
    pub fn is_backspace(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            })
        )
    }

    /// Check if event is Up arrow
    pub fn is_up(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                ..
            })
        )
    }

    /// Check if event is Down arrow
    pub fn is_down(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            })
        )
    }

    /// Check if event is PageUp
    pub fn is_page_up(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                ..
            })
        )
    }

    /// Check if event is PageDown
    pub fn is_page_down(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::PageDown,
                ..
            })
        )
    }

    /// Get character if this is a character key press
    pub fn char(&self) -> Option<char> {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }) = self
        {
            Some(*c)
        } else {
            None
        }
    }

    /// Check if event is mouse scroll up and get position
    pub fn mouse_scroll_up(&self) -> Option<(u16, u16)> {
        if let Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::ScrollUp,
            column,
            row,
            ..
        }) = self
        {
            Some((*column, *row))
        } else {
            None
        }
    }

    /// Check if event is mouse scroll down and get position
    pub fn mouse_scroll_down(&self) -> Option<(u16, u16)> {
        if let Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::ScrollDown,
            column,
            row,
            ..
        }) = self
        {
            Some((*column, *row))
        } else {
            None
        }
    }

    /// Check if event is a left mouse button click and get position
    pub fn mouse_click(&self) -> Option<(u16, u16)> {
        if let Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::Down(event::MouseButton::Left),
            column,
            row,
            ..
        }) = self
        {
            Some((*column, *row))
        } else {
            None
        }
    }

    // === Advanced Text Editing Shortcuts ===

    /// Check if event is Alt+Backspace or Ctrl+W (delete word backward)
    pub fn is_delete_word_backward(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::ALT)
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Alt+Delete or Ctrl+D (delete word forward)
    pub fn is_delete_word_forward(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Delete,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::ALT)
        )
    }

    /// Check if event is Ctrl+U (delete from cursor to start of line)
    pub fn is_delete_to_start(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+K (delete from cursor to end of line)
    pub fn is_delete_to_end(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+A (move to start of line)
    pub fn is_move_to_line_start(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+E (move to end of line)
    pub fn is_move_to_line_end(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Alt+Left or Ctrl+Left (move word backward)
    pub fn is_move_word_backward(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::ALT) || modifiers.contains(KeyModifiers::CONTROL)
        )
    }

    /// Check if event is Alt+Right or Ctrl+Right (move word forward)
    pub fn is_move_word_forward(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::ALT) || modifiers.contains(KeyModifiers::CONTROL)
        )
    }

    /// Check if event is Ctrl+Home, Ctrl+Up, Alt+Shift+Up (move to document start)
    /// Note: Cmd+Up is intercepted by macOS and won't reach the terminal
    pub fn is_move_to_document_start(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Home,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
                || modifiers.contains(KeyModifiers::SUPER)
                || (modifiers.contains(KeyModifiers::ALT) && modifiers.contains(KeyModifiers::SHIFT))
        )
    }

    /// Check if event is Ctrl+End, Ctrl+Down, Alt+Shift+Down (move to document end)
    /// Note: Cmd+Down is intercepted by macOS and won't reach the terminal
    pub fn is_move_to_document_end(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::End,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
                || modifiers.contains(KeyModifiers::SUPER)
                || (modifiers.contains(KeyModifiers::ALT) && modifiers.contains(KeyModifiers::SHIFT))
        )
    }

    // === File Explorer and Editor Shortcuts ===

    /// Check if event is Ctrl+Alt+F (file explorer)
    pub fn is_file_explorer(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL) && modifiers.contains(KeyModifiers::ALT)
        )
    }

    // === Find/Replace Shortcuts ===

    /// Check if event is Ctrl+F (find dialog)
    pub fn is_find(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+H (find and replace dialog)
    pub fn is_replace(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is F3 (next match in find)
    pub fn is_find_next(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::F(3),
                modifiers,
                ..
            }) if !modifiers.contains(KeyModifiers::SHIFT)
        )
    }

    /// Check if event is Shift+F3 (previous match in find)
    pub fn is_find_prev(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::F(3),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::SHIFT)
        )
    }

    /// Check if event is Ctrl+S (save)
    pub fn is_save(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+X (exit editor / cut in nano)
    pub fn is_ctrl_x(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+O (write out in nano)
    pub fn is_ctrl_o(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is Ctrl+G (git scm)
    pub fn is_git_scm(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }

    /// Check if event is F1 (help dialog)
    pub fn is_help(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::F(1),
                ..
            })
        )
    }

    // === Journal Tree Navigation ===

    /// Check if event expands a journal tree node ('l' or Right arrow, no mods)
    pub fn is_journal_expand(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event collapses a journal tree node ('h' or Left arrow, no mods)
    pub fn is_journal_collapse(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        ) || matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event moves journal cursor down ('j' key, no mods)
    pub fn is_journal_cursor_down(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event moves journal cursor up ('k' key, no mods)
    pub fn is_journal_cursor_up(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
    }

    /// Check if event is Ctrl+B (Sub-Agent Viewer)
    pub fn is_sub_agent_viewer(&self) -> bool {
        matches!(
            self,
            Event::Key(KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: event::KeyEventKind::Press,
            state: event::KeyEventState::empty(),
        })
    }

    #[test]
    fn test_is_quit() {
        assert!(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL).is_quit());
        assert!(!key_event(KeyCode::Char('c'), KeyModifiers::NONE).is_quit());
        assert!(!key_event(KeyCode::Char('q'), KeyModifiers::CONTROL).is_quit());
    }

    #[test]
    fn test_is_reverse_search() {
        assert!(key_event(KeyCode::Char('r'), KeyModifiers::CONTROL).is_reverse_search());
        assert!(!key_event(KeyCode::Char('r'), KeyModifiers::NONE).is_reverse_search());
    }

    #[test]
    fn test_is_session_picker() {
        assert!(key_event(KeyCode::Char('l'), KeyModifiers::CONTROL).is_session_picker());
        assert!(!key_event(KeyCode::Char('l'), KeyModifiers::NONE).is_session_picker());
    }

    #[test]
    fn test_is_enter() {
        assert!(key_event(KeyCode::Enter, KeyModifiers::NONE).is_enter());
        assert!(!key_event(KeyCode::Char('a'), KeyModifiers::NONE).is_enter());
    }

    #[test]
    fn test_is_backspace() {
        assert!(key_event(KeyCode::Backspace, KeyModifiers::NONE).is_backspace());
        assert!(!key_event(KeyCode::Delete, KeyModifiers::NONE).is_backspace());
    }

    #[test]
    fn test_is_escape() {
        assert!(key_event(KeyCode::Esc, KeyModifiers::NONE).is_escape());
        assert!(!key_event(KeyCode::Char('a'), KeyModifiers::NONE).is_escape());
    }

    #[test]
    fn test_is_tab() {
        assert!(key_event(KeyCode::Tab, KeyModifiers::NONE).is_tab());
        assert!(!key_event(KeyCode::Char('a'), KeyModifiers::NONE).is_tab());
    }

    #[test]
    fn test_is_up_down() {
        assert!(key_event(KeyCode::Up, KeyModifiers::NONE).is_up());
        assert!(key_event(KeyCode::Down, KeyModifiers::NONE).is_down());
        assert!(!key_event(KeyCode::Left, KeyModifiers::NONE).is_up());
    }

    #[test]
    fn test_is_shift_enter() {
        assert!(key_event(KeyCode::Enter, KeyModifiers::SHIFT).is_shift_enter());
        assert!(!key_event(KeyCode::Enter, KeyModifiers::NONE).is_shift_enter());
    }

    #[test]
    fn test_is_console_view() {
        assert!(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL).is_console_view());
        assert!(!key_event(KeyCode::Char('d'), KeyModifiers::NONE).is_console_view());
    }

    #[test]
    fn test_is_page_up_down() {
        assert!(key_event(KeyCode::PageUp, KeyModifiers::NONE).is_page_up());
        assert!(key_event(KeyCode::PageDown, KeyModifiers::NONE).is_page_down());
        assert!(!key_event(KeyCode::PageUp, KeyModifiers::NONE).is_page_down());
    }

    #[test]
    fn test_char() {
        assert_eq!(key_event(KeyCode::Char('a'), KeyModifiers::NONE).char(), Some('a'));
        assert_eq!(key_event(KeyCode::Char('Z'), KeyModifiers::SHIFT).char(), Some('Z'));
        assert_eq!(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL).char(), None);
        assert_eq!(key_event(KeyCode::Enter, KeyModifiers::NONE).char(), None);
    }

    #[test]
    fn test_is_delete_word_backward() {
        assert!(key_event(KeyCode::Backspace, KeyModifiers::ALT).is_delete_word_backward());
        assert!(key_event(KeyCode::Char('w'), KeyModifiers::CONTROL).is_delete_word_backward());
        assert!(!key_event(KeyCode::Backspace, KeyModifiers::NONE).is_delete_word_backward());
    }

    #[test]
    fn test_is_delete_word_forward() {
        assert!(key_event(KeyCode::Delete, KeyModifiers::ALT).is_delete_word_forward());
        assert!(!key_event(KeyCode::Delete, KeyModifiers::NONE).is_delete_word_forward());
    }

    #[test]
    fn test_is_delete_to_start() {
        assert!(key_event(KeyCode::Char('u'), KeyModifiers::CONTROL).is_delete_to_start());
        assert!(!key_event(KeyCode::Char('u'), KeyModifiers::NONE).is_delete_to_start());
    }

    #[test]
    fn test_is_delete_to_end() {
        assert!(key_event(KeyCode::Char('k'), KeyModifiers::CONTROL).is_delete_to_end());
        assert!(!key_event(KeyCode::Char('k'), KeyModifiers::NONE).is_delete_to_end());
    }

    #[test]
    fn test_is_move_to_line_start() {
        assert!(key_event(KeyCode::Char('a'), KeyModifiers::CONTROL).is_move_to_line_start());
        assert!(!key_event(KeyCode::Char('a'), KeyModifiers::NONE).is_move_to_line_start());
    }

    #[test]
    fn test_is_move_to_line_end() {
        assert!(key_event(KeyCode::Char('e'), KeyModifiers::CONTROL).is_move_to_line_end());
        assert!(!key_event(KeyCode::Char('e'), KeyModifiers::NONE).is_move_to_line_end());
    }

    #[test]
    fn test_is_move_word_backward() {
        assert!(key_event(KeyCode::Left, KeyModifiers::ALT).is_move_word_backward());
        assert!(key_event(KeyCode::Left, KeyModifiers::CONTROL).is_move_word_backward());
        assert!(!key_event(KeyCode::Left, KeyModifiers::NONE).is_move_word_backward());
    }

    #[test]
    fn test_is_move_word_forward() {
        assert!(key_event(KeyCode::Right, KeyModifiers::ALT).is_move_word_forward());
        assert!(key_event(KeyCode::Right, KeyModifiers::CONTROL).is_move_word_forward());
        assert!(!key_event(KeyCode::Right, KeyModifiers::NONE).is_move_word_forward());
    }

    // Note: EventHandler tests require a tokio runtime and are tested in integration tests

    #[test]
    fn test_mouse_scroll_up() {
        let event = Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::ScrollUp,
            column: 10,
            row: 20,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(event.mouse_scroll_up(), Some((10, 20)));

        let event = Event::Tick;
        assert_eq!(event.mouse_scroll_up(), None);
    }

    #[test]
    fn test_mouse_scroll_down() {
        let event = Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::ScrollDown,
            column: 15,
            row: 25,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(event.mouse_scroll_down(), Some((15, 25)));

        let event = Event::Tick;
        assert_eq!(event.mouse_scroll_down(), None);
    }

    #[test]
    fn test_mouse_click() {
        let event = Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::Down(event::MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(event.mouse_click(), Some((5, 10)));

        // Right click should not match
        let event = Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::Down(event::MouseButton::Right),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(event.mouse_click(), None);

        // Mouse up should not match
        let event = Event::Mouse(event::MouseEvent {
            kind: event::MouseEventKind::Up(event::MouseButton::Left),
            column: 5,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(event.mouse_click(), None);

        let event = Event::Tick;
        assert_eq!(event.mouse_click(), None);
    }

    #[test]
    fn test_is_file_explorer() {
        // File explorer requires Ctrl+Alt+F
        assert!(key_event(KeyCode::Char('f'), KeyModifiers::CONTROL | KeyModifiers::ALT).is_file_explorer());
        assert!(!key_event(KeyCode::Char('f'), KeyModifiers::CONTROL).is_file_explorer()); // Ctrl+F alone is find
        assert!(!key_event(KeyCode::Char('f'), KeyModifiers::ALT).is_file_explorer()); // Alt+F alone is not file explorer
        assert!(!key_event(KeyCode::Char('f'), KeyModifiers::NONE).is_file_explorer());
    }

    #[test]
    fn test_is_save() {
        assert!(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL).is_save());
        assert!(!key_event(KeyCode::Char('s'), KeyModifiers::NONE).is_save());
    }

    #[test]
    fn test_is_ctrl_x() {
        assert!(key_event(KeyCode::Char('x'), KeyModifiers::CONTROL).is_ctrl_x());
        assert!(!key_event(KeyCode::Char('x'), KeyModifiers::NONE).is_ctrl_x());
    }

    #[test]
    fn test_is_ctrl_o() {
        assert!(key_event(KeyCode::Char('o'), KeyModifiers::CONTROL).is_ctrl_o());
        assert!(!key_event(KeyCode::Char('o'), KeyModifiers::NONE).is_ctrl_o());
    }

    #[test]
    fn test_is_git_scm() {
        assert!(key_event(KeyCode::Char('g'), KeyModifiers::CONTROL).is_git_scm());
        assert!(!key_event(KeyCode::Char('g'), KeyModifiers::NONE).is_git_scm());
    }

    #[test]
    fn test_is_help() {
        assert!(key_event(KeyCode::F(1), KeyModifiers::NONE).is_help());
        assert!(!key_event(KeyCode::Char('?'), KeyModifiers::NONE).is_help()); // ? is for typing questions!
        assert!(!key_event(KeyCode::Char('h'), KeyModifiers::NONE).is_help());
        assert!(!key_event(KeyCode::F(2), KeyModifiers::NONE).is_help());
    }
}
