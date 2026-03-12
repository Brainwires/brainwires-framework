# Changelog

All notable changes to the Brainwires Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-03-12

### Breaking Changes

#### Crate Merges (23 → 19 crates)

| Old Crate | Merged Into | Migration |
|-----------|-------------|-----------|
| `brainwires-brain` | `brainwires-cognition` | `use brainwires_brain::*` → `use brainwires_cognition::knowledge::*` (feature `knowledge`) |
| `brainwires-prompting` | `brainwires-cognition` | `use brainwires_prompting::*` → `use brainwires_cognition::prompting::*` (feature `prompting`) |
| `brainwires-rag` | `brainwires-cognition` | `use brainwires_rag::*` → `use brainwires_cognition::rag::*` (feature `rag`) |
| `brainwires-relay` | `brainwires-agent-network` | `use brainwires_relay::*` → `use brainwires_agent_network::*` (feature `server`) |
| `brainwires-mesh` | `brainwires-agent-network` | `use brainwires_mesh::*` → `use brainwires_agent_network::mesh::*` (feature `mesh`) |
| `brainwires-seal` | `brainwires-agents/seal/` | `use brainwires_seal::*` → `use brainwires_agents::seal::*` (feature `seal`) |

#### Feature Flag Removals
- Removed zero-dependency feature flags that added no conditional compilation value.
- Fixed import paths across all crates affected by feature flag removal.

### Added

#### Cognition (`brainwires-cognition`)
- New unified intelligence crate combining knowledge graphs, adaptive prompting, RAG, spectral math, and code analysis.
- **Knowledge subsystem** (from `brainwires-brain`): `BrainClient`, thought capture, PKS/BKS, entity graphs, semantic memory search.
- **Prompting subsystem** (from `brainwires-prompting`): 15 techniques in 4 categories, task clustering, temperature optimization, learning coordinator.
- **RAG subsystem** (from `brainwires-rag`): `RagClient`, codebase indexing, AST-aware chunking, hybrid vector + BM25 search, git history search, code navigation.
- **Spectral subsystem**: MSS-inspired spectral subset selection for diverse RAG retrieval using log-determinant diversity scoring.
- **Spectral graph operations** (`spectral::graph_ops`): Laplacian construction, Fiedler vector via inverse power iteration, spectral clustering (recursive bisection), algebraic connectivity, effective resistance, Spielman-Srivastava-inspired sparsification, and spectral centrality/bisection — extends spectral methods beyond RAG to general graph analysis.
- **Spectral methods on `RelationshipGraph`**: `spectral_clusters(k)` for semantic community detection within connected components, `spectral_central_nodes(limit)` for structural bridge-node identification, `connectivity()` for graph health monitoring via algebraic connectivity, and `sparsify(epsilon)` for pruning redundant edges while preserving spectral properties. All feature-gated under `spectral`.
- Feature flags: `knowledge` (default), `prompting` (default), `rag`, `spectral`, `code-analysis`, `tree-sitter-languages`, `native` (everything), `wasm`.

#### Agents (`brainwires-agents`)
- **Planner-Worker-Judge cycle orchestration**: Plan→Work→Judge loop for scaling multi-agent coding tasks, inspired by Cursor's planner-worker pipeline pattern. Each cycle: a `PlannerAgent` explores the codebase and creates dynamic tasks, workers execute them via `TaskOrchestrator` with dependency-aware scheduling, and a `JudgeAgent` evaluates results with structured verdicts (Complete, Continue, FreshRestart, Abort).
  - `planner_agent`: LLM-powered dynamic task planner with JSON output parsing, sub-planner recursion, and cycle detection on the task graph.
  - `judge_agent`: LLM-powered cycle evaluator with structured verdict types.
  - `cycle_orchestrator`: Full Plan→Work→Judge loop with fresh `TaskManager` per cycle, configurable `max_cycles`/`max_workers`, and worktree integration prep.
  - New system prompts: `planner_agent_prompt()` and `judge_agent_prompt()`.
  - `spawn_agent_with_context()` on agent pool for per-worker custom `AgentContext`.
  - New communication messages: `CycleStarted`, `CycleCompleted`, `PlanCreated`, `WorkerBranchMerged`.
- **SEAL integration**: Moved `brainwires-seal` into `brainwires-agents/seal/` as a feature-gated module.
  - Feature flags: `seal`, `seal-mdap`, `seal-knowledge`, `seal-feedback`.
  - `SealKnowledgeCoordinator` now integrates with `brainwires-cognition` instead of `brainwires-brain`.
- 4 standalone examples added for agent usage patterns.

#### Agent Network (`brainwires-agent-network`)
- New crate formed by merging `brainwires-relay` (MCP server framework, encrypted IPC, remote bridge) and `brainwires-mesh` (distributed mesh networking).
- Feature flags: `server` (default), `client` (default), `mesh`, `auth-keyring`.

#### Storage (`brainwires-storage`)
- New `vector-db` feature: vector database trait + backends (LanceDB, Qdrant), BM25 keyword search, glob/path utilities — used by `brainwires-cognition` RAG subsystem.
- Removed `agents` feature and `PersistentTaskManager` (decoupled from agents layer).

#### Build & CI
- `xtask ci` command for local CI: runs `cargo fmt --check`, `cargo clippy`, and `cargo test` in a single command via the xtask pattern (`cargo xtask ci`). Added `.cargo/config.toml` alias and updated `CONTRIBUTING.md` with usage instructions.

