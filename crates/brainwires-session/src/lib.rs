#![deny(missing_docs)]
//! Pluggable session-persistence for the Brainwires Agent Framework.
//!
//! See crate README.md for an overview. The [`SessionStore`] trait is the
//! single extension point; [`InMemorySessionStore`] is the default impl used
//! by tests and ephemeral sessions, and [`SqliteSessionStore`] (behind the
//! `sqlite` feature) provides disk-backed persistence.

mod error;
mod memory;
#[cfg(feature = "sqlite")]
mod sqlite;
mod types;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

pub use brainwires_core::Message;
pub use error::SessionError;
pub use memory::InMemorySessionStore;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteSessionStore;
pub use types::{SessionId, SessionRecord};

/// Trait implemented by every session-persistence backend.
///
/// Implementations must be cheap to share via `Arc` and safe to call
/// concurrently from any async context.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load a session's full transcript. Returns `Ok(None)` when the id is
    /// not known — callers should treat this as "fresh session".
    async fn load(&self, id: &SessionId) -> Result<Option<Vec<Message>>>;

    /// Overwrite a session's full transcript. Creating the session if
    /// it didn't already exist.
    ///
    /// Implementations should treat the provided slice as the authoritative
    /// state and persist it atomically — a crash mid-write must leave the
    /// store with either the old or new transcript, never a partial one.
    async fn save(&self, id: &SessionId, messages: &[Message]) -> Result<()>;

    /// Enumerate every session the store knows about, newest-last. Returns
    /// metadata only — use [`Self::load`] to read message content.
    async fn list(&self) -> Result<Vec<SessionRecord>>;

    /// Remove a session. Deleting an unknown id is a no-op (not an error).
    async fn delete(&self, id: &SessionId) -> Result<()>;
}

/// Convenience alias used by downstream crates that hold the store behind
/// an `Arc`.
pub type ArcSessionStore = Arc<dyn SessionStore>;
