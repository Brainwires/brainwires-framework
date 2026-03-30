//! Unified Application Error Types
//!
//! Provides a consolidated error type hierarchy for the brainwires-cli application.
//! This module unifies error handling across different subsystems while preserving
//! domain-specific error context.
//!
//! # Usage
//!
//! ```ignore
//! use crate::error::{AppError, AppResult};
//!
//! fn my_function() -> AppResult<()> {
//!     // Errors from subsystems automatically convert to AppError
//!     let result = some_mdap_operation()?;
//!     Ok(())
//! }
//! ```
//!
//! # Error Categories
//!
//! - **Agent**: Multi-agent system errors (communication, coordination, task execution)
//! - **Mdap**: MDAP framework errors (voting, decomposition, microagents)
//! - **Tool**: Tool execution errors (with retry classification)
//! - **Storage**: Database and persistence errors
//! - **Auth**: Authentication and authorization errors
//! - **Config**: Configuration and settings errors
//! - **Ipc**: Inter-process communication errors
//! - **Provider**: AI provider errors (API calls, streaming)
//! - **Network**: Network and HTTP errors
//! - **Io**: File system and I/O errors

use std::fmt;
use thiserror::Error;

use crate::mdap::error::MdapError;
use crate::tools::error::ToolErrorCategory;

/// Unified application error type
#[derive(Error, Debug)]
pub enum AppError {
    // ==========================================================================
    // Agent System Errors
    // ==========================================================================
    /// Agent communication error
    #[error("Agent communication error: {0}")]
    AgentCommunication(String),

    /// Agent coordination error (locks, resources)
    #[error("Agent coordination error: {0}")]
    AgentCoordination(String),

    /// Agent task execution error
    #[error("Agent task error: {0}")]
    AgentTask(String),

    /// Agent timeout
    #[error("Agent timeout: {context} after {timeout_secs}s")]
    AgentTimeout { context: String, timeout_secs: u64 },

    // ==========================================================================
    // MDAP Framework Errors
    // ==========================================================================
    /// MDAP framework error
    #[error("MDAP error: {0}")]
    Mdap(#[from] MdapError),

    // ==========================================================================
    // Tool Errors
    // ==========================================================================
    /// Tool execution error with classification
    #[error("Tool '{tool}' failed: {message}")]
    Tool {
        tool: String,
        message: String,
        #[source]
        category: Option<ToolErrorCategoryWrapper>,
    },

    /// Tool permission denied
    #[error("Tool '{tool}' permission denied: {reason}")]
    ToolPermission { tool: String, reason: String },

    /// Tool validation error
    #[error("Tool '{tool}' validation error: {message}")]
    ToolValidation { tool: String, message: String },

    // ==========================================================================
    // Storage Errors
    // ==========================================================================
    /// Database connection error
    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    /// Database query error
    #[error("Database query error: {0}")]
    DatabaseQuery(String),

    /// Data serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Data deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Storage not found
    #[error("Not found: {entity} with id '{id}'")]
    NotFound { entity: String, id: String },

    // ==========================================================================
    // Authentication Errors
    // ==========================================================================
    /// Authentication required
    #[error("Authentication required: {0}")]
    AuthRequired(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Invalid token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    // ==========================================================================
    // Configuration Errors
    // ==========================================================================
    /// Configuration file error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid configuration value
    #[error("Invalid config value for '{key}': {reason}")]
    ConfigValue { key: String, reason: String },

    /// Missing required configuration
    #[error("Missing required config: {0}")]
    ConfigMissing(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvVar(String),

    // ==========================================================================
    // IPC Errors
    // ==========================================================================
    /// IPC connection error
    #[error("IPC connection error: {0}")]
    IpcConnection(String),

    /// IPC protocol error
    #[error("IPC protocol error: {0}")]
    IpcProtocol(String),

    /// IPC message error
    #[error("IPC message error: {0}")]
    IpcMessage(String),

    // ==========================================================================
    // Provider Errors
    // ==========================================================================
    /// Provider API error
    #[error("Provider API error: {provider} - {message}")]
    ProviderApi { provider: String, message: String },

    /// Provider rate limit
    #[error("Provider rate limit: {provider} - retry after {retry_after_secs}s")]
    ProviderRateLimit {
        provider: String,
        retry_after_secs: u64,
    },

    /// Provider streaming error
    #[error("Provider streaming error: {0}")]
    ProviderStream(String),

    /// Provider not available
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),

    // ==========================================================================
    // Network Errors
    // ==========================================================================
    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    // ==========================================================================
    // I/O Errors
    // ==========================================================================
    /// File I/O error
    #[error("File I/O error: {0}")]
    FileIo(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Standard I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // ==========================================================================
    // General Errors
    // ==========================================================================
    /// Internal error (should not happen in normal operation)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Cancelled by user
    #[error("Operation cancelled")]
    Cancelled,

    /// Generic error with anyhow context
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Wrapper for ToolErrorCategory to implement Error trait
#[derive(Debug)]
pub struct ToolErrorCategoryWrapper(pub ToolErrorCategory);

impl std::error::Error for ToolErrorCategoryWrapper {}

impl fmt::Display for ToolErrorCategoryWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.0.category_name(), self.0.error_message())
    }
}

/// Result type alias using AppError
pub type AppResult<T> = Result<T, AppError>;

// ==========================================================================
// Conversions from standard library types
// ==========================================================================

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            AppError::Deserialization(err.to_string())
        } else {
            AppError::Serialization(err.to_string())
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AppError::Timeout(err.to_string())
        } else if err.is_connect() {
            AppError::Connection(err.to_string())
        } else if err.is_status() {
            AppError::Http(err.to_string())
        } else {
            AppError::Http(err.to_string())
        }
    }
}

