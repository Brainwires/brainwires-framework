# Changelog

All notable changes to the Brainwires Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

#### RAG (`brainwires-rag`)
- Git search results now return the actual commit date instead of hardcoded `0`. The `commit_date` field in `GitSearchResult` is now populated from the `indexed_at` metadata stored in the vector database during indexing.
- Dirty flag is now cleared immediately after embeddings + cache are flushed to disk in both full and incremental indexing paths. Previously, the dirty flag was only cleared in the outer `do_index_smart` wrapper, so an unclean exit after successful indexing could leave the flag stuck, causing unnecessary full reindexes on next startup.

### Added

#### RAG (`brainwires-rag`)
- `indexed_at` field on `SearchResult` — exposes the chunk indexing timestamp (Unix epoch seconds) from the vector database. Defaults to `0` for backwards compatibility.

## [0.1.0] - 2025-03-06

### Added

#### Core (`brainwires-core`)
- `Provider` trait with streaming support (`stream_chat`) and `ChatOptions` builder
- `Message`, `Role`, `ContentBlock`, `ChatResponse`, `StreamChunk` types
- `Tool`, `ToolUse`, `ToolResult`, `ToolRegistry` for tool definitions
- `EmbeddingProvider` trait with batch support
- `VectorStore` trait (backend-agnostic vector database interface)
- `Task`, `WorkingSet`, `PlanMetadata` types
- `FrameworkError` hierarchy with `thiserror`
- Graph types: `GraphNode`, `GraphEdge`, `EntityType`, `EdgeType`

#### Providers (`brainwires-providers`)
- Anthropic, OpenAI, Google (Gemini), Ollama provider implementations
- Groq, Together, Fireworks, Anyscale via OpenAI-compatible protocol
- `ChatProviderFactory` for dynamic provider creation from config
- Rate limiting, model listing, streaming responses
- Optional local LLM support via `llama-cpp-2` feature
- Optional Bedrock and Vertex AI authentication
- Ollama multimodal image support (base64 extraction from `ContentBlock::Image`)

#### Agents (`brainwires-agents`)
- `AgentRuntime` with communication hub and file lock coordination
- `TaskManager` and `TaskQueue` for agent task lifecycle
- `ValidationConfig` with file existence, syntax, duplicate, and build checks
- `AccessControlManager` with contention strategies
- `GitCoordinator` for multi-agent git operations
- `PlanExecutorAgent` for structured plan execution
- Extended reasoning support (feature-gated)
- Evaluation framework for benchmarking (feature-gated)

#### MDAP (`brainwires-mdap`)
- Multi-Dimensional Adaptive Planning with k-agent voting
- `Composer` for aggregating multi-agent results
- `FirstToAheadByKVoter` voting strategy
- Red flag validation and microagent configuration
- Recursive task decomposition

#### Brain (`brainwires-brain`)
- Personal Knowledge Store (PKS) and Behavioral Knowledge Store (BKS)
- Entity extraction and relationship graphs
- Persistent thought storage
- Knowledge integration with prompting system

#### Storage (`brainwires-storage`)
- LanceDB-backed tiered memory (hot/warm/cold)
- Semantic search across conversation history
- Lock store for concurrent access

#### Prompting (`brainwires-prompting`)
- `PromptGenerator` with technique library
- `TemperatureOptimizer` for adaptive temperature selection
- `TaskClusterManager` for grouping similar tasks
- Knowledge-aware prompt construction (feature-gated)

#### Permissions (`brainwires-permissions`)
- `PolicyEngine` with capability profiles
- `TrustManager` with trust levels and escalation
- `AuditLogger` for security audit trails
- Anomaly detection for unusual tool usage

#### Model Tools (`brainwires-model-tools`)
- File operations (read, write, edit, delete, list)
- Bash command execution
- Git operations
- Web fetch and search
- Code search with semantic queries
- Validation tools (syntax, duplicates, build)
- Tool orchestration engine (feature-gated)
- Smart router for tool selection (feature-gated)

#### MCP (`brainwires-mcp`)
- MCP client for connecting to external MCP servers
- `McpConfigManager` for server configuration

#### Relay (`brainwires-relay`)
- MCP server mode for exposing agents as tools
- IPC and remote relay for cross-process communication
- Agent-to-Agent (A2A) protocol support (feature-gated)
- Heartbeat monitoring and attachment transfer

#### RAG (`brainwires-rag`)
- AST-aware code chunking with tree-sitter
- Hybrid vector + BM25 keyword search
- Git-aware indexing with blame and history
- LanceDB and Qdrant vector backends
- Relation extraction and storage
- MCP server integration

#### Skills (`brainwires-skills`)
- Pluggable skill definitions
- Slash command registry

#### Code Interpreters (`brainwires-code-interpreters`)
- Sandboxed JavaScript execution (Rhai)
- Sandboxed Lua execution
- Python and additional language support (feature-gated)

#### WASM (`brainwires-wasm`)
- Browser-compatible WASM bindings for core agent functionality

#### SEAL (`brainwires-seal`)
- Self-Evolving Agentic Learning system
- Feedback-driven prompt improvement
- Coreference resolution and query analysis
- Knowledge integration (feature-gated)
- Structured `PatternHint` for BKS-to-SEAL pattern transfer
- `QueryCore::resolved` field for tracking coreference-resolved queries
- Execution timing propagation through `record_outcome`

#### Mesh (`brainwires-mesh`)
- Distributed agent mesh networking
- Topology management (star, ring, full mesh)
- Message routing with configurable strategies
- Peer discovery protocols
- Federation gateways for cross-mesh communication

#### Audio (`brainwires-audio`)
- Hardware audio capture and playback (CPAL)
- Speech-to-text and text-to-speech traits
- FLAC encoding/decoding support
- Local STT support (feature-gated)
- Unit tests for types, device, and error modules

#### Datasets (`brainwires-datasets`)
- JSONL I/O for training data
- Tokenization (HuggingFace tokenizers, tiktoken)
- Deduplication pipelines
- Format conversion between training formats

#### Training (`brainwires-training`)
- Cloud fine-tuning for 6 providers (OpenAI, Anthropic, Google, Together, Fireworks, Anyscale)
- Local LoRA/QLoRA/DoRA training via Burn
- Training job management and monitoring

#### Autonomy (`brainwires-autonomy`)
- Self-improvement strategies
- Evaluation-driven optimization
- Supervisor agent patterns
- Attention mechanisms for context prioritization
- Unit tests for config, error, metrics, attention, health, parallel, training loop, forge, branch manager, investigator, and strategies

#### Facade (`brainwires`)
- Unified re-exports of all 22 sub-crates via feature flags
- `prelude` module with commonly needed types
- Convenience feature bundles: `full`, `researcher`, `agent-full`, `learning`

### Changed
- Upgraded `#![warn(missing_docs)]` to `#![deny(missing_docs)]` across all 22 crates
- Added doc comments to all previously undocumented public items (~155 warnings resolved)

[0.1.0]: https://github.com/Brainwires/brainwires-framework/releases/tag/v0.1.0
