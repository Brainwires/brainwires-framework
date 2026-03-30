//! Session Client
//!
//! Connects to a session server and provides terminal I/O.

use std::io::{Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Context, Result};
use nix::sys::termios::{self, SetArg, Termios};

/// Global flag to signal window resize event from SIGWINCH handler
static WINSIZE_CHANGED: AtomicBool = AtomicBool::new(false);

/// SIGWINCH signal handler - sets flag to trigger window size update
extern "C" fn handle_sigwinch(_: libc::c_int) {
    WINSIZE_CHANGED.store(true, Ordering::SeqCst);
}

/// Enable verbose logging for debugging
const DEBUG_LOGGING: bool = false;

fn client_log(msg: &str) {
    if DEBUG_LOGGING {
        // Log to stderr so it doesn't interfere with TUI output
        // But since we're in raw mode, this might not display well
        // Let's log to a file instead
        use std::fs::OpenOptions;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/brainwires_client.log")
        {
            let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%H:%M:%S%.3f"), msg);
        }
    }
}

use chrono;

/// Session client that connects to a server
pub struct SessionClient {
    session_id: String,
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    original_termios: Option<Termios>,
}

impl SessionClient {
    /// Connect to an existing session
    pub fn connect(session_id: &str) -> Result<Self> {
        let socket_path = super::get_session_socket_path(session_id)?;

        if !socket_path.exists() {
            bail!("Session not found: {}", session_id);
        }

        let stream = UnixStream::connect(&socket_path)
            .with_context(|| format!("Failed to connect to session: {}", session_id))?;

        Ok(Self {
            session_id: session_id.to_string(),
            socket_path,
            stream: Some(stream),
            original_termios: None,
        })
    }

