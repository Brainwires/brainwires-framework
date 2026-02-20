//! Framework error types

use thiserror::Error;

/// Core framework errors
#[derive(Error, Debug)]
pub enum FrameworkError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias using FrameworkError
pub type FrameworkResult<T> = Result<T, FrameworkError>;