// ==========================================================================
// Helper methods
// ==========================================================================

impl AppError {
    /// Create a tool error with classification
    pub fn tool(tool: impl Into<String>, error: impl Into<String>) -> Self {
        let tool = tool.into();
        let message = error.into();
        let category = crate::tools::error::classify_error(&tool, &message);
        AppError::Tool {
            tool,
            message,
            category: Some(ToolErrorCategoryWrapper(category)),
        }
    }

    /// Create a not found error
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        AppError::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a config value error
    pub fn config_value(key: impl Into<String>, reason: impl Into<String>) -> Self {
        AppError::ConfigValue {
            key: key.into(),
            reason: reason.into(),
        }
    }

    /// Create a provider API error
    pub fn provider_api(provider: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::ProviderApi {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Tool { category: Some(wrapper), .. } => wrapper.0.is_retryable(),
            AppError::Timeout(_) => true,
            AppError::Connection(_) => true,
            AppError::ProviderRateLimit { .. } => true,
            AppError::ProviderStream(_) => true,
            AppError::AgentTimeout { .. } => true,
            _ => false,
        }
    }

    /// Check if error is authentication related
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            AppError::AuthRequired(_)
                | AppError::AuthFailed(_)
                | AppError::TokenExpired
                | AppError::InvalidToken(_)
        )
    }

    /// Check if error is configuration related
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            AppError::Config(_) | AppError::ConfigValue { .. } | AppError::ConfigMissing(_)
        )
    }

    /// Get suggested retry delay in seconds, if applicable
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            AppError::ProviderRateLimit { retry_after_secs, .. } => Some(*retry_after_secs),
            AppError::Tool { category: Some(wrapper), .. } => {
                if let ToolErrorCategory::ExternalService { retry_after: Some(d), .. } = &wrapper.0 {
                    Some(d.as_secs())
                } else {
                    None
                }
            }
            AppError::Timeout(_) | AppError::Connection(_) => Some(2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_error_creation() {
        let err = AppError::tool("bash", "Connection refused");
        assert!(matches!(err, AppError::Tool { .. }));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_not_found_error() {
        let err = AppError::not_found("conversation", "abc123");
        assert!(matches!(err, AppError::NotFound { .. }));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_auth_error_detection() {
        assert!(AppError::AuthRequired("test".into()).is_auth_error());
        assert!(AppError::TokenExpired.is_auth_error());
        assert!(!AppError::Config("test".into()).is_auth_error());
    }

    #[test]
    fn test_config_error_detection() {
        assert!(AppError::Config("test".into()).is_config_error());
        assert!(AppError::ConfigMissing("key".into()).is_config_error());
        assert!(!AppError::AuthRequired("test".into()).is_config_error());
    }

    #[test]
    fn test_retry_after() {
        let err = AppError::ProviderRateLimit {
            provider: "openai".into(),
            retry_after_secs: 30,
        };
        assert_eq!(err.retry_after_secs(), Some(30));

        let err = AppError::Timeout("test".into());
        assert_eq!(err.retry_after_secs(), Some(2));

        let err = AppError::FileNotFound("test".into());
        assert_eq!(err.retry_after_secs(), None);
    }

    #[test]
    fn test_mdap_error_conversion() {
        use crate::mdap::error::VotingError;

        let voting_err = VotingError::Cancelled;
        let mdap_err = MdapError::Voting(voting_err);
        let app_err: AppError = mdap_err.into();

        assert!(matches!(app_err, AppError::Mdap(_)));
    }
}
