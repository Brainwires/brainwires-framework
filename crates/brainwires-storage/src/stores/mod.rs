//! Domain stores for conversation, message, task, plan, and other data persistence.
//!
//! ## Architecture
//!
//! Stores are designed to be backend-agnostic via the [`StorageBackend`](backend::StorageBackend)
//! trait. The default backend is [`LanceBackend`](backends::LanceBackend) (embedded LanceDB).
//!
//! The legacy [`LanceClient`](lance_client::LanceClient) wrapper is still available
//! for code that hasn't migrated to the trait yet.

// ── Storage backend abstraction ─────────────────────────────────────────

/// Backend-agnostic storage trait and generic types.
pub mod backend;

/// Concrete backend implementations.
pub mod backends;

// ── LanceDB connection wrapper (legacy) ─────────────────────────────────

/// LanceDB connection and table management.
#[cfg(feature = "native")]
pub mod lance_client;

// ── Domain stores ───────────────────────────────────────────────────────

/// Conversation metadata storage.
#[cfg(feature = "native")]
pub mod conversation_store;
/// Cold-tier key fact storage with vector search.
#[cfg(feature = "native")]
pub mod fact_store;
/// Image analysis storage with semantic search.
#[cfg(feature = "native")]
pub mod image_store;
/// Cross-process lock coordination (SQLite-backed).
#[cfg(feature = "native")]
pub mod lock_store;
/// Message storage with vector search (hot tier).
#[cfg(feature = "native")]
pub mod message_store;
/// Execution plan storage with markdown export.
#[cfg(feature = "native")]
pub mod plan_store;
/// Warm-tier compressed summary storage with vector search.
#[cfg(feature = "native")]
pub mod summary_store;
/// Task and agent state persistence.
#[cfg(feature = "native")]
pub mod task_store;
/// Tier assignment metadata tracking.
#[cfg(feature = "native")]
pub mod tier_metadata_store;

// ── Always available ────────────────────────────────────────────────────

/// Reusable plan template storage (pure logic, no DB dependency).
pub mod template_store;
