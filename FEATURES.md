# Brainwires Framework — Complete Feature List

A comprehensive catalog of every feature provided by the framework's 19 crates and 9 extras.

---

## Table of Contents

- [Core Types & Traits](#core-types--traits)
- [AI Providers](#ai-providers)
- [Agent Orchestration](#agent-orchestration)
- [Tool System](#tool-system)
- [MCP Protocol](#mcp-protocol)
- [Agent Networking](#agent-networking)
- [MDAP Voting](#mdap-voting)
- [Storage & Memory](#storage--memory)
- [RAG & Code Search](#rag--code-search)
- [Knowledge & Brain](#knowledge--brain)
- [Adaptive Prompting](#adaptive-prompting)
- [SEAL (Self-Evolving Agentic Learning)](#seal-self-evolving-agentic-learning)
- [Permissions & Security](#permissions--security)
- [Audio](#audio)
- [Code Interpreters](#code-interpreters)
- [Skills System](#skills-system)
- [Channels](#channels)
- [Datasets & Training Data](#datasets--training-data)
- [Model Training & Fine-Tuning](#model-training--fine-tuning)
- [Distributed Mesh Networking](#distributed-mesh-networking)
- [Agent-to-Agent (A2A) Protocol](#agent-to-agent-a2a-protocol)
- [Autonomous Operations](#autonomous-operations)
- [Reasoning & Inference](#reasoning--inference)
- [Evaluation Framework](#evaluation-framework)
- [Proxy Framework](#proxy-framework)
- [WASM Bindings](#wasm-bindings)
- [Extras & Standalone Binaries](#extras--standalone-binaries)
- [Facade Crate & Feature Flags](#facade-crate--feature-flags)

---

## Core Types & Traits

**Crate:** `brainwires-core`

Foundation types shared by all framework crates.

- **Message system** — `Message`, `Role`, `ContentBlock`, `ImageSource`, `MessageContent`, streaming `StreamChunk`, `ChatResponse`, `Usage` tracking
- **Stateless history** — `serialize_messages_to_stateless_history()` for API-ready conversation formatting
- **Tool definitions** — `Tool`, `ToolInputSchema`, `ToolResult`, `ToolUse`, `ToolCaller`, `ToolContext`, `ToolMode`
- **Idempotency** — `IdempotencyRecord`, `IdempotencyRegistry` for deduplicating tool calls
- **Staged writes** — `StagedWrite`, `StagingBackend`, `CommitResult` for transactional file operations
- **Task system** — `Task`, `TaskStatus`, `TaskPriority`, `AgentResponse`
- **Plan system** — `PlanMetadata`, `PlanStatus`, step budgets, serializable plans
- **Plan parsing** — `parse_plan_steps()`, `steps_to_tasks()`, structured output parsers (`JsonOutputParser`, `RegexOutputParser`) (feature: `planning`)
- **Provider trait** — `Provider` async trait, `ChatOptions` (temperature, max tokens, top-p, stop sequences)
- **Permission modes** — `PermissionMode` (auto, ask, reject)
- **Knowledge graph types** — `EntityType`, `EdgeType`, `GraphNode`, `GraphEdge`, `EntityStoreT`, `RelationshipGraphT` traits
- **Embedding trait** — `EmbeddingProvider` for pluggable embedding backends
- **Vector store trait** — `VectorStore`, `VectorSearchResult` for similarity search abstraction
- **Working set** — `WorkingSet` with LRU eviction, `WorkingSetConfig`, token estimation utilities
- **Content source** — `ContentSource` for tracking where content originates
- **Lifecycle hooks** — Interceptors for framework events
- **Error handling** — `FrameworkError`, `FrameworkResult`
- **WASM support** — `wasm` feature flag for browser-compatible builds

---

## AI Providers

**Crate:** `brainwires-providers`

Unified multi-provider AI interface with 18 provider types.

### Chat Providers

| Provider | Protocol | Auth |
|----------|----------|------|
| **Anthropic** (Claude) | Anthropic Messages | `x-api-key` header |
| **OpenAI** (GPT) | Chat Completions | Bearer token |
| **OpenAI Responses** | Responses API (`/v1/responses`) | Bearer token |
| **Google** (Gemini) | `generateContent` | Bearer token |
| **Ollama** | Native chat (`/api/chat`) | None (local) |
| **Groq** | OpenAI-compatible | Bearer token |
| **Together AI** | OpenAI-compatible | Bearer token |
| **Fireworks AI** | OpenAI-compatible | Bearer token |
| **Anyscale** | OpenAI-compatible | Bearer token |
| **Amazon Bedrock** | Anthropic Messages via AWS SigV4 | AWS SigV4 signing |
| **Google Vertex AI** | Anthropic Messages via OAuth2 | Google OAuth |
| **Brainwires HTTP** | Custom relay protocol | Bearer token |
| **Custom** | User-defined | Configurable |

### Audio API Clients

| Client | Capabilities |
|--------|-------------|
| **ElevenLabs** | TTS + STT |
| **Deepgram** | TTS + STT |
| **Google Cloud TTS** | TTS |
| **Azure Speech** | TTS + STT |
| **Fish Audio** | TTS + ASR |
| **Cartesia** | TTS |
| **Murf AI** | TTS |

### Infrastructure

- **ChatProviderFactory** — Registry-driven protocol dispatch, creates providers from `ProviderConfig`
- **Provider registry** — Static metadata (protocol, auth scheme, endpoint, model listing URL) for all providers
- **RateLimitedClient** — HTTP client with built-in rate limiting
- **RateLimiter** — Token-bucket rate limiter
- **Model listing** — `ModelLister`, `AvailableModel`, `ModelCapability` for querying available models
- **Local LLM** — `llama-cpp-2` integration for local inference (feature: `llama-cpp-2`)
- **Streaming** — All providers return async streams for real-time output

---

## Agent Orchestration

**Crate:** `brainwires-agents`

Multi-agent infrastructure for autonomous task execution.

### Agent Types

- **TaskAgent** — Autonomous agent executing tasks with tool access, configurable iteration limits, validation loops
- **ValidatorAgent** — Rule-based validation agent for quality checks
- **PlanExecutorAgent** — Executes multi-step plans with approval modes (auto, manual, checkpoint)
- **TaskOrchestrator** — Hierarchical task decomposition with failure policies (fail-fast, continue, retry)

### Workflow Graph Builder

- **WorkflowBuilder** — Declarative DAG-based workflow pipelines with `node()`, `edge()`, `conditional()`, and `build()`
- **WorkflowContext** — Shared state map accessible to all workflow nodes during execution
- **WorkflowResult** — Collected per-node results after execution
- **Parallel fan-out / fan-in** — Nodes with shared predecessors run concurrently via `tokio::spawn`
- **Conditional routing** — Skip downstream branches based on runtime conditions
- **Cycle detection** — Compile-time validation via `petgraph::algo::is_cyclic_directed`

### Runtime & Lifecycle

- **AgentRuntime** — Core agent execution loop with `run_agent_loop()`
- **AgentPool** — Concurrent agent management with lifecycle tracking and pool statistics
- **AgentContext** — Working directory, tool registry, capabilities per agent
- **ExecutionGraph** — Step-by-step telemetry recording (`StepNode`, `ToolCallRecord`, `RunTelemetry`)

### Communication

- **CommunicationHub** — Inter-agent messaging bus with 50+ message types
- **AgentMessage** — Typed messages: `StatusUpdate`, `HelpRequest`, `TaskResult`, `ToolRequest`, conflict info
- **ConflictInfo** — Git operation conflict detection and reporting

### Coordination Patterns

- **ContractNet** — Bidding protocol for agent task negotiation
- **SagaExecutor** — Compensating transactions for distributed operations with rollback
- **OptimisticController** — Optimistic locking with version-based conflict detection
- **MarketAllocator** — Market-based task allocation
- **WaitQueue** — Queue-based coordination primitives
- **ThreeStateModel** — State snapshots for rollback support (`StateSnapshot`, proposed operations)

### File & Resource Coordination

- **FileLockManager** — File-level read/write locks with deadlock prevention
- **ResourceLockManager** — Scoped resource locking with heartbeat-based liveness
- **AccessControlManager** — Advanced access control with contention strategies and lock persistence
- **OperationTracker** — Operation tracking with heartbeat-based liveness checking

### Task Management

- **TaskManager** — Hierarchical task decomposition and dependency tracking
- **TaskQueue** — Priority-based scheduling with dependency awareness

### Git Coordination

- **GitCoordinator** — Git operation locking with `GitLockRequirements`
- **GitOperationRunner** — Safe concurrent git operations
- **WorktreeManager** — Git worktree management for agent isolation (feature: `native`)

### OpenTelemetry Export (feature: `otel`)

- **export_to_otel()** — Maps `ExecutionGraph` and `RunTelemetry` to hierarchical OpenTelemetry spans
- **Span hierarchy** — Root `agent.run` → `agent.iteration.{N}` → `agent.tool.{name}`
- **Attributes** — Token counts, costs, timing, and error information attached as span attributes
- **Compatible** — Works with Jaeger, Datadog, Grafana, and any OpenTelemetry-compatible backend

### Validation

- **ValidationLoop** — Quality checks before agent completion
- **ValidationConfig** — Configurable checks: file existence, duplicate detection, syntax, build verification
- **ResourceChecker** — Conflict detection and resolution
- **Confidence scoring** — `extract_confidence()`, `quick_confidence_check()`, `ResponseConfidence`

---

## Tool System

**Crate:** `brainwires-tool-system`

Composable tool implementations for agent use.

### Built-in Tools (always available)

- **BashTool** — Shell command execution with proactive output management
- **FileOpsTool** — Read, write, edit, patch, list, search, delete, create directory
- **GitTool** — Status, diff, log, stage, commit, push, pull, branch, checkout, and more
- **WebTool** — URL fetching
- **SearchTool** — Regex-based code search (respects `.gitignore`)
- **ValidationTool** — Code quality checks (duplicate detection, build verification, syntax checking)
- **ToolSearchTool** — Meta-tool for dynamic tool discovery at runtime

### Tool Infrastructure

- **ToolRegistry** — Composable container with `with_builtins()` for all tools, category-based organization
- **ToolExecutor** — Permission checking, lock acquisition, working set tracking, error handling
- **ToolPreHook** — Pre-execution hooks with `PreHookDecision` (allow/deny/modify)
- **TransactionManager** — Transactional file operations with commit/rollback (feature: `native`)
- **Error taxonomy** — `classify_error()`, `ToolErrorCategory`, `RetryStrategy`, `ToolOutcome`
- **Sanitization** — `contains_sensitive_data()`, `is_injection_attempt()`, `redact_sensitive_data()`, content source wrapping

### Feature-Gated Tools

- **OrchestratorTool** — Rhai script orchestration (feature: `orchestrator`)
- **CodeExecTool** — Sandboxed multi-language code execution (feature: `interpreters`)
- **SemanticSearchTool** — RAG-powered semantic codebase search (feature: `rag`)

### OpenAPI Tool Generation (feature: `openapi`)

- **openapi_to_tools()** — Parse OpenAPI 3.x specs (JSON or YAML) and generate `Tool` definitions
- **OpenApiTool** — Pairs a `Tool` definition with its `OpenApiEndpoint` metadata (method, path, parameters, base URL)
- **execute_openapi_tool()** — Execute an OpenAPI-generated tool call against the live API
- **OpenApiAuth** — Authentication support: `Bearer`, `ApiKey` (header/query), `Basic`
- **OpenApiParam** — Parameter extraction from path, query, header, and request body
- **HttpMethod** — GET, POST, PUT, PATCH, DELETE support

---

## MCP Protocol

**Crate:** `brainwires-mcp`

MCP client for connecting to external MCP servers.

- **McpClient** — Connect, list/call tools, read resources, get prompts
- **StdioTransport** — Stdio-based transport layer
- **McpConfigManager** — Server configuration management
- **JSON-RPC 2.0** — Full request/response/notification/error types
- **MCP types** — `McpTool`, `McpResource`, `McpPrompt`, capabilities, initialization
- **Progress tracking** — `ProgressParams`, `McpNotification`
- **Resource reading** — `ReadResourceParams`, `ResourceContent`
- **Prompt system** — `GetPromptParams`, `PromptMessage`, `PromptArgument`

---

## Agent Networking

**Crate:** `brainwires-agent-network`

MCP server framework, middleware pipeline, agent IPC, remote bridge, and optional mesh networking.

### MCP Server Framework

- **McpServer** — Full MCP server lifecycle management
- **McpHandler** — Request handler trait
- **McpToolRegistry** — Tool registration with `McpToolDef` and `ToolHandler`
- **ServerTransport** — Stdio server transport
- **Middleware pipeline:**
  - `AuthMiddleware` — Authentication
  - `LoggingMiddleware` — Request/response logging
  - `RateLimitMiddleware` — Rate limiting
  - `ToolFilterMiddleware` — Tool access filtering

### Agent Communication

- **IPC** — Inter-process communication socket protocol
- **Remote relay** — Bridge and realtime protocol for remote agent communication
- **Auth** — Authentication for relay connections

### Agent Management

- **AgentManager** — Agent lifecycle management (`AgentInfo`, `AgentResult`, `SpawnConfig`)
- **AgentToolRegistry** — Pre-built MCP tools for agent operations (spawn, list, status, stop, await)

### Relay Client

- **AgentNetworkClient** — Connect to remote agent network servers (feature: `client`)

---

## MDAP Voting

**Crate:** `brainwires-agents` (feature `mdap`)

Multi-Dimensional Adaptive Planning implementing the MAKER paper.

- **FirstToAheadByKVoter** — Consensus algorithm where k agents vote for error correction
- **Microagent system** — Minimal-context single-step agents (m=1 decomposition), `MicroagentConfig`, `MicroagentProvider`
- **Task decomposition:**
  - `SequentialDecomposer` — Linear step-by-step decomposition
  - `AtomicDecomposer` — Single-step atomic tasks
  - `BinaryRecursiveDecomposer` — Divide-and-conquer splitting
  - `SimpleRecursiveDecomposer` — Simple recursive breakdown
- **Red flag validation** — `StandardRedFlagValidator`, `RedFlagConfig`, output format checking
- **Cost estimation** — `estimate_mdap()`, `ModelCosts`, probability optimization
- **Metrics** — `MdapMetrics` for execution tracking and reporting
- **Composer** — `StandardComposer`, `CompositionBuilder` for assembling subtask outputs
- **Tool intent** — `ToolIntent`, `ToolSchema`, `ToolCategory` for stateless execution

---

## Storage & Memory

**Crate:** `brainwires-storage`

LanceDB-backed persistent storage with semantic search.

### Vector Database

- **LanceClient** — LanceDB connection and table management
- **FastEmbedManager** — Text embeddings via FastEmbed ONNX model (all-MiniLM-L6-v2)
- **CachedEmbeddingProvider** — LRU-cached embedding provider

### Data Stores

- **MessageStore** — Conversation messages with vector search
- **ConversationStore** — Conversation metadata
- **TaskStore** — Task persistence with agent state tracking (`AgentStateStore`)
- **PlanStore** — Execution plan storage with markdown export
- **TemplateStore** — Reusable plan template storage
- **LockStore** — Cross-process lock coordination with statistics
- **ImageStore** — Image analysis storage with semantic search

### Tiered Memory

- **TieredMemory** — Three-tier memory hierarchy:
  - **Hot** — Recent messages, full fidelity
  - **Warm** — `SummaryStore` with compressed message summaries
  - **Cold** — `FactStore` with extracted key facts
- **TierMetadataStore** — Tier tracking metadata
- **MemoryAuthority** — Canonical write tokens (`CanonicalWriteToken`)
- **MultiFactorScore** — Multi-factor relevance scoring for search

### File Context

- **FileContextManager** — File content management with chunking (`FileChunk`, `FileContent`)

### Agent Integration

- **PersistentTaskManager** — Persistent task management bridging storage and agents (feature: `agents`)

---

## RAG & Code Search

**Crate:** `brainwires-cognition` (feature: `rag`)

RAG-based codebase indexing and semantic search.

- **RagClient** — Core library API combining all functionality
- **Hybrid search** — Vector similarity (FastEmbed) + BM25 keyword matching (Tantivy) with Reciprocal Rank Fusion
- **Dual database support** — LanceDB (embedded, default) or Qdrant (external server)
- **Smart indexing** — Auto-detects full vs incremental updates with persistent file hash caching
- **AST-based chunking** — Tree-sitter parsing for 12 programming languages (feature: `tree-sitter-languages`)
- **Git history search** — Semantic search over commit history with on-demand indexing
- **Code relations** — Definition finding, reference finding, call graph extraction
- **Document processing** — PDF, markdown, etc. (feature: `documents`)
- **Multi-project support** — Project-scoped indexing and querying
- **Configuration** — Environment variable support, customizable chunk sizes and thresholds

---

## Knowledge & Brain

**Crate:** `brainwires-cognition` (feature: `knowledge`)

Central knowledge crate for persistent thought storage and entity graphs.

- **BrainClient** — Persistent thought storage with semantic search
- **Thought system** — `Thought`, `ThoughtCategory`, `ThoughtSource` with full CRUD operations
- **Knowledge systems:**
  - **BKS** (Behavioral Knowledge Store) — Behavioral truths and patterns
  - **PKS** (Personal Knowledge Store) — Personal facts and preferences
- **Entity graph:**
  - **EntityStore** — Entity types, extraction results, contradiction detection (`ContradictionEvent`, `ContradictionKind`)
  - **RelationshipGraph** — Edge types, entity context, graph traversal
- **Fact extraction** — Automatic categorization and tag extraction from text
- **MCP tool types** — Request/response types for search, capture, delete, list, and memory stats

---

## Adaptive Prompting

**Crate:** `brainwires-cognition` (feature: `prompting`)

Implements "Adaptive Selection of Prompting Techniques" (arXiv:2510.18162).

- **15 prompting techniques** — Chain-of-thought, few-shot, zero-shot, and more with `TechniqueCategory` and `ComplexityLevel`
- **Task clustering** — K-means clustering by semantic similarity with `TaskClusterManager`
- **Technique library** — Metadata with BKS integration for technique selection
- **Prompt generator** — Dynamic multi-source prompt generation with `GeneratedPrompt`
- **Learning coordinator** — Technique effectiveness tracking, BKS promotion, cluster summaries
- **Temperature optimization** — Adaptive temperature per cluster with performance tracking
- **Storage** — SQLite persistence for clusters and performance data (feature: `native`)
- **SEAL integration** — `SealProcessingResult` for connecting with SEAL pipeline

---

## SEAL (Self-Evolving Agentic Learning)

**Crate:** `brainwires-agents` (feature: `seal`)

Self-evolving agent capabilities without retraining.

- **SealProcessor** — Main pipeline orchestrating all components
- **Coreference resolution** — Resolves pronouns and elliptical references ("it", "the file", "that function") to concrete entities from dialog history
- **Query core extraction** — Structured S-expression-like query cores (`QueryCore`, `QueryOp`, `QueryExpr`) for graph traversal
- **Self-evolving learning** — `LearningCoordinator` with `GlobalMemory` and `LocalMemory`, pattern matching and reliability tracking
- **Reflection module** — Post-execution analysis, error correction, quality scoring with `ReflectionReport`
- **Knowledge integration** — Entity resolution strategies, SEAL-Brain coordinator (feature: `knowledge`)
- **MDAP integration** — Record MDAP execution metrics for learning (feature: `mdap`)

---

## Permissions & Security

**Crate:** `brainwires-permissions`

Capability-based permission system.

### Capabilities

- **AgentCapabilities** — Granular control over:
  - `FilesystemCapabilities` — Path patterns, read/write/execute
  - `ToolCapabilities` — Tool categories, allow/deny lists
  - `NetworkCapabilities` — Domain restrictions, protocols
  - `GitCapabilities` — Operation-level control (clone, push, force-push)
  - `SpawningCapabilities` — Agent spawning limits
  - `ResourceQuotas` — CPU, memory, disk limits

### Profiles

- Pre-defined capability sets: `read_only`, `standard_dev`, `full_access`

### Policy Engine

- **PolicyEngine** — Rule-based enforcement with conditions and actions
- **EnforcementMode** — Strict, permissive, audit-only
- **PolicyCondition** / **PolicyAction** / **PolicyDecision**

### Audit & Trust

- **AuditLogger** — Event logging with querying and statistics
- **AuditEvent** — Typed events with outcomes and feedback signals
- **TrustManager** — Trust levels, violation tracking, trust factor management
- **AnomalyDetector** — Anomaly detection with configurable thresholds

### Approval System

- **ApprovalRequest** / **ApprovalResponse** — Severity-based approval workflow

---

## Audio

**Crate:** `brainwires-audio`

Audio capture, playback, speech-to-text, and text-to-speech.

### Core

- **AudioCapture** trait — Audio input abstraction
- **AudioPlayback** trait — Audio output abstraction
- **SpeechToText** trait — STT abstraction
- **TextToSpeech** trait — TTS abstraction
- **AudioRingBuffer** — Ring buffer for streaming audio data
- **WAV utilities** — `encode_wav()`, `decode_wav()`
- **Device enumeration** — `AudioDevice`, `DeviceDirection`

### Hardware Backends (feature: `native`)

- **CpalCapture** — Hardware audio capture via cpal
- **CpalPlayback** — Hardware audio playback via cpal

### Cloud API Integrations (feature: `native`)

| Implementation | Type | Provider |
|---------------|------|----------|
| `OpenAiTts` | TTS | OpenAI |
| `OpenAiStt` | STT | OpenAI |
| `ElevenLabsTts` | TTS | ElevenLabs |
| `ElevenLabsStt` | STT | ElevenLabs |
| `DeepgramTts` | TTS | Deepgram |
| `DeepgramStt` | STT | Deepgram |
| `GoogleTts` | TTS | Google Cloud |
| `AzureTts` | TTS | Azure |
| `AzureStt` | STT | Azure |
| `FishTts` | TTS | Fish Audio |
| `FishStt` | STT | Fish Audio |
| `CartesiaTts` | TTS | Cartesia |
| `MurfTts` | TTS | Murf AI |

### Local Inference

- **WhisperStt** — Local STT via whisper.cpp (feature: `local-stt`)
- **FLAC support** — `encode_flac()`, `decode_flac()` (feature: `flac`)

---

## Code Interpreters

**Crate:** `brainwires-code-interpreters`

Sandboxed multi-language code execution.

| Language | Feature | Engine | Notes |
|----------|---------|--------|-------|
| **Rhai** | `rhai` | Native Rust | Fastest startup |
| **Lua** | `lua` | mlua | Small runtime, good stdlib |
| **JavaScript** | `javascript` | Boa | ECMAScript compliant |
| **Python** | `python` | RustPython | CPython 3.12 compatible |

- **Executor** — Unified execution interface with `ExecutionRequest`
- **WASM support** — Browser-compatible execution (feature: `wasm`)
- **Language detection** — `supported_languages()`, `is_language_supported()`

---

## Skills System

**Crate:** `brainwires-skills`

Markdown-based agent skill packages.

- **SKILL.md format** — YAML frontmatter (name, description, allowed-tools, model, metadata) + markdown body
- **SkillRegistry** — Skill registration and lookup
- **SkillRouter** — Automatic skill matching from user input
- **SkillExecutor** — Execution modes: `SubagentPrepared` (delegate to subagent) or `ScriptPrepared`
- **Progressive disclosure** — Metadata loaded at startup, full content loaded on-demand
- **SkillSource** — Multiple sources (built-in, user, project)

---

## Channels

**Crate:** `brainwires-channels`

Universal messaging channel contract for adapter implementations (Discord, Telegram, Slack, etc.).

- **Channel** trait — Core interface that all messaging adapters must implement
- **ChannelMessage** — Core message types with attachments, embeds, and media
- **ChannelEvent** — Events: message received, edited, deleted, reactions, presence changes
- **ChannelCapabilities** — Capability flags for adapter feature negotiation
- **ChannelUser** / **ChannelSession** — User and session identity types
- **ChannelHandshake** — Gateway handshake protocol for adapter registration
- **Conversion** — Bidirectional conversion between `ChannelMessage` and agent-network `MessageEnvelope`

---

## Datasets & Training Data

**Crate:** `brainwires-datasets`

Training data pipelines for fine-tuning workflows.

### I/O

- **JsonlReader** / **JsonlWriter** — Streaming JSONL I/O for training examples and preference pairs

### Data Types

- **TrainingExample** — Messages with roles and content
- **PreferencePair** — Chosen/rejected response pairs for RLHF/DPO
- **TrainingMessage** / **TrainingRole** — Message-level types

### Format Conversion

- **OpenAiFormat** — OpenAI fine-tuning format
- **TogetherFormat** — Together AI format
- **AlpacaFormat** — Alpaca instruction format
- **ShareGptFormat** — ShareGPT conversation format
- **ChatMlFormat** — ChatML format
- **detect_format()** — Automatic format detection

### Quality & Validation

- **DataValidator** — Configurable validation with `ValidationReport`
- **DatasetStats** — Token distributions, role counts, histogram buckets
- **Deduplicator** — Exact deduplication for examples and preference pairs (feature: `dedup`)

### Sampling

- **train_eval_split()** — Train/eval splitting with configurable ratios
- **curriculum_order()** — Curriculum learning ordering
- **sample_n()** — Random sampling

### Tokenization

- **HfTokenizer** — Hugging Face tokenizers (feature: `hf-tokenizer`)
- **TiktokenTokenizer** — OpenAI tiktoken (feature: `tiktoken`)

---

## Model Training & Fine-Tuning

**Crate:** `brainwires-training`

Cloud and local model fine-tuning.

### Cloud Fine-Tuning (feature: `cloud`)

- **FineTuneProvider** trait — Unified interface for all cloud providers
- **FineTuneProviderFactory** — Create providers from config
- Supported providers: **OpenAI**, **Together**, **Fireworks**, **Anyscale**, **Bedrock**, **Vertex AI**

### Local Training (feature: `local`)

- **LoRA** — Low-Rank Adaptation
- **QLoRA** — Quantized LoRA
- **DoRA** — Weight-Decomposed LoRA
- **Burn framework** — GPU-accelerated training via wgpu + ndarray backends
- **ComputeDevice** — CPU, GPU, or auto-detect
- **TrainedModelArtifact** — Output artifacts with SafeTensors weight loading

### Configuration

- **TrainingHyperparams** — Learning rate, epochs, batch size, warmup
- **LoraConfig** — Rank, alpha, dropout, target modules
- **AdapterMethod** — LoRA, QLoRA, DoRA selection
- **AlignmentMethod** — SFT, DPO, RLHF
- **LrScheduler** — Cosine, linear, constant, warmup

### Job Management

- **TrainingManager** — Job lifecycle management
- **TrainingJobStatus** — Queued, running, completed, failed
- **TrainingProgress** — Step counts, loss tracking, ETA

---

## Distributed Mesh Networking

**Crate:** `brainwires-agent-network` (feature: `mesh`)

Connect agents across processes and machines.

- **MeshTopology** — Topology management with layout types (`TopologyType`)
- **MeshNode** — Node definitions with `NodeCapabilities` and `NodeState`
- **MessageRouter** — Message routing with multiple strategies (`RoutingStrategy`)
- **RouteEntry** — Route table entries
- **PeerDiscovery** — Peer discovery protocols (`DiscoveryProtocol`)
- **FederationGateway** — Cross-mesh communication with `FederationPolicy`

---

## Agent-to-Agent (A2A) Protocol

**Crate:** `brainwires-a2a`

Implementation of Google's A2A protocol for interoperable agent communication.

- **AgentCard** — Discovery metadata describing capabilities and skills
- **Task lifecycle** — Submission, execution tracking, artifact delivery (`TaskState`, `TaskSendParams`, `TaskQueryParams`)
- **Message types** — Text, file, and structured data parts (`Part`, `Artifact`)
- **Authentication** — Pluggable auth schemes: API key, OAuth2, JWT, Bearer
- **AgentProvider** / **AgentSkill** — Provider and skill metadata
- **JSON-RPC 2.0** — Full request/response envelopes with typed method constants
- **Push notifications** — `TaskPushNotificationConfig`, `AuthenticationInfo`
- **Streaming** — Server-Sent Events for real-time task updates
- **Client** — HTTP client with JSON-RPC and REST transports (feature: `client`)
- **Server** — HTTP server with JSON-RPC and REST routers (feature: `server`)
- **gRPC** — Protocol Buffers types, client transport, and server service (feature: `grpc`)

---

## Autonomous Operations

**Crate:** `brainwires-autonomy`

Self-improvement, Git workflows, and human-out-of-loop execution.

### Self-Improvement (feature: `self-improve`)

- **SelfImprovementController** — Autonomous improvement cycles
- **ImprovementStrategy** / **ImprovementCategory** — Strategy definitions
- **TaskGenerator** — Generate improvement tasks
- **Comparator** — Compare before/after results (`ComparisonResult`, `PathResult`)

### Eval-Driven Feedback (feature: `eval-driven`)

- **AutonomousFeedbackLoop** — Continuous evaluation and improvement
- **FeedbackLoopConfig** / **FeedbackLoopReport**

### Git Workflow Pipeline (feature: `git-workflow`)

- **GitWorkflowPipeline** — Full pipeline: trigger -> investigate -> branch -> fix -> PR -> merge
- **GitForge** trait — Abstraction over GitHub, GitLab, etc.
- **IssueInvestigator** — Analyze issues to determine fix approach
- **BranchManager** — Branch creation and management
- **ChangeMaker** — Apply code changes
- **PullRequestManager** — PR creation and management
- **MergePolicy** — Automated merge decisions
- **WorkflowTrigger** — Event triggers (programmatic, webhook)
- **WebhookServer** — HTTP server for Git forge events (feature: `webhook`)

### Agent Operations

- **AgentSupervisor** — Health monitoring and recovery (feature: `supervisor`)
- **AttentionMechanism** — RAG-integrated attention (feature: `attention`)
- **ParallelCoordinator** — Parallel agent coordination with optional MDAP (feature: `parallel`)
- **HealthMonitor** — `HealthStatus`, `DegradationSignal`, `PerformanceMetrics`
- **HibernateManifest** — Session hibernation and resume

### Safety

- **SafetyGuard** — Safety checks for autonomous operations
- **ApprovalPolicy** — Human approval requirements
- **AutonomousOperation** — Operation classification

### Metrics

- **SessionMetrics** — Per-session performance tracking
- **SessionReport** — Summary reports

---

## Reasoning & Inference

**Module:** `brainwires-agents::reasoning` (feature: `reasoning`)

Provider-agnostic inference components for quality and cost optimization.

### Named Reasoning Strategies

- **ReasoningStrategy** trait — Common interface for reasoning loop control (`system_prompt()`, `is_complete()`, `next_action()`)
- **ReActStrategy** — Thought → Action → Observation loop (Yao et al., 2022) with configurable max steps
- **ReflexionStrategy** — Self-critique after each action with revised plans (Shinn et al., 2023)
- **ChainOfThoughtStrategy** — "Let's think step by step" structured reasoning (Wei et al., 2022)
- **TreeOfThoughtsStrategy** — Multi-branch exploration with pruning and best-path selection (Yao et al., 2023)
- **StrategyStep** — Typed reasoning trace steps: `Thought`, `Action`, `Observation`, `Reflection`, `Branch`

### Tier 1 — Quick Wins

- **LocalRouter** — Semantic query classification for tool routing
- **ComplexityScorer** — Task complexity scoring for adaptive MDAP k values
- **LocalValidator** — Response validation for red-flagging

### Tier 2 — Context & Retrieval

- **LocalSummarizer** — Context summarization for tiered memory demotion
- **RetrievalClassifier** — Enhanced retrieval gating with semantic understanding
- **RelevanceScorer** — Context re-ranking based on semantic relevance
- **StrategySelector** — Decomposition strategy selection for MDAP
- **EntityEnhancer** — Semantic entity extraction beyond regex patterns

All components accept `Arc<dyn Provider>` and fall back to pattern-based logic when unavailable.

---

## Evaluation Framework

**Module:** `brainwires-agents::eval` (feature: `eval`)

Monte Carlo evaluation framework for agent quality assurance.

- **EvaluationSuite** — N-trial Monte Carlo runner with `SuiteConfig`
- **EvaluationCase** trait — Single evaluatable scenario, with built-in helpers (`AlwaysPassCase`, `AlwaysFailCase`, `StochasticCase`)
- **TrialResult** / **EvaluationStats** — Per-trial results with Wilson-score 95% confidence intervals
- **ToolSequenceRecorder** — Record and diff tool call sequences (`SequenceDiff`)
- **AdversarialTestCase** — Prompt injection, ambiguity, budget stress tests
- **Regression tests** — Regression detection across versions
- **Stability tests** — Consistency checks
- **Fault reports** — Structured fault documentation

---

## Proxy Framework

**Crate:** `brainwires-proxy` *(extras/)*

Protocol-agnostic proxy for debugging AI API traffic.

- **ProxyBuilder** — Fluent API for proxy construction
- **ProxyService** — Core proxy engine
- **Transports:**
  - HTTP/HTTPS via hyper (feature: `http`)
  - WebSocket via tokio-tungstenite (feature: `websocket`)
  - TLS termination via tokio-rustls (feature: `tls`)
- **Middleware stack** — `ProxyLayer`, `LayerAction`, composable `MiddlewareStack`
- **Format conversion** — `Converter`, `StreamConverter`, `ConversionRegistry`, `FormatDetector`
- **Inspector API** — HTTP query API for captured traffic (feature: `inspector-api`)
- **Request tracking** — `RequestId` for correlating requests/responses

---

## WASM Bindings

**Crate:** `brainwires-wasm`

Browser-compatible WASM bindings via `wasm-bindgen`.

- **Message validation** — `validate_message()` for JSON message normalization
- **Tool validation** — `validate_tool()` for JSON tool definition validation
- **History serialization** — `serialize_history()` for stateless protocol format
- **Version** — `version()` for framework version
- **Code interpreters** — WASM interpreter support (feature: `interpreters`)
- **Orchestrator** — WASM orchestrator with execution limits (feature: `orchestrator`)

---

## Extras & Standalone Binaries

### brainwires-brain-server *(extras/)*

MCP server binary wrapping `brainwires-brain` for use with AI assistants (Claude Desktop, etc.).

### brainwires-rag-server *(extras/)*

MCP server binary wrapping `brainwires-rag` for semantic code search via MCP protocol.

### agent-chat *(extras/)*

Simplified open-source AI chat client built on the framework. Includes CLI commands for config, models, and auth.

### reload-daemon *(extras/)*

File-watching daemon for automatic server reloading during development.

### brainwires-gateway *(extras/)*

WebSocket/HTTP gateway daemon connecting messaging channels to agents. Manages channel adapters, agent routing, and webhook ingestion.

### brainwires-discord-channel *(extras/)*

Discord channel adapter implementing the `Channel` trait from `brainwires-channels`.

### brainwires-skill-registry *(extras/)*

Skill registry service for discovering and distributing agent skills.

---

## Facade Crate & Feature Flags

**Crate:** `brainwires`

Re-exports all framework crates behind feature flags.

| Feature | Default | Description |
|---------|---------|-------------|
| `core` | Always | Core types and traits |
| `tools` | Yes | Tool definitions and execution |
| `agents` | Yes | Multi-agent orchestration |
| `providers` | No | AI provider integrations |
| `chat` | No | Chat provider wrappers (alias for `providers`) |
| `storage` | No | Vector storage and semantic search |
| `mcp` | No | MCP client support |
| `mcp-server` | No | MCP server re-exports (rmcp, schemars) |
| `mdap` | No | MDAP voting framework |
| `prompting` | No | Adaptive prompting techniques |
| `knowledge` | No | BKS/PKS knowledge systems (alias for `brain`) |
| `brain` | No | Central knowledge crate |
| `permissions` | No | Capability-based permissions |
| `seal` | No | Self-Evolving Agentic Learning |
| `agent-network` | No | MCP server, IPC, remote bridge |
| `rag` | No | RAG engine with code search |
| `rag-full-languages` | No | RAG + all Tree-sitter language parsers |
| `interpreters` | No | Sandboxed code interpreters |
| `orchestrator` | No | Rhai script orchestration |
| `reasoning` | No | Local inference components |
| `openapi` | No | OpenAPI 3.x spec → Tool generation |
| `otel` | No | OpenTelemetry span export for agent traces |
| `eval` | No | Evaluation framework |
| `skills` | No | SKILL.md skill system |
| `audio` | No | Audio capture, STT, TTS |
| `datasets` | No | Training data pipelines |
| `training` | No | Model training (base types) |
| `training-cloud` | No | Cloud fine-tuning providers |
| `training-local` | No | Local LoRA/QLoRA/DoRA training |
| `training-full` | No | All training + all datasets |
| `autonomy` | No | Autonomous operations |
| `mesh` | No | Distributed agent mesh |
| `a2a` | No | Agent-to-Agent protocol |
| `proxy` | No | Protocol proxy framework |
| `wasm` | No | WASM browser bindings |
| `bedrock` | No | Amazon Bedrock provider |
| `vertex-ai` | No | Google Vertex AI provider |
| `llama-cpp-2` | No | Local LLM inference |
| `learning` | No | SEAL + knowledge integration |
| `agent-full` | No | agents + permissions + prompting + tools |
| `researcher` | No | providers + agents + storage + rag + training + datasets |
| `full` | No | Everything enabled |
