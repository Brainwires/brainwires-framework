# brainwires-memory

Tiered hot/warm/cold agent memory primitives for the Brainwires Agent Framework.

Originally lived inside `brainwires-storage`; lifted into its own crate so
the storage crate can stay focused on generic primitives (the
`StorageBackend` trait, embeddings, BM25, file context, paths) while this
crate owns the memory-tier domain.

## What's here

- **`MessageStore`** — hot tier; full conversation messages with vector
  embeddings and TTL eviction.
- **`SummaryStore`** — warm tier; compressed message summaries with
  semantic search over the `summary` field.
- **`FactStore`** — cold tier; key-fact extracts (decisions, definitions,
  requirements, code changes, configuration) with semantic search over
  the `fact` text.
- **`MentalModelStore`** — synthesised behavioural / structural / causal /
  procedural beliefs the agent built up about the user or task.
- **`TierMetadataStore`** — per-message tier placement, access counts,
  importance scores, authority. Drives promotion / demotion decisions.
- **`TieredMemory`** — orchestrator that owns all four stores and runs
  the multi-factor scoring (similarity × recency × importance) for
  search, plus the demotion / promotion heuristics that move messages
  between tiers as they age.

## Backend selection

Every store is generic over `brainwires_storage::StorageBackend`. The
default backend is `LanceDatabase` (LanceDB), but any other
`StorageBackend` implementation in `brainwires-storage::databases::*`
(Postgres, Surreal, MySQL, …) plugs in unchanged.

## Embeddings

`MessageStore`, `SummaryStore`, `FactStore`, `MentalModelStore` take an
`Arc<CachedEmbeddingProvider>` (LRU-cached FastEmbed by default, supplied
by `brainwires-storage::embeddings`). `TierMetadataStore` does not embed —
it tracks placement metadata only.

## Quick start

```rust
use std::sync::Arc;
use brainwires_storage::{CachedEmbeddingProvider, LanceDatabase};
use brainwires_memory::{
    MessageStore, MessageMetadata,
    TieredMemory, TieredMemoryConfig,
};

# async fn example() -> anyhow::Result<()> {
let db = Arc::new(LanceDatabase::new("/tmp/agent.db").await?);
let embeddings = Arc::new(CachedEmbeddingProvider::new()?);

// Hot tier
let messages = MessageStore::new(db.clone(), embeddings.clone());
messages.ensure_table().await?;

// Tiered orchestration
let memory = TieredMemory::new(
    messages,
    db.clone(),
    embeddings.clone(),
    TieredMemoryConfig::default(),
);
memory.ensure_tables().await?;
# Ok(()) }
```

## Related crates

- **`brainwires-storage`** — the trait + backends + embeddings this crate
  builds on.
- **`brainwires-cli` `crate::storage`** — CLI-domain stores
  (`Conversation`, `Plan`, `Template`, `Lock`, `Task`, `Image`).
- **`brainwires-memory-server`** — separate `extras` crate; mem0-API HTTP
  service backed by `brainwires-knowledge`'s LanceDB ThoughtStore. Not a
  consumer of this crate — different layer (external REST surface vs
  internal tiered primitives).
