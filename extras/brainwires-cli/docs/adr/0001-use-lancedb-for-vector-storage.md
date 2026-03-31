# ADR 0001: Use LanceDB for Vector Storage

## Status

Accepted

## Context

The brainwires-cli needs persistent storage for conversation history with semantic search capabilities. This enables "infinite context" - the ability to retrieve relevant past messages even from long-ago conversations.

Requirements:
- Store conversation messages with embeddings
- Fast semantic similarity search (< 100ms for typical queries)
- Local-first operation (no external services required)
- Support for tiered storage (hot/warm/cold)
- Easy to embed in a Rust application

## Options Considered

### 1. LanceDB

**Pros:**
- Pure Rust implementation
- Columnar storage (efficient for large datasets)
- Built-in vector search
- Single-file database (easy deployment)
- Active development, good Rust bindings

**Cons:**
- Relatively new project
- Less ecosystem tooling than established databases

### 2. Qdrant

**Pros:**
- Mature vector database
- Excellent search performance
- Strong community

**Cons:**
- Requires separate server process
- More complex deployment
- External dependency

### 3. SQLite + pgvector-style extension

**Pros:**
- Very mature SQLite
- Can use existing sqlite workflows

**Cons:**
- No native vector operations
- Would need custom extension or workaround
- Performance not optimized for vector ops

### 4. In-memory only (no persistence)

**Pros:**
- Simple implementation
- Fast reads/writes

**Cons:**
- Data lost on restart
- No "infinite context" capability
- Memory usage grows unbounded

## Decision

Use **LanceDB** for vector storage.

LanceDB provides the best balance of:
- Native Rust integration (no FFI complexity)
- Built-in vector search capabilities
- Local-first operation (single file, no server)
- Good performance for our use case

## Consequences

### Positive
- Single dependency for all storage needs
- Semantic search "just works"
- Easy deployment (no external services)
- Tiered storage can be implemented via different tables

### Negative
- Tied to LanceDB's evolution
- Less community resources than SQLite
- May need to handle schema migrations carefully

### Mitigations
- Abstract storage behind trait for potential future swap
- Monitor LanceDB project health and have contingency plan
- Document schema versioning approach

## References

- [LanceDB Documentation](https://lancedb.github.io/lancedb/)
- `src/storage/lance_client.rs` - Implementation
