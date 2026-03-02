//! Error types for the autonomy crate.

use thiserror::Error;

/// Top-level error type for autonomous operations.
#[derive(Error, Debug)]
pub enum AutonomyError {
    #[error("Safety stop: {0}")]
    SafetyStop(String),

    #[error("Budget exceeded: ${0:.2}")]
    BudgetExceeded(f64),

    #[error("Circuit breaker tripped after {0} consecutive failures")]
    CircuitBreakerTripped(u32),

    #[error("Diff limit exceeded: {0} lines")]
    DiffLimitExceeded(u32),

    #[error("Cycle limit reached: {0}")]
    CycleLimitReached(u32),

    #[error("Git error: {0}")]
    GitError(String),

    #[error("Forge error: {0}")]
    ForgeError(String),

    #[error("Webhook error: {0}")]
    WebhookError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Investigation error: {0}")]
    InvestigationError(String),

    #[error("Merge policy error: {0}")]
    MergePolicyError(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Convenience result alias.
pub type AutonomyResult<T> = Result<T, AutonomyError>;
