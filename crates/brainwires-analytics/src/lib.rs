//! Unified analytics collection, persistence, and querying for the
//! Brainwires Agent Framework.
//!
//! # Overview
//!
//! This crate provides three complementary ways to collect analytics:
//!
//! 1. **Explicit emission** — call [`AnalyticsCollector::record`] directly from any
//!    instrumented site (providers, agents, gateways). Provides full data fidelity
//!    (tokens, cost, latency) because the emitter has access to the return values.
//!
//! 2. **`tracing` layer** — register [`AnalyticsLayer`] alongside your existing
//!    `tracing-subscriber` setup. Automatically intercepts known span names
//!    (`provider.chat`, etc.) without modifying the instrumented code.
//!
//! 3. **[`AnalyticsQuery`]** (feature `sqlite`) — read aggregated analytics from
//!    the local SQLite database: cost by model, tool frequency, daily summaries.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use brainwires_analytics::{AnalyticsCollector, SqliteAnalyticsSink, AnalyticsLayer};
//! use tracing_subscriber::prelude::*;
//!
//! // 1. Create a sink and collector
//! let sink      = SqliteAnalyticsSink::new_default()?;
//! let collector = AnalyticsCollector::new(vec![Box::new(sink)]);
//!
//! // 2. Register the tracing layer
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .with(AnalyticsLayer::new(collector.clone()))
//!     .init();
//!
//! // 3. Emit events directly where you have full data
//! collector.record(AnalyticsEvent::Custom {
//!     session_id: None,
//!     name: "my_event".into(),
//!     payload: serde_json::json!({"key": "value"}),
//!     timestamp: chrono::Utc::now(),
//! });
//!
//! // 4. Query at any time
//! let query = AnalyticsQuery::new_default()?;
//! query.rebuild_summaries()?;
//! let costs = query.cost_by_model(None, None)?;
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

#[cfg(feature = "sqlite")]
pub mod query;

pub use collector::AnalyticsCollector;
pub use error::{AnalyticsError, AnalyticsResult};
pub use events::AnalyticsEvent;
pub use layer::AnalyticsLayer;
pub use sink::{AnalyticsSink, BoxedSink};
pub use sinks::memory::{DEFAULT_CAPACITY, MemoryAnalyticsSink};

#[cfg(feature = "sqlite")]
pub use sinks::sqlite::SqliteAnalyticsSink;

#[cfg(feature = "sqlite")]
pub use query::{AnalyticsQuery, CostByModelRow, DailySummaryRow, ToolFrequencyRow};
