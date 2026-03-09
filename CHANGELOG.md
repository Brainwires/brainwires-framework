# Changelog

All notable changes to the Brainwires Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-09

### Added

#### A2A (`brainwires-a2a`)
- New crate: full Agent-to-Agent protocol — JSON-RPC 2.0, HTTP/REST, and gRPC bindings.
- `A2aClient` with unified transport selection, `A2aServer` with `A2aHandler` trait.
- AgentCard discovery at `/.well-known/agent-card.json`, SSE streaming, push notification CRUD.
- gRPC support via tonic-build from official `a2a.proto` with full type conversions.
- 71 tests covering serde roundtrips, SSE parsing, streaming, HTTP integration.

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
- **OpenAI Responses API**: Full-spec coverage — all 7 tool types, 11 output item types, 35+ streaming event types, structured outputs, reasoning config, and all 6 REST endpoints.
- `ProviderType::OpenAiResponses` with registry entry, factory integration, model listing support, and `base_url` passthrough.
- Response ID tracking for automatic conversation chaining.

#### Agents (`brainwires-agents`)
- `AgentRuntime` with communication hub and file lock coordination
- `TaskManager` and `TaskQueue` for agent task lifecycle
- `ValidationConfig` with file existence, syntax, duplicate, and build checks
- `AccessControlManager` with contention strategies
- `GitCoordinator` for multi-agent git operations
- `PlanExecutorAgent` for structured plan execution
- Extended reasoning support (feature-gated)
- Evaluation framework for benchmarking (feature-gated)
- `AgentLifecycleHooks` trait with 10 hook points: before/after iteration, provider call, tool execution, completion, and context pressure.
- `ToolDecision::Delegate` for sub-agent spawning, `ConversationView` for history manipulation, `DefaultDelegationHandler` wrapping `AgentPool`.
- `#[non_exhaustive]` on `AgentContext` and `TaskAgentConfig`.

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
- `indexed_at` field on `SearchResult` — exposes the chunk indexing timestamp (Unix epoch seconds) from the vector database.
- Upgraded `zip` dependency from v2 to v8 (pure-Rust `lzma-rust2`).

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
- **BPE tokenizer integration**: `Tokenizer` trait with `ModelTokenizer` (HuggingFace `tokenizers` crate) and `SimpleTokenizer` (byte-level fallback). New `tokenizer_path` config option on `LocalTrainingConfig`.
- **SafeTensors model weight loading**: `weight_loader.rs` with `SafeTensorsLoader` for loading pre-trained base weights (f32/f16/bf16 dtype conversion). `LoraLinearConfig::init_with_base_weights()` and `DoraLinearConfig::init_with_base_weights()`.
- **QLoRA quantized base weight loading**: `QLoraLinear` and `QLoraLinearConfig` Burn modules with `init_quantized()` for INT4/INT8 dequantized base weights. Full training loop in `train_qlora()`.
- **DPO/ORPO alignment training**: `PreferenceExample` and `PreferenceDataset` (JSONL: `{"prompt", "chosen", "rejected"}`). `train_dpo_alignment()` with frozen reference model and `train_orpo_alignment()` with single-pass odds ratio loss.
- `TrainingError::NotImplemented` variant for clear stub errors on unimplemented provider methods.
- Dataset loading: JSONL parser supporting prompt/completion and chat message formats (`dataset_loader.rs`).
- Learning rate scheduling: warmup phase + constant/linear/cosine/cosine-warm-restarts strategies (`lr_schedule.rs`).
- Multi-adapter dispatch: LoRA and DoRA training paths with QLoRA/QDoRA fallbacks.
- Validation loop: optional eval dataset evaluated each epoch during local training.
- Weight serialization: adapter weights (A, B, magnitude) written as binary for export.
- Token count tracking in training metrics.
- Weight accessor methods on `LoraLinear` and `DoraLinear` for export.

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

#### Agents (`brainwires-agents`)
- Replaced `panic!()`/`unwrap()` in eval suite with graceful `TrialResult::failure` conversions.
- Implemented `TextMerge` (line-by-line dedup) and `JsonMerge` (recursive deep merge) optimistic concurrency strategies.
- Replaced silent `let _ =` broadcast/send drops with `tracing::warn` logging across contract_net, task_orchestrator, and validator_agent.

#### Providers (`brainwires-providers`)
- Refactored monolithic `openai_responses/mod.rs` into structured modules (`client.rs`, `convert.rs`, `provider.rs`, `types/`).
- 54 new tests covering serde round-trips for all wire types.

#### Training (`brainwires-training`)
- Upgraded Burn from 0.16 to 0.20. Switched from umbrella `burn` crate to individual crates (`burn-core`, `burn-nn`, `burn-optim`, `burn-autodiff`, `burn-wgpu`, `burn-ndarray`) to avoid `cubecl-cpu` links="lzma" conflict with `xz2` from datafusion/lancedb.
- Fixed `squeeze`/`unsqueeze` API calls for Burn 0.19+ compatibility.
- Added `extern crate burn_core as burn` shim for derive macro resolution.
- Cloud providers (Together, Fireworks, Anyscale): extracted `extract_error()` and `parse_job_status()` helpers; `list_jobs()` now parses actual job status instead of hardcoding `Pending`.
- Cloud providers (Bedrock, Vertex): all methods now return explicit `TrainingError::NotImplemented` errors instead of ad-hoc strings.

#### Framework-wide
- Production-readiness audit across 15 crates (40 files): replaced 121 `unwrap()` calls with `context()`/`expect()`/`LazyLock`; fixed 10 clippy warnings; removed 3 deprecated zero-caller functions; removed 3 dead code items; resolved 2 TODO comments.

### Fixed

#### A2A (`brainwires-a2a`)
- Capped SSE stream buffers at 16MB to prevent unbounded memory growth.
- Added bearer token auth on all transports.
- Fixed gRPC error code mapping, mutex for streaming, and bind error propagation.
- Added CORS headers, resilient accept loop, and graceful shutdown.
- Incremental SSE parser with multi-line data support.

#### Audio (`brainwires-audio`)
- Proper error handling for non-UTF-8 model paths in `WhisperStt`.

#### RAG (`brainwires-rag`)
- Fixed use-after-move of `symbol_name` in `find_references`.
- Git search results now return the actual commit date instead of hardcoded `0`.
- Dirty flag is now cleared immediately after embeddings + cache are flushed to disk in both full and incremental indexing paths.

[0.1.0]: https://github.com/Brainwires/brainwires-framework/releases/tag/v0.1.0