#### Licensing
- Added Apache 2.0 and MIT license files to all crates for compliance and distribution.

### Changed

#### Framework-wide
- Reduced crate count from 23 to 19 through strategic merges (see Breaking Changes above).
- Updated all cross-crate import paths for merged crates.
- Updated all README files to reflect post-merge crate structure and integrated documentation from dissolved crates.
- Updated workspace dependency tree in `crates/README.md`.

## [0.2.0] - 2026-03-09

### Changed

#### Framework-wide
- Removed hardcoded crate counts from `CONTRIBUTING.md` and `crates/README.md` to avoid staleness.
- Replaced inline crate listing in `CONTRIBUTING.md` with links to `README.md`, `crates/README.md`, and `extras/README.md`.
- Removed extras table from `crates/README.md`; extras are now documented in their own `extras/README.md`.
- Applied `cargo fmt --all` across workspace.

### Added

#### SEAL (`brainwires-seal`)
- **Feedback Bridge** (`feedback_bridge.rs`): New module that wires `AuditLogger` user feedback (thumbs-up/down + corrections) into the SEAL learning loop. `FeedbackBridge` pulls `FeedbackSignal` events on demand and converts them into `LearningCoordinator` outcomes and `PatternHint` entries in global memory.
- New `feedback` feature gate (`dep:brainwires-permissions`, `dep:tokio`) keeps the `AuditLogger` dependency optional.
- 7 unit tests covering per-run processing, recent-feedback queries, correction application, and run isolation.

#### Facade (`brainwires`)
- `learning` convenience feature now includes `permissions` and `brainwires-seal/feedback`, completing the full feedback loop: `AuditLogger → FeedbackBridge → LearningCoordinator → BKS promotion`.

### Changed

#### Framework-wide
- **MSRV bumped from 1.88 to 1.91** — required by updated AWS SDK dependencies (`aws-config`, `aws-sigv4`, `aws-smithy-*`, etc.).
- Updated CI toolchain from Rust 1.88 to 1.91 across all 5 GitHub Actions jobs.
- Added `protoc` installation step to CI (required by `lance-encoding` build dependency).
- Applied `cargo fmt --all` across workspace.

#### Dependencies
- **rmcp** 0.8 → 1.1 (non-exhaustive structs, renamed features/types)
- **tokio-tungstenite** 0.21 → 0.26 (`Message::Text` now wraps `Utf8Bytes`)
- **rand** 0.8 → 0.10 (`thread_rng` → `rng`, `RngCore` → `Rng`, `gen_range` → `random_range`)
- **bincode** 1 → 2 (new serde encode/decode API)
- **serde_yaml** → **serde_yml** 0.0.12 (crate rename)
- **tonic** 0.12 → 0.13, **prost** 0.13 → 0.14 (removed `async_trait` macro)
- **lancedb** 0.23 → 0.26, **arrow** 56 → 57
- **toml** 0.8 → 1.0, **git2** 0.19 → 0.20, **lru** 0.12 → 0.16
- **boa_engine** 0.20 → 0.21, **tokenizers** 0.21 → 0.22, **tiktoken-rs** 0.7 → 0.9

### Fixed
- Fixed invalid crates.io category slug (`science::ml` → `artificial-intelligence`) on `brainwires-training`.
- Updated publish script rate limits for existing-crate version publishes (burst 30, then 1/min).

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
- **Workflow Graph Builder**: Declarative DAG workflows with `WorkflowBuilder`, parallel fan-out/fan-in, conditional routing, shared `WorkflowContext` state, and failure propagation. Topological validation via `petgraph`.
- **Named Reasoning Strategies** (feature-gated `reasoning`): `ReActStrategy`, `ReflexionStrategy`, `ChainOfThoughtStrategy`, `TreeOfThoughtsStrategy` — each with system prompts, completion detection, and step limits. `StrategyPreset` enum for factory creation.
- **OpenTelemetry Export** (feature-gated `otel`): `export_to_otel()` maps `ExecutionGraph` to hierarchical OTel spans (`agent.run` → `agent.iteration.N` → `agent.tool.name`). `telemetry_attributes()` for attaching metrics to existing spans.
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

#### Model Tools (`brainwires-tool-system`)
- File operations (read, write, edit, delete, list)
- Bash command execution
- Git operations
- Web fetch and search
- Code search with semantic queries
- Validation tools (syntax, duplicates, build)
- Tool orchestration engine (feature-gated)
- Smart router for tool selection (feature-gated)
- **OpenAPI Tool Generation** (feature-gated `openapi`): `openapi_to_tools()` parses OpenAPI 3.x JSON/YAML specs into `Tool` definitions. `execute_openapi_tool()` handles path/query param substitution and Bearer/API-key/Basic auth.

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

### Refactored
- Renamed `brainwires-model-tools` to `brainwires-tool-system` to better reflect the crate's scope (registry, execution, built-in implementations, error taxonomy, sanitization, orchestration, code execution, semantic search, OpenAPI generation, smart routing).

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

[0.3.0]: https://github.com/Brainwires/brainwires-framework/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Brainwires/brainwires-framework/releases/tag/v0.2.0
[0.1.0]: Untagged initial release
