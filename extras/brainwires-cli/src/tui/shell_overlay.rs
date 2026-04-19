//! Interactive shell overlay — drop into a live `bash` (or `$SHELL`) session
//! from inside the TUI. The shell owns the terminal for its lifetime; we
//! relinquish raw mode + alternate screen + mouse capture while it runs,
//! then restore on return.
//!
//! This is deliberately simpler than [`exec_overlay`](crate::tui::exec_overlay):
//! that module keeps raw mode active and captures output. Here we need the
//! shell to have direct TTY access — prompts, ncurses, color, SIGWINCH, the
//! whole lot — so we hand the terminal over wholesale.
//!
//! Gated to Unix. Windows gets a clear error message from the caller.

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus};

/// Result of a shell invocation.
pub struct ShellResult {
    /// `sh`/`bash` exit code. `None` if terminated by a signal.
    pub exit_code: Option<i32>,
    /// Raw `ExitStatus` — surface to the caller for richer logging if needed.
    pub status: ExitStatus,
}

/// Run an interactive shell with the terminal fully handed over. Returns
/// the shell's exit code (or `None` if signalled). Restores the TUI
/// terminal state on return, even if the spawn fails.
///
/// - `cwd`: working directory for the shell.
/// - `shell`: explicit program path; defaults to `$SHELL` then `/bin/bash`.
#[cfg(unix)]
pub fn run_interactive_shell(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cwd: &Path,
    shell: Option<&str>,
) -> Result<ShellResult> {
    let program = shell
        .map(String::from)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());

    // Tear down the TUI terminal state so the shell gets a clean TTY.
    {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = disable_raw_mode();
    }

    // Register a panic guard so any panic between here and restore leaves
    // the terminal in a sane state. Dropped at the end of this scope.
    struct RestoreGuard;
    impl Drop for RestoreGuard {
        fn drop(&mut self) {
            // Best-effort only; errors here are logged, not surfaced.
            let mut stdout = io::stdout();
            let _ = enable_raw_mode();
            let _ = execute!(stdout, EnterAlternateScreen, EnableMouseCapture);
        }
    }
    let _restore = RestoreGuard;

    // Print a one-line banner so the user knows which shell they're in.
    eprintln!("(brainwires) interactive shell — type `exit` or press Ctrl+D to return");

    // Spawn with stdio inherited so the child owns the TTY directly.
    // `.status()` blocks until the child exits; SIGWINCH / job-control /
    // color / prompts all work because we've released the terminal.
    let status = Command::new(&program)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to spawn shell: {}", program))?;

    // RestoreGuard drops here → raw mode + alt screen + mouse capture come back.
    // Redraw so ratatui's cached frame matches reality.
    drop(_restore);
    // One more clear for a fresh paint; the guard already flipped modes back.
    terminal.clear().ok();

    Ok(ShellResult {
        exit_code: status.code(),
        status,
    })
}

/// Windows stub — explicitly unsupported for now. Callers should gate their
/// `/shell` dispatch with a user-friendly error rather than calling this.
#[cfg(not(unix))]
pub fn run_interactive_shell(
    _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    _cwd: &Path,
    _shell: Option<&str>,
) -> Result<ShellResult> {
    anyhow::bail!("/shell is not yet supported on this platform")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// Smoke: the helper function can parse a program string from `$SHELL`
    /// fallback chain. We can't actually run an interactive shell inside
    /// `cargo test` (no TTY) so the behavior test stays manual — this just
    /// verifies the module compiles and the signature is stable.
    #[test]
    fn module_exports_and_signature_compile() {
        // This is a compile-time-only check — calling would require a
        // real terminal handle. Confirms public surface.
        fn _assert_sig(
            t: &mut Terminal<CrosstermBackend<io::Stdout>>,
            p: &Path,
        ) -> Result<ShellResult> {
            run_interactive_shell(t, p, None)
        }
    }
}
