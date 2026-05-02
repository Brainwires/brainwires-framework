//! # brainwires-memory
//!
//! Tiered hot/warm/cold agent memory primitives. Originally lived inside
//! `brainwires-storage`, lifted out so a single-purpose crate carries the
//! memory orchestration without dragging the storage crate into domain
//! schemas.
//!
//! - **Hot tier** — full messages with embeddings ([`MessageStore`]).
//! - **Warm tier** — compressed summaries ([`SummaryStore`]).
//! - **Cold tier** — key-fact extracts ([`FactStore`]).
//! - **Mental models** — synthesised behavioural / structural / causal /
//!   procedural beliefs ([`MentalModelStore`]).
//! - **Tier metadata** — placement, access counts, importance scores
//!   ([`TierMetadataStore`]).
//! - **Orchestration** — promotion / demotion across the tiers
//!   ([`TieredMemory`]).
//!
//! All stores are generic over `brainwires_storage::StorageBackend` so
//! the same code runs against any backend the storage crate exposes.

pub mod fact_store;
pub mod mental_model_store;
pub mod message_store;
pub mod summary_store;
pub mod tier_metadata_store;
pub mod tiered_memory;

pub use fact_store::{FactStore, facts_field_defs};
pub use mental_model_store::{MentalModel, MentalModelStore, ModelType};
pub use message_store::{MessageMetadata, MessageStore};
pub use summary_store::{SummaryStore, summaries_field_defs};
pub use tier_metadata_store::TierMetadataStore;

#[cfg(feature = "native")]
pub use fact_store::facts_schema;
#[cfg(feature = "native")]
pub use message_store::messages_schema;
#[cfg(feature = "native")]
pub use summary_store::summaries_schema;
pub use tiered_memory::{
    CanonicalWriteToken, FactType, KeyFact, MemoryAuthority, MemoryTier, MessageSummary,
    MultiFactorScore, TierMetadata, TieredMemory, TieredMemoryConfig, TieredMemoryStats,
    TieredSearchResult,
};
