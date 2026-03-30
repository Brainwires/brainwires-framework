use console::style;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Track if logger has been initialized
static LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the logger
pub fn init() {
    init_with_output(true);
}

/// Initialize the logger with optional output
/// When `enable_output` is false, console tracing will be disabled (useful for TUI mode)
/// File logging is ALWAYS enabled regardless of enable_output
pub fn init_with_output(enable_output: bool) {
    // Only initialize once to avoid panic
    if LOGGER_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Create logs directory
    let log_dir = get_log_directory().unwrap_or_else(|_| PathBuf::from(".brainwires/logs"));
    let _ = std::fs::create_dir_all(&log_dir);

    // Create file appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "brainwires.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Filter level based on environment
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("brainwires_cli=debug,info"));

    if !enable_output {
        // TUI mode: Only log to file, disable console
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(non_blocking_file)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .with_file(true),
            )
            .init();
    } else {
        // CLI mode: Log to both file and STDERR (never stdout - MCP uses stdout for protocol)
        let (non_blocking_stderr, _stderr_guard) = tracing_appender::non_blocking(std::io::stderr());

        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(non_blocking_file)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .with_file(true),
            )
            .with(
                fmt::layer()
                    .with_writer(non_blocking_stderr)  // Use stderr, not stdout
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_line_number(false)
                    .with_file(false),
            )
            .init();

        std::mem::forget(_stderr_guard);
    }

    // Leak the guard so it persists for the program lifetime
    std::mem::forget(_guard);

    tracing::info!("Logging initialized - files: {}", log_dir.display());
}

/// Logger utility for pretty terminal output
pub struct Logger;

impl Logger {
    /// Log an info message
    pub fn info<S: AsRef<str>>(message: S) {
        println!("{} {}", style("ℹ").blue(), message.as_ref());
    }

    /// Log a success message
    pub fn success<S: AsRef<str>>(message: S) {
        println!("{} {}", style("✓").green(), message.as_ref());
    }

    /// Log a warning message
    pub fn warn<S: AsRef<str>>(message: S) {
        eprintln!("{} {}", style("⚠").yellow(), message.as_ref());
    }

    /// Log an error message
    pub fn error<S: AsRef<str>>(message: S) {
        eprintln!("{} {}", style("✗").red(), message.as_ref());
    }

    /// Log a debug message (only in debug builds)
    pub fn debug<S: AsRef<str>>(message: S) {
        if cfg!(debug_assertions) {
            println!("{} {}", style("🐛").dim(), style(message.as_ref()).dim());
        }
    }

    /// Print a blank line
    pub fn newline() {
        println!();
    }

    /// Create a spinner (uses indicatif)
    pub fn spinner<S: Into<String>>(message: S) -> indicatif::ProgressBar {
        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.into());
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        spinner
    }

    /// Write text without a newline (for streaming)
    pub fn write<S: AsRef<str>>(text: S) {
        print!("{}", text.as_ref());
        let _ = std::io::stdout().flush();
    }

    /// Write styled text
    pub fn write_styled<S: AsRef<str>>(text: S, color: &str) {
        let styled = match color {
            "red" => style(text.as_ref()).red(),
            "green" => style(text.as_ref()).green(),
            "blue" => style(text.as_ref()).blue(),
            "yellow" => style(text.as_ref()).yellow(),
            "cyan" => style(text.as_ref()).cyan(),
            "magenta" => style(text.as_ref()).magenta(),
            "white" => style(text.as_ref()).white(),
            "gray" => style(text.as_ref()).dim(),
            "bold" => style(text.as_ref()).bold(),
            _ => style(text.as_ref()),
        };
        print!("{}", styled);
        let _ = std::io::stdout().flush();
    }
}

// Re-export for convenience
pub use Logger as logger;

/// Get the log directory path: ~/.brainwires/logs/
fn get_log_directory() -> Result<PathBuf, std::env::VarError> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    Ok(PathBuf::from(home).join(".brainwires").join("logs"))
}

/// Get path to current log file (for user reference)
pub fn get_current_log_file() -> Option<PathBuf> {
    let log_dir = get_log_directory().ok()?;
    let date = chrono::Local::now().format("%Y-%m-%d");
    Some(log_dir.join(format!("brainwires.log.{}", date)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger() {
        // Just ensure they don't panic
        Logger::info("Test info");
        Logger::success("Test success");
        Logger::warn("Test warning");
        Logger::error("Test error");
        Logger::debug("Test debug");
    }
}