    /// Run the client - proxy I/O between terminal and server
    pub fn run(&mut self) -> Result<()> {
        client_log("Client run() starting");

        // Take the stream and set non-blocking
        let mut stream = self.stream.take().context("Not connected")?;
        stream.set_nonblocking(true)?;
        let socket_fd = stream.as_raw_fd();
        client_log(&format!("Socket FD: {}", socket_fd));

        // Set terminal to raw mode
        self.set_raw_mode()?;
        client_log("Terminal in raw mode");

        // Get the raw fds
        let stdin_fd = std::io::stdin().as_raw_fd();
        let stdout_fd = std::io::stdout().as_raw_fd();
        client_log(&format!("stdin_fd={}, stdout_fd={}", stdin_fd, stdout_fd));

        // Set stdin non-blocking
        unsafe {
            let flags = libc::fcntl(stdin_fd, libc::F_GETFL);
            libc::fcntl(stdin_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let mut stdin_buf = [0u8; 4096];
        let mut socket_buf = [0u8; 4096];

        // Send initial window size
        self.send_winsize_to(&mut stream)?;
        client_log("Sent initial winsize");

        // Install SIGWINCH handler to detect terminal resize
        unsafe {
            libc::signal(libc::SIGWINCH, handle_sigwinch as libc::sighandler_t);
        }
        client_log("Installed SIGWINCH handler");

        let session_id = self.session_id.clone();
        let mut loop_count = 0u64;
        let mut last_log = std::time::Instant::now();

        loop {
            loop_count += 1;

            // Log every second
            if last_log.elapsed().as_secs() >= 1 {
                client_log(&format!("Loop iteration {}", loop_count));
                last_log = std::time::Instant::now();
            }

            // Check if window size changed (SIGWINCH received)
            if WINSIZE_CHANGED.swap(false, Ordering::SeqCst) {
                client_log("SIGWINCH detected - sending new window size");
                if let Err(e) = self.send_winsize_to(&mut stream) {
                    client_log(&format!("Failed to send window size: {}", e));
                }
            }

            // Read from stdin, send to socket
            let n = unsafe {
                libc::read(stdin_fd, stdin_buf.as_mut_ptr() as *mut libc::c_void, stdin_buf.len())
            };

            if n > 0 {
                let n = n as usize;
                client_log(&format!("stdin -> socket: {} bytes", n));

                if let Err(e) = stream.write_all(&stdin_buf[..n]) {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        client_log(&format!("Socket write error: {}", e));
                        bail!("Write to server failed: {}", e);
                    }
                }
            }

            // Read from socket, write to stdout
            match stream.read(&mut socket_buf) {
                Ok(0) => {
                    // Server closed connection
                    client_log("Server closed connection");
                    println!("\r\n[session ended]");
                    break;
                }
                Ok(n) => {
                    client_log(&format!("socket -> stdout: {} bytes", n));
                    unsafe {
                        libc::write(stdout_fd, socket_buf.as_ptr() as *const libc::c_void, n);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    client_log(&format!("Socket read error: {}", e));
                    bail!("Read from server failed: {}", e);
                }
            }

            // Small sleep to avoid busy loop
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        client_log("Client run() exiting");
        Ok(())
    }

    /// Set terminal to raw mode
    fn set_raw_mode(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let stdin_fd = unsafe { BorrowedFd::borrow_raw(stdin.as_raw_fd()) };

        // Save original termios
        let original = termios::tcgetattr(&stdin_fd)?;
        self.original_termios = Some(original.clone());

        // Set raw mode
        let mut raw = original;
        termios::cfmakeraw(&mut raw);
        termios::tcsetattr(&stdin_fd, SetArg::TCSANOW, &raw)?;

        Ok(())
    }

    /// Restore terminal mode
    fn restore_terminal(&mut self) {
        if let Some(ref original) = self.original_termios {
            let stdin = std::io::stdin();
            let stdin_fd = unsafe { BorrowedFd::borrow_raw(stdin.as_raw_fd()) };
            let _ = termios::tcsetattr(&stdin_fd, SetArg::TCSANOW, original);
        }
    }

    /// Send window size to server
    fn send_winsize_to(&self, stream: &mut UnixStream) -> Result<()> {
        // Get current window size
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        unsafe {
            libc::ioctl(std::io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut ws);
        }

        client_log(&format!("Window size: {}x{}", ws.ws_col, ws.ws_row));

        // Send window size as a special message
        // Format: [WINSIZE_MAGIC][cols:u16][rows:u16]
        // The server will detect this and update the PTY size
        let magic = [0x1b, 0x5d, 0x57, 0x53]; // ESC ] W S - custom escape sequence
        let cols = ws.ws_col.to_be_bytes();
        let rows = ws.ws_row.to_be_bytes();
        let msg = [magic[0], magic[1], magic[2], magic[3], cols[0], cols[1], rows[0], rows[1]];

        stream.write_all(&msg)?;
        client_log("Sent window size to server");

        Ok(())
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl Drop for SessionClient {
    fn drop(&mut self) {
        self.restore_terminal();
    }
}

/// Attach to a session (or the most recent one)
pub fn attach(session_id: Option<&str>) -> Result<()> {
    let session_id = if let Some(id) = session_id {
        id.to_string()
    } else {
        // Find most recent session
        let sessions = super::list_sessions()?;
        if sessions.is_empty() {
            bail!("No active sessions found");
        }
        // Return the first (most recent based on naming convention)
        sessions.into_iter().next().unwrap()
    };

    // Try to connect to the PTY socket
    match SessionClient::connect(&session_id) {
        Ok(mut client) => {
            println!("[attached to session {}]", session_id);
            client.run()
        }
        Err(e) => {
            // Check if the agent is still alive even though PTY socket failed
            let sessions_dir = crate::utils::paths::PlatformPaths::sessions_dir()?;
            let ipc_socket = sessions_dir.join(format!("{}.sock", session_id));

            if ipc_socket.exists() {
                // Try connecting to IPC socket to verify agent is alive
                use std::os::unix::net::UnixStream;
                use std::time::Duration;
                if let Ok(stream) = UnixStream::connect(&ipc_socket) {
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                    // Agent is alive but PTY isn't responding
                    println!("Session {} has a running agent but no PTY server.", session_id);
                    println!();
                    println!("This can happen when:");
                    println!("  - The TUI was suspended with Ctrl+Z and resumed with 'bg'");
                    println!("  - The PTY server crashed but the agent continued");
                    println!();
                    println!("Options:");
                    println!("  1. Use the GUI/web interface to interact with this agent");
                    println!("  2. Kill this session and start a new one:");
                    println!("     brainwires kill {}", session_id);
                    println!("     brainwires chat");
                    println!();
                    println!("Tip: To properly background a TUI session, use the /background");
                    println!("     slash command instead of Ctrl+Z.");
                    return Ok(());
                }
            }
            // No agent either, return original error
            Err(e)
        }
    }
}
