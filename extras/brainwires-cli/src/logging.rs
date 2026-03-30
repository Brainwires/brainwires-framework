// File logging with daily rotation to ~/.brainwires/logs/
// Logs to file: ~/.brainwires/logs/brainwires.log.YYYY-MM-DD
//! Comprehensive logging system for brainwires-cli
//!
//! Logs to both stdout and rotating files in ~/.brainwires/logs/

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging system with file and stdout output
pub fn init_logging() -> Result<()> {
    // Create logs directory
    let log_dir = get_log_directory()?;
    std::fs::create_dir_all(&log_dir)?;

    // Create file appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "brainwires.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Create stdout appender
    let (non_blocking_stdout, _guard) = tracing_appender::non_blocking(std::io::stdout());

    // Build the subscriber with both file and stdout layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("brainwires_cli=debug,info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false)  // No ANSI codes in files
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true),
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking_stdout)
                .with_target(false)  // Cleaner stdout output
                .with_line_number(false),
        )
        .init();

    // Leak the guards so they persist for the program lifetime
    std::mem::forget(_guard);

    tracing::info!("Logging initialized to {}", log_dir.display());
    Ok(())
}




/// Get the log directory path: ~/.brainwires/logs/
fn get_log_directory() -> Result<PathBuf> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    Ok(PathBuf::from(home).join(".brainwires").join("logs"))
}

/// Get path to current log file
pub fn get_current_log_file() -> Result<PathBuf> {
    let log_dir = get_log_directory()?;
    let date = chrono::Local::now().format("%Y-%m-%d");
    Ok(log_dir.join(format!("brainwires.log.{}", date)))
}
