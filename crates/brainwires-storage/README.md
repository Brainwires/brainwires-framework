# brainwires-storage

[![Crates.io](https://img.shields.io/crates/v/brainwires-storage.svg)](https://crates.io/crates/brainwires-storage)
[![Documentation](https://img.shields.io/docsrs/brainwires-storage)](https://docs.rs/brainwires-storage)
[![License](https://img.shields.io/crates/l/brainwires-storage.svg)](LICENSE)

LanceDB-backed storage, tiered memory, and document management for the Brainwires Agent Framework.

## Overview

`brainwires-storage` is the persistent backend for the Brainwires Agent Framework's infinite context memory system. The crate provides conversation storage with semantic search, document ingestion with hybrid retrieval, three-tier memory hierarchy, image analysis storage, entity extraction with contradiction detection, cross-process lock coordination, and reusable plan templates — enabling agents to maintain unbounded context, coordinate safely, and retrieve relevant knowledge across sessions.

**Design principles:**

- **Semantic-first retrieval** — all stores embed content via all-MiniLM-L6-v2 (384 dimensions) and search by vector similarity, so queries match meaning rather than keywords
- **Hybrid search** — document retrieval combines vector similarity with BM25 keyword scoring via Reciprocal Rank Fusion (RRF) for best-of-both-worlds accuracy
- **Three-tier memory** — hot (full messages with TTL), warm (compressed summaries), cold (extracted facts) with automatic demotion/promotion based on importance and access patterns
- **Memory safety** — contradiction detection flags conflicting facts for human review; canonical write tokens gate long-lived writes; session TTL auto-expires ephemeral data
- **Cross-process coordination** — SQLite-backed locks with WAL mode, stale lock detection via PID/hostname, and automatic cleanup for multi-instance deployments
- **Feature-gated portability** — pure types and logic compile everywhere; native-only modules (LanceDB, Arrow, SQLite) are behind the `native` feature for WASM compatibility

```text
  ┌───────────────────────────────────────────────────────────────────────┐
  │                        brainwires-storage                            │
  │                                                                      │
  │  ┌─── Core Infrastructure ─────────────────────────────────────────┐ │
  │  │  LanceClient ──► LanceDB connection & table management          │ │
  │  │  EmbeddingProvider ──► all-MiniLM-L6-v2 with LRU cache (1000)  │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │                                                                      │
  │  ┌─── Message & Conversation Storage ──────────────────────────────┐ │
  │  │  MessageStore ──► vector search, TTL expiry, batch ops          │ │
  │  │  ConversationStore ──► metadata, listing by recency             │ │
  │  │  TaskStore / AgentStateStore ──► task & agent persistence       │ │
  │  │  PlanStore ──► execution plans with markdown export             │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │                                                                      │
  │  ┌─── Tiered Memory System ────────────────────────────────────────┐ │
  │  │  Hot ──► full messages (MessageStore, session TTL)              │ │
  │  │  Warm ──► compressed summaries (SummaryStore)                   │ │
  │  │  Cold ──► extracted facts (FactStore)                           │ │
  │  │  TierMetadataStore ──► access tracking, importance scoring      │ │
  │  │  TieredMemory ──► adaptive search, demotion/promotion           │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │                                                                      │
  │  ┌─── Document Management ─────────────────────────────────────────┐ │
  │  │  DocumentProcessor ──► PDF, DOCX, Markdown, plain text          │ │
  │  │  DocumentChunker ──► paragraph/sentence-aware segmentation      │ │
  │  │  DocumentStore ──► hybrid search (vector + BM25 via RRF)       │ │
  │  │  DocumentMetadataStore ──► hash-based deduplication             │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │                                                                      │
  │  ┌─── Images ────────────────────────────────────────────────────────┐ │
  │  │  ImageStore ──► analyzed images with semantic search             │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │  Note: EntityStore and RelationshipGraph moved to brainwires-brain  │
  │                                                                      │
  │  ┌─── Coordination & Templates ────────────────────────────────────┐ │
  │  │  LockStore ──► SQLite WAL locks, stale detection, cleanup       │ │
  │  │  TemplateStore ──► reusable plans with {{variable}} substitution│ │
  │  │  PersistentTaskManager ──► auto-persist task state for agents   │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  └───────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-storage = "0.1"
```

Store and search conversation messages:

```rust
use brainwires_storage::{LanceClient, EmbeddingProvider, MessageStore, MessageMetadata};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize storage
    let client = Arc::new(LanceClient::new("~/.brainwires/db").await?);
    let embeddings = Arc::new(EmbeddingProvider::new()?);
    client.initialize(embeddings.dimension()).await?;

    let store = MessageStore::new(client.clone(), embeddings.clone());

    // Store a message
    store.add(MessageMetadata {
        message_id: "msg-001".into(),
        conversation_id: "conv-001".into(),
        role: "assistant".into(),
        content: "The auth module uses JWT tokens with RS256 signing".into(),
        token_count: Some(42),
        model_id: Some("claude-opus-4-6".into()),
        images: None,
        created_at: chrono::Utc::now().timestamp(),
        expires_at: None,
    }).await?;

    // Semantic search across all conversations
    let results = store.search("how does authentication work?", 5, 0.7).await?;
    for (msg, score) in &results {
        println!("[{:.2}] {}: {}", score, msg.role, msg.content);
    }

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Enables LanceDB, Arrow, SQLite, FastEmbed, file processing, and all native-only stores |
| `agents` | Yes (via `native`) | Enables `PersistentTaskManager` integration with `brainwires-agents` |
| `wasm` | No | Enables WASM-compatible compilation via `brainwires-core/wasm` |

```toml
# Default (full native functionality)
brainwires-storage = "0.1"

# WASM-compatible (pure types and logic only)
brainwires-storage = { version = "0.1", default-features = false, features = ["wasm"] }

# Native without agent integration
brainwires-storage = { version = "0.1", default-features = false, features = ["native"] }
```

**Module availability by feature:**

| Module | Always | `native` | `agents` |
|--------|--------|----------|----------|
| `document_types`, `document_chunker` | Yes | — | — |
| `image_types` | Yes | — | — |
| `template_store` | Yes | — | — |
| `lance_client`, `embeddings` | — | Yes | — |
| `message_store`, `conversation_store` | — | Yes | — |
| `task_store`, `plan_store`, `lock_store` | — | Yes | — |
| `document_store`, `document_processor` | — | Yes | — |
| `document_metadata_store`, `document_bm25` | — | Yes | — |
| `image_store` | — | Yes | — |
| `tiered_memory`, `summary_store`, `fact_store` | — | Yes | — |
| `tier_metadata_store`, `file_context` | — | Yes | — |
| `persistent_task_manager` | — | — | Yes |

## Architecture

### LanceClient

LanceDB connection manager that initializes and provides access to all storage tables.

| Method | Description |
|--------|-------------|
| `new(db_path)` | Create connection to LanceDB at path |
| `initialize(embedding_dim)` | Initialize all tables with given embedding dimension |
| `connection()` | Get raw LanceDB connection reference |
| `db_path()` | Get database path |

**Table initializers:** `ensure_conversations_table()`, `ensure_messages_table(dim)`, `ensure_tasks_table()`, `ensure_plans_table()`, `ensure_documents_table(dim)`, `ensure_document_metadata_table()`, `ensure_images_table(dim)`, `ensure_summaries_table(dim)`, `ensure_facts_table(dim)`, `ensure_tier_metadata_table()`.

### EmbeddingProvider

Text embedding with LRU caching, backed by FastEmbed (all-MiniLM-L6-v2, 384 dimensions).

| Method | Description |
|--------|-------------|
| `new()` | Create provider with default model |
| `embed(text)` | Embed single text -> `Vec<f32>` |
| `embed_cached(text)` | Embed with LRU cache (1000 entries) -> `Vec<f32>` |
| `embed_batch(texts)` | Embed multiple texts -> `Vec<Vec<f32>>` |
| `dimension()` | Get embedding dimension (384) |
| `cache_len()` | Get current cache size |
| `clear_cache()` | Clear the LRU cache |

### MessageStore

Conversation messages with vector search and TTL expiry support.

| Method | Description |
|--------|-------------|
| `new(client, embeddings)` | Create store |
| `add(message)` | Add a single message |
| `add_batch(messages)` | Add multiple messages |
| `get(message_id)` | Get message by ID |
| `get_by_conversation(conversation_id)` | Get all messages in a conversation |
| `search(query, limit, min_score)` | Semantic search across all messages |
| `search_conversation(conversation_id, query, limit, min_score)` | Search within a conversation |
| `delete(message_id)` | Delete a single message |
| `delete_by_conversation(conversation_id)` | Delete all messages in a conversation |
| `delete_expired()` | Delete TTL-expired messages -> count |

**`MessageMetadata`:**

| Field | Type | Description |
|-------|------|-------------|
| `message_id` | `String` | Unique message identifier |
| `conversation_id` | `String` | Parent conversation |
| `role` | `String` | Message role (user, assistant, system) |
| `content` | `String` | Message content |
| `token_count` | `Option<i32>` | Token count estimate |
| `model_id` | `Option<String>` | Model that generated the message |
| `images` | `Option<String>` | JSON-encoded image references |
| `created_at` | `i64` | Unix timestamp |
| `expires_at` | `Option<i64>` | TTL expiry timestamp (session tier) |

### ConversationStore

Conversation metadata with create-or-update semantics.

| Method | Description |
|--------|-------------|
| `new(client)` | Create store |
| `create(id, title, model_id, message_count)` | Create or update conversation |
| `get(conversation_id)` | Get by ID |
| `list(limit)` | List conversations sorted by recency |
| `update(conversation_id, title, message_count)` | Update metadata |
| `delete(conversation_id)` | Delete conversation |

**`ConversationMetadata`:**

| Field | Type | Description |
|-------|------|-------------|
| `conversation_id` | `String` | Unique identifier |
| `title` | `Option<String>` | Conversation title |
| `model_id` | `Option<String>` | Model used |
| `created_at` | `i64` | Creation timestamp |
| `updated_at` | `i64` | Last update timestamp |
| `message_count` | `i32` | Number of messages |

### TieredMemory

Three-tier memory hierarchy with adaptive search and automatic demotion/promotion.

| Method | Description |
|--------|-------------|
| `new(hot_store, client, embeddings, config)` | Create with custom configuration |
| `with_defaults(hot_store, client, embeddings)` | Create with default thresholds |
| `add_message(message, importance)` | Add to hot tier with Session authority |
| `add_canonical_message(message, importance, token)` | Add canonical message (no TTL) |
| `evict_expired()` | Delete expired session messages -> count |
| `record_access(message_id)` | Update access tracking for scoring |
| `search_adaptive(query, conversation_id)` | Similarity-based search across tiers |
| `search_adaptive_multi_factor(query, conversation_id)` | Blended scoring (similarity + recency + importance) |
| `demote_to_warm(message_id, summary)` | Compress message to summary |
| `demote_to_cold(summary_id, fact)` | Extract fact from summary |
| `promote_to_hot(message_id)` | Restore full message from warm tier |
| `get_demotion_candidates(tier, count)` | Get candidates for demotion |
| `get_stats()` | Get tier counts |
| `fallback_summarize(content)` | No-LLM summary (truncate at 75 words) |
| `fallback_fact(summary)` | No-LLM fact extraction |

**`MemoryTier` enum:**

| Variant | Description |
|---------|-------------|
| `Hot` | Full messages, fastest access, session TTL |
| `Warm` | Compressed summaries, reduced storage |
| `Cold` | Extracted facts, minimal footprint |

**`MemoryAuthority` enum:**

| Variant | Description |
|---------|-------------|
| `Ephemeral` | Temporary, auto-expires quickly |
| `Session` | Default, expires with session TTL |
| `Canonical` | Long-lived, requires `CanonicalWriteToken` |

**`TieredMemoryConfig`:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hot_retention_hours` | `u64` | `24` | Hours before hot tier demotion |
| `warm_retention_hours` | `u64` | `168` | Hours before warm tier demotion |
| `importance_threshold_warm` | `f32` | `0.3` | Minimum importance to stay in hot |
| `importance_threshold_cold` | `f32` | `0.1` | Minimum importance to stay in warm |
| `max_hot_messages` | `usize` | `1000` | Hot tier capacity |
| `max_warm_summaries` | `usize` | `5000` | Warm tier capacity |
| `session_ttl_hours` | `u64` | `48` | Session-authority message TTL |

**`TieredSearchResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `message_id` | `String` | Source message identifier |
| `content` | `String` | Full text or summary or fact |
| `tier` | `MemoryTier` | Which tier the result came from |
| `score` | `f32` | Relevance score |
| `metadata` | `Option<TierMetadata>` | Access and importance tracking |

### DocumentStore

Document ingestion with hybrid search (vector + BM25 via Reciprocal Rank Fusion).

| Method | Description |
|--------|-------------|
| `new(client, embeddings, bm25_base_path)` | Create with default chunking |
| `with_chunker_config(client, embeddings, bm25_base_path, config)` | Create with custom chunking |
| `index_file(file_path, scope)` | Index document from file |
| `index_bytes(bytes, file_name, file_type, scope)` | Index document from bytes |
| `search(request)` | Hybrid or vector-only search |
| `delete_document(document_id)` | Delete document and chunks |
| `list_by_conversation(conversation_id)` | List documents in conversation |
| `list_by_project(project_id)` | List documents in project |
| `get_metadata(document_id)` | Get document metadata |
| `get_document_chunks(document_id)` | Get all chunks for a document |
| `count()` | Total document count |

**`DocumentScope` enum:**

| Variant | Description |
|---------|-------------|
| `Conversation(String)` | Scoped to a conversation |
| `Project(String)` | Scoped to a project |
| `Global` | Available everywhere |

**`DocumentType` enum:** `Pdf`, `Markdown`, `PlainText`, `Docx`, `Unknown`.

### DocumentChunker

Paragraph and sentence-aware document segmentation with configurable overlap.

| Method | Description |
|--------|-------------|
| `new()` | Create with default config (1500 target, 2500 max) |
| `with_config(config)` | Create with custom config |
| `chunk(document_id, content)` | Segment content into chunks |

**`ChunkerConfig`:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_chunk_size` | `usize` | `1500` | Target characters per chunk |
| `max_chunk_size` | `usize` | `2500` | Maximum characters per chunk |
| `min_chunk_size` | `usize` | `100` | Minimum characters per chunk |
| `overlap_size` | `usize` | `200` | Overlap between chunks |
| `respect_headers` | `bool` | `true` | Split at markdown headers |
| `respect_paragraphs` | `bool` | `true` | Split at paragraph boundaries |

**Presets:** `ChunkerConfig::small()` (800 target), `ChunkerConfig::large()` (3000 target).

### EntityStore

Entity tracking with relationship storage and contradiction detection for memory poisoning protection.

| Method | Description |
|--------|-------------|
| `new()` | Create empty store |
| `add_extraction(result, message_id, timestamp)` | Add entities and detect contradictions |
| `pending_contradictions()` | View unresolved contradictions |
| `drain_contradictions()` | Take and clear contradictions |
| `get(name, entity_type)` | Get entity by name and type |
| `get_by_type(entity_type)` | Get all entities of a type |
| `get_top_entities(limit)` | Get most-mentioned entities |
| `get_related(entity_name)` | Get related entity names |
| `get_message_ids(entity_name)` | Get messages mentioning entity |
| `all_entities()` | Iterate all entities |
| `all_relationships()` | Get all relationships |
| `stats()` | Get entity and relationship counts |

**`Entity`:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Entity name |
| `entity_type` | `EntityType` | Classification |
| `message_ids` | `Vec<String>` | Messages mentioning this entity |
| `first_seen` | `i64` | First mention timestamp |
| `last_seen` | `i64` | Most recent mention |
| `mention_count` | `usize` | Total mentions |

**`EntityType` enum:** `File`, `Function`, `Type`, `Error`, `Concept`, `Variable`, `Command`.

**`Relationship` enum:** `Defines`, `References`, `Modifies`, `DependsOn`, `Contains`, `CoOccurs`.

**`ContradictionKind` enum:** `ConflictingDefinition`, `ConflictingModification`.

### RelationshipGraph

In-memory entity relationship graph with traversal and importance scoring.

| Method | Description |
|--------|-------------|
| `new()` | Create empty graph |
| `add_node(name, entity_type)` | Add entity node |
| `add_edge(from, to, edge_type)` | Add relationship edge |
| `get_node(name)` | Get node by name |
| `get_neighbors(name)` | Get adjacent entities |
| `get_edges(name)` | Get edges from a node |
| `shortest_path(from, to)` | Find shortest path between entities |
| `importance_score(name)` | Calculate node importance (degree centrality) |

**`EdgeType` enum:** `Contains`, `References`, `DependsOn`, `Modifies`, `Defines`, `CoOccurs`.

### ImageStore

Analyzed image storage with semantic search over LLM-generated descriptions.

| Method | Description |
|--------|-------------|
| `new(client, embeddings)` | Create store |
| `compute_hash(bytes)` | SHA256 hash for deduplication |
| `store(metadata, storage)` | Store image with metadata |
| `store_from_bytes(bytes, analysis, conversation_id, format)` | Store from raw bytes |
| `get(image_id)` | Get image metadata |
| `get_by_hash(file_hash)` | Deduplicate by content hash |
| `search(request)` | Semantic search on analysis text |
| `list_by_conversation(conversation_id)` | List images in conversation |
| `list_by_message(message_id)` | List images in message |
| `delete(image_id)` | Delete image |
| `delete_by_conversation(conversation_id)` | Delete all in conversation |
| `get_image_data(image_id)` | Retrieve stored image data |
| `count_by_conversation(conversation_id)` | Count images |

**`ImageFormat` enum:** `Png`, `Jpeg`, `Gif`, `Webp`, `Svg`.

**`ImageStorage` enum:** `Base64(String)`, `FilePath(String)`, `Url(String)`.

### LockStore

SQLite-backed cross-process lock coordination with stale lock detection.

| Method | Description |
|--------|-------------|
| `new_default()` | Use `~/.brainwires/locks.db` |
| `new_with_path(db_path)` | Use custom database path |
| `try_acquire(lock_type, resource_path, agent_id, timeout)` | Acquire lock (idempotent per agent) |
| `release(lock_type, resource_path, agent_id)` | Release a lock |
| `release_all_for_agent(agent_id)` | Release all locks held by agent |
| `is_locked(lock_type, resource_path)` | Check lock status |
| `cleanup_stale()` | Remove expired and dead-process locks |
| `list_locks()` | List all active locks |
| `force_release(lock_id)` | Admin: force release any lock |
| `stats()` | Lock statistics |

**Lock types:** `file_read`, `file_write`, `build`, `test`, `build_test`.

**`LockRecord`:**

| Field | Type | Description |
|-------|------|-------------|
| `lock_id` | `String` | Unique lock identifier |
| `lock_type` | `String` | Lock type (file_read, file_write, etc.) |
| `resource_path` | `String` | Resource being locked |
| `agent_id` | `String` | Agent holding the lock |
| `process_id` | `i32` | OS process ID |
| `hostname` | `String` | Machine hostname |
| `acquired_at` | `i64` | Acquisition timestamp |
| `expires_at` | `Option<i64>` | Expiry timestamp |

**`LockStats`:**

| Field | Type | Description |
|-------|------|-------------|
| `total_locks` | `usize` | Total active locks |
| `file_read_locks` | `usize` | Active read locks |
| `file_write_locks` | `usize` | Active write locks |
| `build_locks` | `usize` | Active build locks |
| `test_locks` | `usize` | Active test locks |
| `stale_locks` | `usize` | Detected stale locks |

### TaskStore

Task persistence with bidirectional conversion to `brainwires_core::Task`.

| Method | Description |
|--------|-------------|
| `new(client)` | Create store |
| `save(task, conversation_id)` | Persist a task |
| `get(task_id)` | Get task by ID |
| `get_by_conversation(conversation_id)` | Get all tasks in conversation |
| `get_by_plan(plan_id)` | Get all tasks in plan |
| `delete(task_id)` | Delete a task |
| `delete_by_conversation(conversation_id)` | Delete all in conversation |
| `delete_by_plan(plan_id)` | Delete all in plan |

**`AgentStateStore`** — tracks background agent execution state with the same CRUD pattern: `save(state)`, `get(agent_id)`, `get_by_conversation(id)`, `get_by_task(id)`, `delete(agent_id)`, `delete_by_conversation(id)`.

### TemplateStore

JSON file-based reusable plan template storage with `{{variable}}` substitution.

| Method | Description |
|--------|-------------|
| `new(data_dir)` | Create store (creates `templates.json`) |
| `save(template)` | Save a template |
| `get(template_id)` | Get by ID |
| `get_by_name(name)` | Case-insensitive partial match |
| `list()` | List all sorted by usage count |
| `list_by_category(category)` | Filter by category |
| `search(query)` | Search name, description, tags |
| `delete(template_id)` | Delete template |
| `update(template)` | Update template |
| `mark_used(template_id)` | Increment usage counter |

**`PlanTemplate`:**

| Field | Type | Description |
|-------|------|-------------|
| `template_id` | `String` | Unique identifier |
| `name` | `String` | Template name |
| `description` | `String` | What this template is for |
| `content` | `String` | Template content with `{{variables}}` |
| `category` | `Option<String>` | Template category |
| `tags` | `Vec<String>` | Searchable tags |
| `variables` | `Vec<String>` | Auto-extracted `{{variable}}` names |
| `usage_count` | `u32` | Times used |

| Method (on `PlanTemplate`) | Description |
|-----------------------------|-------------|
| `new(name, description, content)` | Create template (auto-extracts variables) |
| `from_plan(name, description, content, plan_id)` | Create from existing plan |
| `with_category(category)` | Builder: set category |
| `with_tags(tags)` | Builder: set tags |
| `instantiate(substitutions)` | Replace `{{var}}` placeholders |
| `mark_used()` | Increment counter and update timestamp |

### PersistentTaskManager (requires `agents` feature)

Task manager that auto-persists to LanceDB on every mutation for agent integration.

| Method | Description |
|--------|-------------|
| `new(client, conversation_id)` | Create and load existing tasks |
| `new_for_plan(client, conversation_id, plan_id)` | Create scoped to a plan |
| `create_task(description, parent_id, priority)` | Create and persist immediately |
| `add_subtask(parent_id, description)` | Add child task |
| `start_task(task_id)` | Mark in-progress |
| `complete_task(task_id, summary)` | Mark completed |
| `fail_task(task_id, error)` | Mark failed |
| `add_dependency(task_id, depends_on)` | Add task dependency |
| `persist_all()` | Force save all tasks |
| `reload()` | Refresh from storage |
| `clear()` | Clear memory and storage |

**Read-only delegates:** `get_task()`, `get_ready_tasks()`, `get_root_tasks()`, `get_task_tree()`, `get_all_tasks()`, `get_tasks_by_status()`, `count()`, `get_stats()`, `get_progress()`, `get_overall_progress()`, `format_tree()`.

**Type alias:** `SharedPersistentTaskManager = Arc<RwLock<PersistentTaskManager>>` for multi-agent sharing.

## Usage Examples

### Store and search conversation messages

```rust
use brainwires_storage::{LanceClient, EmbeddingProvider, MessageStore, MessageMetadata};
use std::sync::Arc;

let client = Arc::new(LanceClient::new("~/.brainwires/db").await?);
let embeddings = Arc::new(EmbeddingProvider::new()?);
client.initialize(embeddings.dimension()).await?;

let store = MessageStore::new(client.clone(), embeddings.clone());

// Add messages
store.add(MessageMetadata {
    message_id: "msg-001".into(),
    conversation_id: "conv-001".into(),
    role: "assistant".into(),
    content: "We should use B-tree indexes for the user lookup table".into(),
    token_count: Some(35),
    model_id: None,
    images: None,
    created_at: chrono::Utc::now().timestamp(),
    expires_at: None,
}).await?;

// Semantic search
let results = store.search("database indexing strategy", 5, 0.7).await?;
for (msg, score) in &results {
    println!("[{:.2}] {}", score, msg.content);
}

// Search within a conversation
let results = store.search_conversation("conv-001", "indexing", 3, 0.6).await?;
```

### Use tiered memory for infinite context

```rust
use brainwires_storage::{
    TieredMemory, TieredMemoryConfig, MessageStore, MessageMetadata,
    MemoryTier, LanceClient, EmbeddingProvider,
};
use std::sync::Arc;

let client = Arc::new(LanceClient::new("~/.brainwires/db").await?);
let embeddings = Arc::new(EmbeddingProvider::new()?);
client.initialize(embeddings.dimension()).await?;

let hot_store = Arc::new(MessageStore::new(client.clone(), embeddings.clone()));

let config = TieredMemoryConfig {
    hot_retention_hours: 12,
    warm_retention_hours: 168,
    max_hot_messages: 500,
    session_ttl_hours: 24,
    ..TieredMemoryConfig::default()
};

let mut memory = TieredMemory::new(hot_store, client.clone(), embeddings.clone(), config);

// Add message to hot tier
memory.add_message(MessageMetadata {
    message_id: "msg-042".into(),
    conversation_id: "conv-001".into(),
    role: "assistant".into(),
    content: "JWT tokens expire after 15 minutes".into(),
    token_count: Some(20),
    model_id: None,
    images: None,
    created_at: chrono::Utc::now().timestamp(),
    expires_at: None,
}, 0.8).await?;

// Search across all tiers with multi-factor scoring
let results = memory.search_adaptive_multi_factor("token expiration", Some("conv-001")).await?;
for result in &results {
    println!("[{:?} {:.2}] {}", result.tier, result.score, result.content);
}

// Demote old messages to warm tier
let candidates = memory.get_demotion_candidates(MemoryTier::Hot, 10).await?;
for msg_id in candidates {
    let summary = memory.fallback_summarize("original content here");
    memory.demote_to_warm(&msg_id, brainwires_storage::tiered_memory::MessageSummary {
        summary_id: uuid::Uuid::new_v4().to_string(),
        original_message_id: msg_id.clone(),
        conversation_id: "conv-001".into(),
        summary,
        key_entities: vec!["JWT".into(), "token".into()],
        created_at: chrono::Utc::now().timestamp(),
    }).await?;
}
```

### Index and search documents with hybrid retrieval

```rust
use brainwires_storage::{
    DocumentStore, DocumentScope, DocumentSearchRequest, DocumentType,
    LanceClient, EmbeddingProvider,
};
use std::sync::Arc;
use std::path::Path;

let client = Arc::new(LanceClient::new("~/.brainwires/db").await?);
let embeddings = Arc::new(EmbeddingProvider::new()?);
client.initialize(embeddings.dimension()).await?;

let store = DocumentStore::new(client.clone(), embeddings.clone(), "~/.brainwires/bm25");

// Index a file
let metadata = store.index_file(
    Path::new("docs/architecture.md"),
    DocumentScope::Project("my-project".into()),
).await?;
println!("Indexed: {} ({} chunks)", metadata.title.unwrap_or_default(), metadata.chunk_count);

// Hybrid search (vector + BM25)
let results = store.search(DocumentSearchRequest {
    query: "authentication flow".into(),
    limit: 10,
    min_score: 0.5,
    conversation_id: None,
    project_id: Some("my-project".into()),
    file_types: None,
    use_hybrid: true,
}).await?;

for result in &results {
    println!("[{:.2}] {} (chunk {})", result.score, result.document_id, result.chunk_index);
    println!("  {}", result.content);
}
```

### Track entities and detect contradictions

> **Note:** `EntityStore` and `RelationshipGraph` have moved to `brainwires-brain`.

```rust
use brainwires_brain::{EntityStore, Entity, EntityType, Relationship, ExtractionResult};

let mut store = EntityStore::new();

// Add extracted entities from messages
store.add_extraction(ExtractionResult {
    entities: vec![
        Entity {
            name: "auth.rs".into(),
            entity_type: EntityType::File,
            message_ids: vec![],
            first_seen: 0,
            last_seen: 0,
            mention_count: 0,
        },
    ],
    relationships: vec![
        Relationship::Defines {
            source: "auth.rs".into(),
            target: "validate_token".into(),
        },
    ],
}, "msg-001", chrono::Utc::now().timestamp());

// Check for contradictions (memory poisoning protection)
let contradictions = store.drain_contradictions();
for c in &contradictions {
    println!("Contradiction: {:?} on {}", c.kind, c.subject);
    println!("  Existing: {}", c.existing_context);
    println!("  New: {}", c.new_context);
}

// Query entities
let top = store.get_top_entities(5);
for entity in top {
    println!("{} ({:?}): {} mentions", entity.name, entity.entity_type, entity.mention_count);
}
```

### Coordinate multi-process access with locks

```rust
use brainwires_storage::LockStore;
use std::time::Duration;

let locks = LockStore::new_default().await?;

// Acquire a write lock with 30-second timeout
let acquired = locks.try_acquire(
    "file_write",
    "src/main.rs",
    "agent-001",
    Some(Duration::from_secs(30)),
).await?;

if acquired {
    // Do exclusive work on file...
    println!("Lock acquired, writing file");

    // Release when done
    locks.release("file_write", "src/main.rs", "agent-001").await?;
}

// Cleanup stale locks from dead processes
let cleaned = locks.cleanup_stale().await?;
println!("Cleaned {} stale locks", cleaned);

// Check lock statistics
let stats = locks.stats().await?;
println!("Active: {} total, {} writes, {} stale", stats.total_locks, stats.file_write_locks, stats.stale_locks);
```

### Create and use plan templates

```rust
use brainwires_storage::{TemplateStore, PlanTemplate};
use std::collections::HashMap;

let store = TemplateStore::new("~/.brainwires/templates")?;

// Create a template from a successful plan
let template = PlanTemplate::new(
    "API Endpoint".into(),
    "Template for adding a new REST API endpoint".into(),
    "1. Create handler in src/handlers/{{handler_name}}.rs\n\
     2. Add route in src/routes.rs for {{method}} {{path}}\n\
     3. Add tests in tests/{{handler_name}}_test.rs".into(),
)
.with_category("backend".into())
.with_tags(vec!["api".into(), "rest".into(), "endpoint".into()]);

store.save(&template)?;
println!("Variables: {:?}", template.variables);
// → ["handler_name", "method", "path"]

// Instantiate with values
let mut vars = HashMap::new();
vars.insert("handler_name".into(), "create_user".into());
vars.insert("method".into(), "POST".into());
vars.insert("path".into(), "/api/users".into());

let plan = template.instantiate(&vars);
println!("{}", plan);

// Search templates
let results = store.search("api endpoint")?;
for t in &results {
    println!("{}: {} (used {} times)", t.name, t.description, t.usage_count);
}
```

## Integration

Use via the `brainwires` facade crate with the `storage` feature, or depend on `brainwires-storage` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.1", features = ["storage"] }

# Direct
[dependencies]
brainwires-storage = "0.1"
```

The crate re-exports all components at the top level:

```rust
use brainwires_storage::{
    // Always available
    DocumentType, DocumentMetadata, DocumentChunk, DocumentSearchRequest, DocumentSearchResult,
    ExtractedDocument, ChunkerConfig, DocumentChunker,
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
    Entity, EntityType, Relationship, ExtractionResult,
    ContradictionEvent, ContradictionKind, EntityStoreStats,
    RelationshipGraph, GraphNode, GraphEdge, EdgeType, EntityContext,
    PlanTemplate, TemplateStore,
};

// Native-only
#[cfg(feature = "native")]
use brainwires_storage::{
    LanceClient, EmbeddingProvider,
    ConversationMetadata, ConversationStore,
    MessageMetadata, MessageStore,
    TaskMetadata, TaskStore, AgentStateMetadata, AgentStateStore,
    PlanStore,
    LockStore, LockRecord, LockStats,
    DocumentProcessor, DocumentMetadataStore,
    DocumentBM25Manager, DocumentBM25Result, DocumentBM25Stats,
    DocumentStore, DocumentScope,
    ImageStore,
    SummaryStore, FactStore, TierMetadataStore,
    CanonicalWriteToken, MemoryAuthority, MemoryTier,
    MultiFactorScore, TieredMemory, TieredMemoryConfig, TieredSearchResult,
    FileChunk, FileContent, FileContextManager,
    EntityStore,
};

// Agent integration
#[cfg(all(feature = "native", feature = "agents"))]
use brainwires_storage::{PersistentTaskManager, SharedPersistentTaskManager};
```

A `prelude` module is also available for convenient imports:

```rust
use brainwires_storage::prelude::*;
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
