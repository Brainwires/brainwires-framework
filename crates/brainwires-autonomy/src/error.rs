//! Error types for the autonomy crate.

use thiserror::Error;

/// Top-level error type for autonomous operations.
#[derive(Error, Debug)]
pub enum AutonomyError {
    /// Safety constraint triggered a stop.
    #[error("Safety stop: {0}")]
    SafetyStop(String),

    /// Budget limit exceeded.
    #[error("Budget exceeded: ${0:.2}")]
    BudgetExceeded(f64),

    /// Circuit breaker tripped after consecutive failures.
    #[error("Circuit breaker tripped after {0} consecutive failures")]
    CircuitBreakerTripped(u32),

    /// Diff size limit exceeded.
    #[error("Diff limit exceeded: {0} lines")]
    DiffLimitExceeded(u32),

    /// Maximum cycle count reached.
    #[error("Cycle limit reached: {0}")]
    CycleLimitReached(u32),

    /// Git operation error.
    #[error("Git error: {0}")]
    GitError(String),

    /// Forge (GitHub/GitLab) operation error.
    #[error("Forge error: {0}")]
    ForgeError(String),

    /// Webhook delivery or parsing error.
    #[error("Webhook error: {0}")]
    WebhookError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Agent execution error.
    #[error("Agent error: {0}")]
    AgentError(String),

    /// Investigation/analysis error.
    #[error("Investigation error: {0}")]
    InvestigationError(String),

    /// Merge policy violation.
    #[error("Merge policy error: {0}")]
    MergePolicyError(String),

    /// Other unclassified error.
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Convenience result alias.
pub type AutonomyResult<T> = Result<T, AutonomyError>;
