# brainwires-brain

[![Crates.io](https://img.shields.io/crates/v/brainwires-brain.svg)](https://crates.io/crates/brainwires-brain)
[![Documentation](https://img.shields.io/docsrs/brainwires-brain)](https://docs.rs/brainwires-brain)
[![License](https://img.shields.io/crates/l/brainwires-brain.svg)](LICENSE)

Central knowledge crate for the Brainwires Agent Framework — persistent thought capture, semantic memory search, entity graphs, and knowledge systems (PKS/BKS).

## Overview

`brainwires-brain` is the canonical knowledge crate for the Brainwires framework. It provides persistent thought capture, semantic memory search, entity/relationship graphs, and knowledge retrieval via BrainClient. The MCP server binary is provided separately by the `brainwires-brain-server` crate (in `extras/brainwires-brain-server/`).

**Design principles:**

- **Thought-centric capture** — every piece of knowledge is a `Thought` with auto-detected category, extracted tags, and configurable importance (0.0–1.0)
- **Dual knowledge extraction** — captured thoughts automatically feed into PKS (Personal Knowledge System) for fact extraction and BKS (Behavioral Knowledge System) for behavioral truths
- **Semantic-first retrieval** — all thoughts are embedded via all-MiniLM-L6-v2 (384 dimensions) and stored in LanceDB for vector similarity search
- **Canonical persistence** — thoughts persist indefinitely with no TTL or auto-eviction, unlike session-scoped messages
- **Local-first** — all processing happens locally using `fastembed`; no API keys, no network calls
- **Library-first** — use as a Rust library; MCP server binary is in `extras/brainwires-brain-server/`
- **Entity & relationship graphs** — entity tracking with contradiction detection and relationship graphs with shortest-path scoring

```text
  ┌──────────────────────────────────────────────────────────────────────┐
  │                         brainwires-brain                            │
  │                                                                     │
  │  ┌─── BrainClient (Library API) ──────────────────────────────────┐ │
  │  │                                                                 │ │
  │  │  capture_thought() ──► Embed ──► LanceDB (vector store)        │ │
  │  │       │                          + PKS fact extraction          │ │
  │  │       │                                                         │ │
  │  │  search_memory() ──► Vector search (thoughts)                  │ │
  │  │                       + Keyword search (PKS facts)              │ │
  │  │                       + Merged & ranked by score                │ │
  │  │                                                                 │ │
  │  │  search_knowledge() ──► PKS (personal facts)                   │ │
  │  │                          + BKS (behavioral truths)              │ │
  │  │                                                                 │ │
  │  │  list_recent() / get_thought() / delete_thought()              │ │
  │  │  memory_stats() ──► Aggregate dashboard                        │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  │                                                                     │
  │  ┌─── Entity & Knowledge Graph ───────────────────────────────────┐ │
  │  │                                                                 │ │
  │  │  EntityStore ──► entity tracking, contradiction detection       │ │
  │  │  RelationshipGraph ──► nodes, edges, shortest path, scoring     │ │
  │  │  knowledge/ ──► PKS caches, BKS caches, fact extraction         │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  └──────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-brain = "0.1"
```

Capture a thought and search memory:

```rust
use brainwires_brain::{BrainClient, CaptureThoughtRequest, SearchMemoryRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = BrainClient::new().await?;

    // Capture a thought (category auto-detected as "decision")
    let resp = client.capture_thought(CaptureThoughtRequest {
        content: "Decided to use PostgreSQL for the auth service".into(),
        category: None,          // auto-detect
        tags: Some(vec!["db".into(), "auth".into()]),
        importance: Some(0.8),
        source: None,            // defaults to "manual"
    }).await?;
    println!("Captured: {} (category: {}, facts: {})", resp.id, resp.category, resp.facts_extracted);

    // Semantic search across all memory
    let results = client.search_memory(SearchMemoryRequest {
        query: "database choice for authentication".into(),
        limit: 10,
        min_score: 0.6,
        category: None,
        sources: None,
    }).await?;
    for r in &results.results {
        println!("[{:.2}] [{}] {}", r.score, r.source, r.content);
    }

    Ok(())
}
```

## Architecture

### BrainClient

The main library interface orchestrating all storage operations.

| Method | Description |
|--------|-------------|
| `new()` | Create with default paths (`~/.brainwires/brain/`, `pks.db`, `bks.db`) |
| `with_paths(lance, pks, bks)` | Create with explicit paths (useful for testing) |
| `capture_thought(req)` | Embed, store, auto-detect category, extract PKS facts |
| `search_memory(req)` | Semantic vector search on thoughts + keyword search on PKS facts |
| `list_recent(req)` | Time-filtered listing with optional category filter |
| `get_thought(id)` | Retrieve a single thought by UUID |
| `search_knowledge(req)` | Query PKS personal facts and BKS behavioral truths |
| `memory_stats()` | Aggregate dashboard (counts, categories, recency, top tags) |
| `delete_thought(id)` | Hard-delete a thought from LanceDB |

**Internal components:**

| Component | Role |
|-----------|------|
| `LanceClient` | LanceDB connection and table management (from `brainwires-storage`) |
| `EmbeddingProvider` | Local embedding generation (all-MiniLM-L6-v2, 384 dimensions) |
| `PersonalKnowledgeCache` | SQLite-backed personal fact storage and retrieval |
| `BehavioralKnowledgeCache` | SQLite-backed behavioral truth storage and retrieval |
| `PersonalFactCollector` | Regex-based fact extraction from thought content |

### Data Model

#### Thought

The primary unit of knowledge capture.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | UUID v4 identifier |
| `content` | `String` | The thought text |
| `category` | `ThoughtCategory` | Classification (auto-detected or manual) |
| `tags` | `Vec<String>` | Extracted #hashtags + user-provided tags |
| `source` | `ThoughtSource` | How the thought was captured |
| `importance` | `f32` | Importance score (0.0–1.0, default: 0.5) |
| `created_at` | `i64` | Creation timestamp (Unix) |
| `updated_at` | `i64` | Last update timestamp (Unix) |
| `deleted` | `bool` | Deletion flag |

Builder pattern: `Thought::new(content).with_category(cat).with_tags(tags).with_importance(0.8)`

#### ThoughtCategory

| Variant | Auto-detection Keywords |
|---------|------------------------|
| `Decision` | decided, decision, chose, chosen, agreed |
| `Person` | regex: names with "spoke to", "met with", "talked to" |
| `Insight` | realized, noticed, learned, turns out, discovery |
| `MeetingNote` | regex: `\bsync\b`, meeting, standup, retrospective |
| `Idea` | idea, what if, maybe we could, brainstorm |
| `ActionItem` | todo, need to, must, deadline, by end of |
| `Reference` | link, url, http, documentation, reference |
| `General` | fallback when no keywords match |

#### ThoughtSource

| Variant | Description |
|---------|-------------|
| `ManualCapture` | Explicitly captured by user (default) |
| `ConversationExtract` | Extracted from a conversation |
| `Import` | Imported from external source |

### Storage

#### LanceDB (Thoughts)

Thoughts are stored as Arrow RecordBatches in LanceDB with a 384-dimension embedding vector for semantic search. The vector search uses cosine distance, converted to a similarity score: `1.0 / (1.0 + distance)`.

| Property | Value |
|----------|-------|
| Table | `thoughts` |
| Embedding model | all-MiniLM-L6-v2 |
| Dimensions | 384 |
| Default path | `~/.brainwires/brain/` |
| Search method | Vector similarity with distance-to-score conversion |

#### PKS (Personal Knowledge System)

SQLite-backed cache that stores extracted personal facts (identity, preferences, capabilities, constraints). Facts are automatically extracted from captured thoughts via `PersonalFactCollector`.

| Property | Value |
|----------|-------|
| Default path | `~/.brainwires/pks.db` |
| Queue size | 1000 |
| Search | Keyword matching on fact keys and values |

#### BKS (Behavioral Knowledge System)

SQLite-backed cache that stores behavioral truths — patterns and rules observed across interactions.

| Property | Value |
|----------|-------|
| Default path | `~/.brainwires/bks.db` |
| Queue size | 1000 |
| Search | Context pattern matching with confidence scoring |

## Request / Response Types

### CaptureThoughtRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The thought text to capture |
| `category` | `Option<String>` | `None` | Category override (auto-detected if omitted) |
| `tags` | `Option<Vec<String>>` | `None` | Additional tags (merged with auto-extracted #hashtags) |
| `importance` | `Option<f32>` | `0.5` | Importance score (0.0–1.0) |
| `source` | `Option<String>` | `"manual"` | Source identifier |

### CaptureThoughtResponse

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | UUID of the captured thought |
| `category` | `String` | Detected or provided category |
| `tags` | `Vec<String>` | All tags (auto-extracted + user-provided) |
| `importance` | `f32` | Final importance score |
| `facts_extracted` | `usize` | Number of PKS facts extracted |

### SearchMemoryRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | `String` | — | Natural language search query |
| `limit` | `usize` | `10` | Maximum results |
| `min_score` | `f32` | `0.6` | Minimum similarity score (0.0–1.0) |
| `category` | `Option<String>` | `None` | Filter by ThoughtCategory |
| `sources` | `Option<Vec<String>>` | `None` | Filter by source: `"thoughts"`, `"facts"`, or both |

### SearchMemoryResponse

| Field | Type | Description |
|-------|------|-------------|
| `results` | `Vec<MemorySearchResult>` | Ranked results from thoughts and/or facts |
| `total` | `usize` | Total results returned |

### MemorySearchResult

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | Thought text or fact key-value |
| `score` | `f32` | Relevance score (0.0–1.0) |
| `source` | `String` | `"thoughts"` or `"facts"` |
| `thought_id` | `Option<String>` | Thought UUID (thoughts only) |
| `category` | `Option<String>` | Category |
| `tags` | `Option<Vec<String>>` | Tags (thoughts only) |
| `created_at` | `Option<i64>` | Creation timestamp |

### ListRecentRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `usize` | `20` | Maximum results |
| `category` | `Option<String>` | `None` | Filter by category |
| `since` | `Option<String>` | 7 days ago | ISO 8601 timestamp |

### SearchKnowledgeRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | `String` | — | Context to match against |
| `source` | `Option<String>` | `"all"` | `"personal"` (PKS), `"behavioral"` (BKS), or `"all"` |
| `category` | `Option<String>` | `None` | PKS/BKS category filter |
| `min_confidence` | `f32` | `0.5` | Minimum confidence threshold |
| `limit` | `usize` | `10` | Maximum results |

### MemoryStatsResponse

| Field | Type | Description |
|-------|------|-------------|
| `thoughts` | `ThoughtStats` | Total count, by_category, recent_24h/7d/30d, top_tags |
| `pks` | `PksStats` | total_facts, by_category, avg_confidence |
| `bks` | `BksStats` | total_truths, by_category |

## Usage Examples

### Run as MCP server

The MCP server binary is in the separate `brainwires-brain-server` crate:

```bash
cargo run -p brainwires-brain-server -- serve
```

### Search knowledge systems directly

```rust
use brainwires_brain::{BrainClient, SearchKnowledgeRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = BrainClient::new().await?;

    let results = client.search_knowledge(SearchKnowledgeRequest {
        query: "preferred programming language".into(),
        source: Some("personal".into()),  // PKS only
        category: None,
        min_confidence: 0.5,
        limit: 5,
    })?;

    for r in &results.results {
        println!("[{:.2}] {}: {} = {}", r.confidence, r.source, r.key, r.value);
    }

    Ok(())
}
```

### Get memory statistics

```rust
use brainwires_brain::BrainClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = BrainClient::new().await?;
    let stats = client.memory_stats().await?;

    println!("Thoughts: {} total ({} in last 24h)", stats.thoughts.total, stats.thoughts.recent_24h);
    println!("PKS facts: {} (avg confidence: {:.2})", stats.pks.total_facts, stats.pks.avg_confidence);
    println!("BKS truths: {}", stats.bks.total_truths);

    for (tag, count) in &stats.thoughts.top_tags {
        println!("  #{}: {}", tag, count);
    }

    Ok(())
}
```

## MCP Tools & Prompts

When running as an MCP server, 7 tools and 5 prompts are exposed:

| Tool | Description |
|------|-------------|
| `capture_thought` | Capture a thought with auto-detection, embedding, and PKS extraction |
| `search_memory` | Semantic search across thoughts and PKS facts |
| `list_recent` | Browse recent thoughts with category and time filters |
| `get_thought` | Retrieve a specific thought by UUID |
| `search_knowledge` | Query PKS personal facts and BKS behavioral truths |
| `memory_stats` | Dashboard of counts, categories, recency, and top tags |
| `delete_thought` | Delete a thought by UUID |

| Prompt | Description |
|--------|-------------|
| `capture` | Capture a new thought into persistent memory |
| `search` | Semantic search across all memory |
| `recent` | List recently captured thoughts |
| `stats` | Show memory statistics dashboard |
| `knowledge` | Search personal and behavioral knowledge |

### Claude Desktop configuration

Add to `~/.claude/mcp_servers.json` (using the `brainwires-brain-server` binary):

```json
{
  "brainwires-brain": {
    "command": "/path/to/brainwires-brain-server",
    "args": ["serve"]
  }
}
```

## Integration

Use via the `brainwires` facade crate with the `brain` feature, or depend on `brainwires-brain` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.2", features = ["brain"] }

# Direct
[dependencies]
brainwires-brain = "0.1"
```

The crate re-exports all primary types at the top level:

```rust
use brainwires_brain::{
    // Core
    BrainClient,
    Thought, ThoughtCategory, ThoughtSource,
    // Entity & Relationship Graph
    Entity, EntityStore, EntityType, Relationship, RelationshipGraph,
    // Requests
    CaptureThoughtRequest, SearchMemoryRequest, ListRecentRequest,
    GetThoughtRequest, SearchKnowledgeRequest, MemoryStatsRequest,
    DeleteThoughtRequest,
    // Responses
    CaptureThoughtResponse, SearchMemoryResponse, ListRecentResponse,
    GetThoughtResponse, SearchKnowledgeResponse, MemoryStatsResponse,
    DeleteThoughtResponse,
};
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
