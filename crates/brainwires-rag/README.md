# brainwires-rag

[![Crates.io](https://img.shields.io/crates/v/brainwires-rag.svg)](https://crates.io/crates/brainwires-rag)
[![Documentation](https://img.shields.io/docsrs/brainwires-rag)](https://docs.rs/brainwires-rag)
[![License](https://img.shields.io/crates/l/brainwires-rag.svg)](LICENSE)

RAG-based codebase indexing and semantic search for the Brainwires Agent Framework.

## Overview

`brainwires-rag` is a Rust library crate that provides RAG (Retrieval-Augmented Generation) capabilities for understanding and searching large codebases via `RagClient`. The MCP server binary is provided separately by the `brainwires-rag-server` crate (in `extras/brainwires-rag-server/`).

**Design principles:**

- **Hybrid search** — combines FastEmbed vector similarity with Tantivy BM25 keyword matching via Reciprocal Rank Fusion (RRF) for optimal results
- **AST-aware chunking** — Tree-sitter parsing extracts semantic units (functions, classes, methods) for 12 languages, with fixed-line fallback for others
- **Dual database backends** — embedded LanceDB (default, zero external dependencies) or external Qdrant server
- **Smart incremental indexing** — persistent SHA-256 hash cache auto-detects changed files; cross-process filesystem locks prevent corruption
- **Code navigation** — find definitions, references, and call graphs with hybrid precision (AST-based for all languages)
- **Git history search** — semantic search over commit messages and diffs with on-demand indexing
- **Local-first** — all processing happens locally using `fastembed` (all-MiniLM-L6-v2); no API keys, no network calls
- **Library-first** — use as a Rust library; MCP server binary is in `extras/brainwires-rag-server/` (9 tools, 9 slash commands)

```text
  ┌──────────────────────────────────────────────────────────────────────┐
  │                         brainwires-rag                               │
  │                                                                      │
  │  ┌─── RagClient (Library API) ────────────────────────────────────┐  │
  │  │                                                                 │  │
  │  │  index_codebase() ──► FileWalker ──► CodeChunker ──► Embedder  │  │
  │  │       │                 (.gitignore    (Tree-sitter    (FastEmbed│  │
  │  │       │                  aware)         AST parsing)   MiniLM)  │  │
  │  │       ▼                                     │                   │  │
  │  │  HashCache (SHA-256)                        ▼                   │  │
  │  │  (incremental updates)              VectorDatabase              │  │
  │  │                                    ┌────────┴────────┐          │  │
  │  │  query_codebase() ──►              │                 │          │  │
  │  │  search_by_filters() ──►     LanceDB           Qdrant          │  │
  │  │                              (embedded)        (external)       │  │
  │  │                                    │                            │  │
  │  │  search_git_history() ──►    BM25 (Tantivy) ◄── Hybrid RRF    │  │
  │  │  find_definition() ──►       RelationsProvider                  │  │
  │  │  find_references() ──►       (AST-based symbol extraction)      │  │
  │  │  get_call_graph() ──►                                           │  │
  │  └─────────────────────────────────────────────────────────────────┘  │
  │                                                                      │
  │  MCP server binary: extras/brainwires-rag-server/                    │
  │  (9 MCP Tools, 9 Slash Commands, Stdio transport)                   │
  └──────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-rag = "0.1"
```

Index a codebase and search it:

```rust
use brainwires_rag::{RagClient, IndexRequest, QueryRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RagClient::new().await?;

    // Index a codebase (auto-detects full vs incremental)
    let index_req = IndexRequest {
        path: "/path/to/codebase".to_string(),
        project: Some("my-project".to_string()),
        include_patterns: vec!["**/*.rs".to_string()],
        exclude_patterns: vec!["**/target/**".to_string()],
        max_file_size: 1_048_576,
    };
    let response = client.index_codebase(index_req).await?;
    println!("Indexed {} files, {} chunks", response.files_indexed, response.chunks_created);

    // Semantic search with hybrid vector + keyword matching
    let query_req = QueryRequest {
        query: "authentication middleware".to_string(),
        project: Some("my-project".to_string()),
        limit: 10,
        min_score: 0.7,
        hybrid: true,
        path: None,
    };
    let results = client.query_codebase(query_req).await?;
    for r in results.results {
        println!("{} (L{}-{}): score {:.2}", r.file_path, r.start_line, r.end_line, r.score);
    }

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Enables all heavy dependencies: Tree-sitter, FastEmbed, LanceDB, Tantivy, git2 |
| `wasm` | No | WASM-compatible build (types and error modules only, no embeddings or databases) |
| `lancedb-backend` | Yes | Embedded LanceDB vector database (no external dependencies) |
| `qdrant-backend` | No | External Qdrant vector database server support |
| `alt-folder-name` | No | Custom folder naming for data storage paths |
| `stack-graphs` | No | High-precision code navigation (prepared, not fully implemented) |

```toml
# Default (native with embedded LanceDB)
brainwires-rag = "0.1"

# With Qdrant instead of LanceDB
brainwires-rag = { version = "0.1", features = ["native", "qdrant-backend"] }

# WASM target (types only, no processing)
brainwires-rag = { version = "0.1", default-features = false, features = ["wasm"] }
```

## Architecture

### RagClient

The main library interface providing all RAG functionality.

| Method | Description |
|--------|-------------|
| `new()` | Create with default config (loads from file or defaults + env overrides) |
| `with_config(config)` | Create with custom `Config` |
| `index_codebase(req)` | Smart indexing — auto-detects full or incremental mode |
| `query_codebase(req)` | Hybrid semantic + keyword search with adaptive thresholds |
| `search_by_filters(req)` | Filtered search by language, extension, or path pattern |
| `get_statistics()` | Index statistics (files, chunks, embeddings, language breakdown) |
| `clear_index()` | Clear all indexed data |
| `search_git_history(req)` | Semantic search over git commit history |
| `find_definition(req)` | Find where a symbol is defined (LSP-like) |
| `find_references(req)` | Find all references to a symbol |
| `get_call_graph(req)` | Get callers and callees for a function |

**Internal components:**

| Component | Role |
|-----------|------|
| `FastEmbedManager` | Local embedding generation (all-MiniLM-L6-v2, 384 dimensions) |
| `LanceVectorDB` / `QdrantVectorDB` | Vector storage and similarity search |
| `CodeChunker` | AST-based and fixed-line code chunking |
| `HashCache` | Persistent SHA-256 cache for incremental updates |
| `GitCache` | Tracks indexed commits per repository |
| `HybridRelationsProvider` | Code navigation (definitions, references, call graphs) |

### Indexing Pipeline

#### FileWalker

Traverses directories respecting `.gitignore` via the `ignore` crate. Detects 40+ file types across three categories:

| Category | Count | Examples |
|----------|-------|---------|
| Programming Languages | 24 | Rust, Python, JavaScript, TypeScript, Go, Java, Swift, C/C++, C#, Ruby, PHP, Kotlin, Scala, Shell, SQL |
| Documentation | 8 | Markdown, PDF (auto-converted to Markdown), RST, AsciiDoc, Org, Text |
| Configuration | 8 | JSON, YAML, TOML, XML, INI, Properties, .env |

#### CodeChunker

Two chunking strategies:

| Strategy | When Used | Description |
|----------|-----------|-------------|
| **AST-Based** | 12 languages with Tree-sitter support | Extracts semantic units (functions, classes, methods, structs) |
| **Fixed-Lines** | All other file types | 50 lines per chunk (configurable) |

Supported Tree-sitter languages: Rust, Python, JavaScript, TypeScript, Go, Java, Swift, C, C++, C#, Ruby, PHP.

#### Embedding

| Property | Value |
|----------|-------|
| Model | all-MiniLM-L6-v2 |
| Dimensions | 384 |
| Library | FastEmbed (ONNX runtime) |
| Batch size | 8 (configurable) |
| Performance | ~500 embeddings/second |
| Privacy | Fully local, no API calls |

### Vector Database

**Trait-based design** — `VectorDatabase` trait with two implementations:

#### LanceDB (Default)

- Embedded, zero external dependencies
- Apache Arrow columnar storage with ACID transactions
- Zero-copy memory-mapped files
- Hybrid search: vector similarity + Tantivy BM25 with RRF fusion
- Stored at `~/.local/share/brainwires-rag/lancedb/`

#### Qdrant (Optional)

- External server at `http://localhost:6334`
- High-performance vector similarity search
- Self-hosted or cloud-hosted

### Hybrid Search

Combines two ranking signals using Reciprocal Rank Fusion (RRF):

| Signal | Method | Description |
|--------|--------|-------------|
| **Vector** | Cosine similarity | Semantic meaning from embeddings |
| **Keyword** | Tantivy BM25 | Exact token matching |
| **Combined** | RRF (k=60) | `1/(k + rank_vector) + 1/(k + rank_keyword)` |

**Adaptive thresholds:** when no results are found, the threshold is automatically lowered step-by-step: 0.7 → 0.6 → 0.5 → 0.4 → 0.3.

### Code Relations

Provides lightweight LSP-like code navigation:

| Feature | Description |
|---------|-------------|
| **Find Definition** | Locate where symbols are defined |
| **Find References** | Find all usages, categorized by type (Call, Read, Write, Import, TypeReference, Inheritance, Instantiation) |
| **Get Call Graph** | Analyze function relationships (callers and callees to configurable depth) |

Uses `HybridRelationsProvider` which falls back to `RepoMapProvider` (AST-based symbol extraction and identifier matching) for all Tree-sitter-supported languages.

### Git History Search

| Feature | Description |
|---------|-------------|
| **On-demand indexing** | Default: indexes only 10 most recent commits, expands as needed |
| **Commit content** | Indexes both commit messages and diff content |
| **Filtering** | Author, date range, branch, file pattern |
| **Smart caching** | `GitCache` prevents re-indexing the same commits |

### Cross-Process Coordination

| Mechanism | Scope | Purpose |
|-----------|-------|---------|
| **Filesystem locks** (`flock`) | Cross-process | Prevents multiple processes from indexing simultaneously |
| **Broadcast channels** | In-process | Shares indexing results to waiting tasks |
| **Dirty index tracking** | Persistent | Detects and recovers from interrupted indexing operations |

## Request / Response Types

### IndexRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Directory path to index |
| `project` | `Option<String>` | `None` | Project name for multi-project support |
| `include_patterns` | `Vec<String>` | `[]` | Glob patterns to include (e.g., `["**/*.rs"]`) |
| `exclude_patterns` | `Vec<String>` | `[]` | Glob patterns to exclude (e.g., `["**/target/**"]`) |
| `max_file_size` | `usize` | `1_048_576` | Maximum file size in bytes (1 MB) |

### IndexResponse

| Field | Type | Description |
|-------|------|-------------|
| `mode` | `IndexingMode` | `Full` or `Incremental` |
| `files_indexed` | `usize` | Files successfully indexed |
| `chunks_created` | `usize` | Code chunks created |
| `embeddings_generated` | `usize` | Embeddings generated |
| `duration_ms` | `u64` | Time taken |
| `errors` | `Vec<String>` | Non-fatal errors encountered |
| `files_updated` | `usize` | Files updated (incremental only) |
| `files_removed` | `usize` | Files removed (incremental only) |

### QueryRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | `String` | — | Natural language search query |
| `path` | `Option<String>` | `None` | Filter by indexed codebase path |
| `project` | `Option<String>` | `None` | Filter by project name |
| `limit` | `usize` | `10` | Maximum results to return |
| `min_score` | `f32` | `0.7` | Minimum similarity score (0.0–1.0) |
| `hybrid` | `bool` | `true` | Enable hybrid vector + keyword search |

### SearchResult

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `String` | File path relative to indexed root |
| `root_path` | `Option<String>` | Absolute path to indexed root |
| `content` | `String` | The matching code chunk |
| `score` | `f32` | Combined similarity score (0.0–1.0) |
| `vector_score` | `f32` | Vector similarity score |
| `keyword_score` | `Option<f32>` | BM25 keyword score (hybrid only) |
| `start_line` | `usize` | Starting line number |
| `end_line` | `usize` | Ending line number |
| `language` | `String` | Detected programming language |
| `project` | `Option<String>` | Project name |
| `indexed_at` | `i64` | Timestamp when chunk was indexed (Unix epoch seconds, default `0`) |

### SearchGitHistoryRequest

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | `String` | — | Search query for commit history |
| `path` | `String` | `"."` | Codebase path (discovers git repo) |
| `branch` | `Option<String>` | `None` | Branch name (default: current) |
| `max_commits` | `usize` | `10` | Maximum commits to index/search |
| `limit` | `usize` | `10` | Maximum results to return |
| `min_score` | `f32` | `0.7` | Minimum similarity score |
| `author` | `Option<String>` | `None` | Filter by author (regex) |
| `since` | `Option<String>` | `None` | Filter by start date (ISO 8601) |
| `until` | `Option<String>` | `None` | Filter by end date (ISO 8601) |
| `file_pattern` | `Option<String>` | `None` | Filter by file path (regex) |

### GitSearchResult

| Field | Type | Description |
|-------|------|-------------|
| `commit_hash` | `String` | Git commit SHA |
| `commit_message` | `String` | Commit message |
| `author` | `String` | Author name |
| `author_email` | `String` | Author email |
| `commit_date` | `i64` | Commit date (Unix timestamp) |
| `score` | `f32` | Combined similarity score |
| `vector_score` | `f32` | Vector similarity score |
| `keyword_score` | `Option<f32>` | Keyword match score |
| `files_changed` | `Vec<String>` | Files changed in commit |
| `diff_snippet` | `String` | Diff snippet (~500 chars) |

### Code Navigation Requests

**FindDefinitionRequest:** `file_path`, `line` (1-based), `column` (0-based), `project?`

**FindReferencesRequest:** `file_path`, `line`, `column`, `limit` (default: 100), `project?`, `include_definition` (default: true)

**GetCallGraphRequest:** `file_path`, `line`, `column`, `depth` (default: 2, max: 10), `project?`, `include_callers` (default: true), `include_callees` (default: true)

## Usage Examples

### Index and query with custom config

```rust
use brainwires_rag::{RagClient, Config, IndexRequest, QueryRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.embedding.batch_size = 16;
    config.search.min_score = 0.5;
    config.search.hybrid = true;

    let client = RagClient::with_config(config).await?;

    let req = IndexRequest {
        path: "/home/user/project".to_string(),
        project: Some("my-app".to_string()),
        include_patterns: vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
        exclude_patterns: vec!["**/target/**".to_string()],
        max_file_size: 1_048_576,
    };
    let resp = client.index_codebase(req).await?;
    println!("{:?} mode: {} files, {} chunks", resp.mode, resp.files_indexed, resp.chunks_created);

    Ok(())
}
```

### Advanced filtered search

```rust
use brainwires_rag::{RagClient, AdvancedSearchRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RagClient::new().await?;

    let req = AdvancedSearchRequest {
        query: "error handling patterns".to_string(),
        path: None,
        project: Some("my-app".to_string()),
        limit: 5,
        min_score: 0.6,
        file_extensions: vec!["rs".to_string()],
        languages: vec!["Rust".to_string()],
        path_patterns: vec!["src/".to_string()],
    };
    let results = client.search_by_filters(req).await?;
    for r in results.results {
        println!("[{}] {} L{}-{}", r.language, r.file_path, r.start_line, r.end_line);
    }

    Ok(())
}
```

### Search git history

```rust
use brainwires_rag::{RagClient, SearchGitHistoryRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RagClient::new().await?;

    let req = SearchGitHistoryRequest {
        query: "fix authentication bug".to_string(),
        path: "/home/user/project".to_string(),
        branch: None,
        max_commits: 50,
        limit: 5,
        min_score: 0.5,
        author: None,
        since: Some("2025-01-01".to_string()),
        until: None,
        file_pattern: None,
        project: None,
    };
    let resp = client.search_git_history(req).await?;
    for r in resp.results {
        println!("{} ({}) — {}", &r.commit_hash[..8], r.author, r.commit_message);
    }

    Ok(())
}
```

### Find definition and references

```rust
use brainwires_rag::{RagClient, FindDefinitionRequest, FindReferencesRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RagClient::new().await?;

    // Find where a symbol is defined
    let def_req = FindDefinitionRequest {
        file_path: "src/main.rs".to_string(),
        line: 42,
        column: 10,
        project: None,
    };
    let def_resp = client.find_definition(def_req).await?;
    if let Some(def) = def_resp.definition {
        println!("Defined at {}:{}:{}", def.file_path, def.line, def.column);
    }

    // Find all references to a symbol
    let ref_req = FindReferencesRequest {
        file_path: "src/main.rs".to_string(),
        line: 42,
        column: 10,
        limit: 50,
        project: None,
        include_definition: true,
    };
    let ref_resp = client.find_references(ref_req).await?;
    println!("Found {} references to {:?}", ref_resp.total_count, ref_resp.symbol_name);

    Ok(())
}
```

### Run as MCP server

The MCP server binary is in the separate `brainwires-rag-server` crate:

```bash
cargo run -p brainwires-rag-server -- serve
```

## MCP Tools & Slash Commands

When running the MCP server (`brainwires-rag-server`), 9 tools and 9 slash commands are exposed:

| Tool | Slash Command | Description |
|------|---------------|-------------|
| `index_codebase` | `/project:index` | Smart indexing (auto full or incremental) |
| `query_codebase` | `/project:query` | Semantic search with adaptive thresholds |
| `get_statistics` | `/project:stats` | Index statistics and language breakdown |
| `clear_index` | `/project:clear` | Clear all indexed data |
| `search_by_filters` | `/project:search` | Filtered search by language, extension, path |
| `search_git_history` | `/project:git-search` | Semantic git history search |
| `find_definition` | `/project:definition` | Find symbol definition |
| `find_references` | `/project:references` | Find all symbol references |
| `get_call_graph` | `/project:callgraph` | Function call graph analysis |

## Configuration

### Config

Configuration is loaded with priority: environment variables > config file > defaults.

#### VectorDbConfig

| Field | Type | Default | Env Var | Description |
|-------|------|---------|---------|-------------|
| `backend` | `String` | `"lancedb"` | `PROJECT_RAG_DB_BACKEND` | Database backend (`"lancedb"` or `"qdrant"`) |
| `lancedb_path` | `PathBuf` | `~/.local/share/brainwires-rag/lancedb/` | `PROJECT_RAG_LANCEDB_PATH` | LanceDB data directory |
| `qdrant_url` | `String` | `"http://localhost:6334"` | `PROJECT_RAG_QDRANT_URL` | Qdrant server URL |
| `collection_name` | `String` | `"code_embeddings"` | — | Vector collection name |

#### EmbeddingConfig

| Field | Type | Default | Env Var | Description |
|-------|------|---------|---------|-------------|
| `model_name` | `String` | `"all-MiniLM-L6-v2"` | `PROJECT_RAG_MODEL` | Embedding model name |
| `batch_size` | `usize` | `8` | `PROJECT_RAG_BATCH_SIZE` | Batch size per embedding call |
| `timeout_secs` | `u64` | `10` | — | Per-batch timeout in seconds |
| `cancellation_check_interval` | `usize` | `4` | — | Chunks between cancellation checks |

#### IndexingConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunk_size` | `usize` | `50` | Lines per chunk (fixed-lines strategy) |
| `max_file_size` | `usize` | `1_048_576` | Maximum file size to index (1 MB) |
| `include_patterns` | `Vec<String>` | `[]` | Default include glob patterns |
| `exclude_patterns` | `Vec<String>` | `["target", "node_modules", ".git", "dist", "build"]` | Default exclude patterns |

#### SearchConfig

| Field | Type | Default | Env Var | Description |
|-------|------|---------|---------|-------------|
| `min_score` | `f32` | `0.7` | `PROJECT_RAG_MIN_SCORE` | Default minimum similarity score (0.0–1.0) |
| `limit` | `usize` | `10` | — | Default result limit |
| `hybrid` | `bool` | `true` | — | Enable hybrid search by default |

#### CacheConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hash_cache_path` | `PathBuf` | `~/.local/share/brainwires-rag/hash_cache.json` | Persistent file hash cache |
| `git_cache_path` | `PathBuf` | `~/.local/share/brainwires-rag/git_cache.json` | Git commit tracking cache |

### Config file

Load from `~/.brainwires-rag/config.toml`:

```rust
use brainwires_rag::Config;
use std::path::Path;

let config = Config::from_file(Path::new("config.toml"))?;
// or
let config = Config::load_or_default()?;
// or with env overrides
let config = Config::new()?;
```

## Error Handling

`RagError` is a comprehensive error enum with domain-specific variants:

| Variant | Source | Description |
|---------|--------|-------------|
| `Embedding` | `EmbeddingError` | Model init, generation, timeout, dimension mismatch |
| `VectorDb` | `VectorDbError` | Connection, collection, store, search, clear |
| `Indexing` | `IndexingError` | File walking, reading errors |
| `Chunking` | `ChunkingError` | AST parsing, chunking errors |
| `Config` | `ConfigError` | File not found, parse failed, invalid values |
| `Validation` | `ValidationError` | Input validation failures |
| `Git` | `GitError` | Git repository operations |
| `Cache` | `CacheError` | Cache load/save errors |
| `Io` | `std::io::Error` | Filesystem I/O |

Helper methods: `to_user_string()`, `is_user_error()`, `is_retryable()`.

## Performance

| Metric | Value |
|--------|-------|
| Indexing speed | ~1000 files/minute |
| Search latency | 20–30 ms |
| Memory usage | ~100 MB base + 50 MB model + ~4 MB per 10k chunks |
| Storage per chunk | ~1.5 KB |
| Model download | ~50 MB (first run only, cached locally) |

## Integration

Use via the `brainwires` facade crate with the `rag` feature, or depend on `brainwires-rag` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.1", features = ["rag"] }

# Direct
[dependencies]
brainwires-rag = "0.1"
```

The crate re-exports all request/response types at the top level:

```rust
use brainwires_rag::{
    RagClient, Config,
    IndexRequest, IndexResponse, IndexingMode,
    QueryRequest, QueryResponse, SearchResult,
    AdvancedSearchRequest,
    SearchGitHistoryRequest, SearchGitHistoryResponse, GitSearchResult,
    FindDefinitionRequest, FindDefinitionResponse,
    FindReferencesRequest, FindReferencesResponse,
    GetCallGraphRequest, GetCallGraphResponse,
    StatisticsRequest, StatisticsResponse,
    ClearRequest, ClearResponse,
    RagError,
};
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
