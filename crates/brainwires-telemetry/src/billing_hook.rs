use async_trait::async_trait;
use thiserror::Error;

/// Error type returned by [`BillingHook`] implementations.
#[derive(Debug, Error)]
pub enum BillingError {
    /// A hook implementation failed to record the event.
    #[error("billing hook error: {0}")]
    Hook(String),

    /// JSON serialization / deserialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Receives billable usage events emitted by the agent run loop.
///
/// Implement this trait to handle events however your application needs —
/// persist to a database, aggregate into a wallet, forward to Stripe, etc.
/// Pass an `Arc<dyn BillingHook>` (via `BillingHookRef`) into
/// `TaskAgentConfig::billing_hook`.
///
/// The method is fail-open: errors are logged but never abort the agent run.
#[async_trait]
pub trait BillingHook: Send + Sync + 'static {
    async fn on_usage(&self, event: &crate::UsageEvent) -> Result<(), BillingError>;
}
