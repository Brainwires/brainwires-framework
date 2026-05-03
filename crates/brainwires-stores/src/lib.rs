//! # brainwires-stores
//!
//! Opinionated minimum data-store set for the Brainwires Agent Framework.
//!
//! Every store is built on the [`brainwires_storage::StorageBackend`] trait —
//! consumers can swap backends without touching store code. Each store
//! family is gated behind a Cargo feature so consumers only pay for what
//! they use.
//!
//! ## Feature flags
//!
//! - `session` *(default)* — `SessionStore` trait + `InMemorySessionStore`
//!   (and `SqliteSessionStore` with the `sqlite` feature). Full-transcript
//!   persistence keyed by session id.
//! - `task` *(default)* — `TaskStore`, `AgentStateStore`, `PersistentTaskManager`.
//! - `plan` *(default)* — `PlanStore` + `TemplateStore`.
//! - `conversation` *(default)* — `ConversationStore` (catalog metadata —
//!   id, title, model, message count; the actual messages live in
//!   `MessageStore` under the `memory` feature).
//! - `memory` — `MessageStore` / `SummaryStore` / `FactStore` /
//!   `MentalModelStore` / `TierMetadataStore`. Hot/warm/cold tier primitives.
//! - `tiered` (implies `memory`) — `TieredMemory` orchestration.
//! - `dream` (implies `tiered`) — offline consolidation engine.
//! - `lock` — `LockStore`. Coordination locks (rusqlite-backed).
//! - `image` — `ImageStore` with hashing + metadata.
//! - `sqlite` — pulls rusqlite for backends that need it.

#[cfg(feature = "session")]
pub mod session;

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "session")]
pub use session::{
    ArcSessionStore, InMemorySessionStore, ListOptions, Message, SessionBroker, SessionError,
    SessionId, SessionMessage, SessionRecord, SessionStore, SessionSummary, SpawnRequest,
    SpawnedSession,
};

#[cfg(all(feature = "session", feature = "sqlite"))]
pub use session::SqliteSessionStore;

#[cfg(feature = "memory")]
pub use memory::{
    CanonicalWriteToken, FactStore, FactType, KeyFact, MemoryAuthority, MemoryTier, MentalModel,
    MentalModelStore, MessageMetadata, MessageStore, MessageSummary, ModelType, MultiFactorScore,
    SummaryStore, TierMetadata, TierMetadataStore, TieredMemory, TieredMemoryConfig,
    TieredMemoryStats, TieredSearchResult, facts_field_defs, summaries_field_defs,
};

#[cfg(feature = "memory")]
pub use memory::{facts_schema, messages_schema, summaries_schema};
