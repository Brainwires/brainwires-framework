//! Unified telemetry for the Brainwires Agent Framework.
//!
//! Covers both observability (analytics events, tracing layer, SQLite
//! persistence) and billing (usage events, billing hook trait).
//!
//! # Analytics
//!
//! 1. **Explicit emission** — call [`AnalyticsCollector::record`] directly.
//! 2. **`tracing` layer** — register [`AnalyticsLayer`] to intercept known
//!    span names (`provider.chat`, etc.) automatically.
//! 3. **[`AnalyticsQuery`]** (feature `sqlite`) — query aggregated data: cost
//!    by model, tool frequency, daily summaries.
//!
//! # Billing hooks
//!
//! Implement [`BillingHook`] and pass it into `TaskAgentConfig::billing_hook`
//! to receive a [`UsageEvent`] at every provider call and tool call.
//! Full implementations (ledger, wallet, Stripe) live in
//! `extras/brainwires-billing`.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use brainwires_telemetry::{AnalyticsCollector, SqliteAnalyticsSink, AnalyticsLayer};
//! use tracing_subscriber::prelude::*;
//!
//! let sink      = SqliteAnalyticsSink::new_default()?;
//! let collector = AnalyticsCollector::new(vec![Box::new(sink)]);
//!
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .with(AnalyticsLayer::new(collector.clone()))
//!     .init();
//! ```

pub mod collector;
pub mod error;
pub mod events;
pub mod export;
pub mod layer;
pub mod pii;
pub mod schema;
pub mod sink;
pub mod sinks;

// Billing hook surface
pub mod billing_hook;
pub mod usage;

#[cfg(feature = "sqlite")]
pub mod query;

pub use collector::AnalyticsCollector;
pub use error::{AnalyticsError, AnalyticsResult};
pub use events::AnalyticsEvent;
pub use layer::AnalyticsLayer;
pub use sink::{AnalyticsSink, BoxedSink};
pub use sinks::memory::{DEFAULT_CAPACITY, MemoryAnalyticsSink};

pub use billing_hook::{BillingError, BillingHook};
pub use usage::UsageEvent;

#[cfg(feature = "sqlite")]
pub use sinks::sqlite::SqliteAnalyticsSink;

#[cfg(feature = "sqlite")]
pub use query::{AnalyticsQuery, CostByModelRow, DailySummaryRow, ToolFrequencyRow};
