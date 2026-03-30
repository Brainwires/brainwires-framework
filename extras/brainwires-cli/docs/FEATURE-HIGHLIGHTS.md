# Brainwires CLI Summary (v0.5.0)

**Core**: Rust-based AI agent CLI/TUI (~55,700 LOC, 237 files, 12 language AST support)

## Key Systems

- **Orchestrator Agent**: Multi-step task execution, max 25 iterations, tool coordination with approval workflows
- **SEAL (Self-Evolving Agentic Learning)**: Coreference resolution, semantic query extraction, pattern learning without model retraining
- **MDAP (Massively Decomposed Agentic Processes)**: Zero-error execution via voting consensus (first-to-ahead-by-k), red-flagging invalid outputs
- **Multi-Agent Coordination**: Saga transactions (rollback), Contract-Net bidding, optimistic concurrency, git worktree isolation

## Context & Storage

- **Infinite Context**: Tiered memory (hot/warm/cold), semantic search via LanceDB (384-dim embeddings)
- **RAG**: Hybrid search (vector + BM25), AST-aware chunking, 20-30ms latency
- **Storage**: LanceDB (vectors), SQLite (locking), Tantivy (full-text)

## Tools & Interface

- **20+ Tools**: File ops, bash, git, code exec (Python/JS/Rhai/Lua), web search, semantic search
- **Modes**: Interactive, TUI, single-shot, batch, MCP server
- **Permissions**: Auto, interactive, deny

## Platforms

Linux (x86_64/aarch64/armv7), macOS (Universal), Windows (x86_64)
