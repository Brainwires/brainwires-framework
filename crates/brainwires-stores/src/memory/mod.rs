//! Tiered hot/warm/cold agent memory primitives.
//!
//! - **Hot tier** — full messages with embeddings ([`MessageStore`]).
//! - **Warm tier** — compressed summaries ([`SummaryStore`]).
//! - **Cold tier** — key-fact extracts ([`FactStore`]).
//! - **Mental models** — synthesised behavioural / structural / causal /
//!   procedural beliefs ([`MentalModelStore`]).
//! - **Tier metadata** — placement, access counts, importance scores
//!   ([`TierMetadataStore`]).
//! - **Orchestration** — promotion / demotion across the tiers
//!   ([`TieredMemory`]) under the `tiered` feature.
//! - **Consolidation** — offline summarisation / fact extraction / tier demotion
//!   ([`dream`]) under the `dream` feature.
//!
//! All stores are generic over `brainwires_storage::StorageBackend` so the
//! same code runs against any backend the storage crate exposes.

#[cfg(feature = "dream")]
pub mod dream;

pub mod fact_store;
pub mod mental_model_store;
pub mod message_store;
pub mod summary_store;
pub mod tier_metadata_store;
pub mod tiered_memory;

pub use fact_store::{FactStore, facts_field_defs, facts_schema};
pub use mental_model_store::{MentalModel, MentalModelStore, ModelType};
pub use message_store::{MessageMetadata, MessageStore, messages_schema};
pub use summary_store::{SummaryStore, summaries_field_defs, summaries_schema};
pub use tier_metadata_store::TierMetadataStore;
pub use tiered_memory::{
    CanonicalWriteToken, FactType, KeyFact, MemoryAuthority, MemoryTier, MessageSummary,
    MultiFactorScore, TierMetadata, TieredMemory, TieredMemoryConfig, TieredMemoryStats,
    TieredSearchResult,
};
