# Brainwires CLI - Copilot Instructions

## Project Overview

Brainwires CLI is a research-driven AI agent CLI tool implementing cutting-edge papers like SEAL (Self-Evolving Agentic Learning) and MDAP (Massively Decomposed Agentic Processes). Built in Rust with ~42,000+ LOC across 220+ files.

## Architecture & Core Patterns

### Multi-Layered Agent System
- **Orchestrator Agent** (`src/agents/orchestrator.rs`): Main coordinator with 25-iteration limit
- **Worker Agents** (`src/agents/worker.rs`): Parallel task execution
- **SEAL Processor** (`src/seal/`): Context resolution, query extraction, learning
- **Tool Executor** (`src/tools/executor.rs`): 20+ integrated tools with permission control

### Storage Architecture (LanceDB + SQLite + Tantivy)
```rust
// Primary pattern: LanceDB for vectors, SQLite for locking, Tantivy for BM25
src/storage/
├── lance_client.rs        # Vector operations wrapper
├── conversation_store.rs  # Infinite context storage
├── relationship_graph.rs  # Entity graph (SEAL)
├── tiered_memory.rs      # Hot/warm/cold memory
```

### Tool System Pattern
```rust
// Tools are registered and executed through unified interface
// Key files: src/tools/{registry.rs, executor.rs, smart_router.rs}
pub trait Tool: Send + Sync {
    async fn execute(&self, args: ToolArgs, context: &ExecutionContext) -> ToolResult;
}
```

## Critical Build & Test Commands

```bash
# IMPORTANT: Never use timeouts on build commands (10+ hour builds normal)
cargo build                    # Development build
cargo build --release         # Production build
cargo test                    # Run all tests
RUST_LOG=debug cargo test     # With debug logging
./target/release/brainwires   # Binary execution

# Cross-compilation (use scripts)
./scripts/build-release.sh    # Multi-platform builds
./scripts/build-macos.sh      # macOS specific
```

## Key Development Patterns

### Error Handling
```rust
// Consistent Result<T> usage throughout
use anyhow::{Result, Context};
pub type BrainwiresResult<T> = Result<T, crate::types::BrainwiresError>;
```

### Async Patterns
```rust
// Heavy use of tokio::spawn and channels
use tokio::{sync::mpsc, task::JoinHandle};
// Most functions are async, use .await liberally
```

### Configuration Pattern
```rust
// Config in ~/.brainwires/config.json with environment overrides
// Key files: src/config/{mod.rs, manager.rs}
use serde::{Deserialize, Serialize};
```

### Message Storage (Infinite Context)
```rust
// Core pattern: vector embeddings + entity extraction + relationship graph
// Files: src/storage/{conversation_store.rs, tiered_memory.rs}
// Uses FastEmbed (all-MiniLM-L6-v2, 384 dimensions)
```

## Development Conventions

### File Organization
- **Module structure**: Each major component has its own directory under `src/`
- **Error types**: Each module defines its own error types with `thiserror`
- **Tests**: Mirror `src/` structure in `tests/` directory
- **Integration**: Heavy use of `async-trait` for polymorphism

### Naming Conventions
- **Agents**: `orchestrator`, `worker`, `task_agent`
- **Stores**: `conversation_store`, `message_store`, `plan_store`
- **Tools**: Descriptive names like `file_ops`, `semantic_search`
- **Config**: Environment variables prefixed with `BRAINWIRES_`

### Memory Management
```rust
// Key pattern: LRU caches for embeddings, careful Clone usage
use lru::LruCache;
use std::sync::Arc;
// Heavy use of Arc<> for shared state between agents
```

## Critical Integration Points

### MCP Protocol
```bash
# MCP server mode (stdio-based, no files)
brainwires chat --mcp-server
# Test with: echo '{"jsonrpc":"2.0"...}' | ./target/release/brainwires
```

### SEAL Integration Points
```rust
// SEAL is deeply integrated into conversation flow
// Key files: src/seal/{coreference.rs, query_core.rs, learning.rs}
// Always process through SealProcessor before tool execution
```

### RAG System
```bash
# Slash commands for project indexing
/project:index .              # Index codebase
/project:query <search>       # Semantic search
/project:search <query>       # Hybrid search
```

## Testing Strategy

### Test Organization
- **Unit tests**: In each module with `#[cfg(test)]`
- **Integration tests**: `tests/` directory mirroring `src/`
- **MCP tests**: Use stdin/stdout piping, not file-based

### Test Data Patterns
```rust
// Common test setup pattern
use tempfile::tempdir;
use tokio_test;
// Use tempdir() for file system tests
// Mock HTTP with mockito crate
```

### Environment Variables for Tests
```rust
// Key test env vars
RUST_LOG=debug               # Debug logging in tests
BRAINWIRES_TEST_MODE=1       # Test mode flags
```

## Performance Considerations

### Memory Usage
- **Base runtime**: ~10MB
- **With features**: ~40MB
- **Heavy vector operations**: Use streaming where possible

### Build Optimization
```toml
# Release profile in Cargo.toml is heavily optimized
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Concurrency Patterns
```rust
// Use rayon for CPU-bound parallel work
use rayon::prelude::*;
// Use tokio for I/O concurrency
use tokio::task::spawn;
```

## Common Pitfalls & Solutions

### LanceDB Operations
```rust
// Always use transactions for multi-step operations
// Files: src/storage/lance_client.rs patterns
```

### SEAL Learning
```rust
// SEAL learning must be initialized per conversation
// Call seal_processor.init_conversation(id) for each session
```

### Tool Execution
```rust
// Always check permission mode before tool execution
// AccessControlManager is from brainwires::agents (framework crate, re-exported via src/agents/mod.rs)
```

### Cross-Platform Builds
```bash
# Use vendored OpenSSL for consistent builds
# Cross.toml configures cross-compilation environment
```

This is a research codebase - prioritize correctness over simplicity. When implementing new features, follow the established patterns for agent coordination, storage abstraction, and async execution throughout the system.