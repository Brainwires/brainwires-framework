# Changelog

All notable changes to the Brainwires Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### New Crates

- **`brainwires-channels`** — Universal messaging channel contract for adapter implementations. Provides `Channel` trait (7 async methods), `ChannelMessage`, `ChannelEvent` (8 variants), `ChannelCapabilities` (12 bitflags), `ChannelUser`, `ChannelSession`, `ConversationId`, and `ChannelHandshake` protocol. Bidirectional conversion between `ChannelMessage` and agent-network `MessageEnvelope`.
- **`brainwires-mcp-server`** — MCP server framework extracted from `brainwires-agent-network`. Provides `McpServer`, `McpHandler` trait, `McpToolRegistry` (declarative tool registration + dispatch), `ServerTransport`/`StdioServerTransport`, and a composable middleware pipeline: `AuthMiddleware`, `LoggingMiddleware`, `RateLimitMiddleware`, `ToolFilterMiddleware`.

#### Agents (`brainwires-agents`)

- **`ChatAgent`** — Reusable streaming completion loop with per-user session management. Methods: `restore_messages()`, `compact_history()`.
- **Session persistence** — `SessionStore` trait + `JsonFileStore` implementation for persisting conversation history across restarts. Wired into BrainClaw via `memory.persist_conversations` config.

#### Tool System (`brainwires-tool-system`)

- **`BuiltinToolExecutor`** — Centralized dispatch executor for all built-in tools, eliminating duplication across agent implementations.
- **Email tools** (feature `email`) — IMAP/SMTP/Gmail read, send, search, and manage operations.
- **Calendar tools** (feature `calendar`) — Google Calendar/CalDAV event creation, listing, and update operations.

#### Code Interpreters (`brainwires-code-interpreters`)

- **Docker sandbox** — Container-isolated code execution via Docker; `Dockerfile.sandbox` at `crates/brainwires-code-interpreters/docker/`.

#### Skills (`brainwires-skills`)

- **`SkillPackage`** — Distributable skill package format with manifest, skill_content, SHA-256 checksum, and optional ed25519 signature.
- **`RegistryClient`** — HTTP client for publishing to and downloading from a skill registry server.
- **ed25519 signing** (feature `signing`) — Sign and verify skill packages for supply-chain safety.

#### Agent Networking (`brainwires-agent-network`)

- **Device allowlists** — `DeviceAllowlist`, `DeviceStatus` (Allowed/Blocked/Pending), `OrgPolicies`. Bridge computes a SHA-256 device fingerprint from machine-id + hostname + OS on every `Register` message; bails on `Blocked` status from server.
- **Sender verification** — Channel-type and channel-ID allowlists enforced at WebSocket handshake time; master `channels_enabled` switch.
- **Permission relay** — `PermissionRequest`/`PermissionResponse` message types. `PermissionRelay` module with pending request map (oneshot channels), session-allowed list, and configurable timeout. `RemoteBridge::send_permission_request()` sends a request and awaits approval; auto-denies on timeout.

#### Hardware (`brainwires-hardware`)

- **Voice Activity Detection** (always available with `audio`) — `VoiceActivityDetector` trait + `EnergyVad` (pure-Rust RMS energy threshold, no extra deps). Feature `vad` adds `WebRtcVad` (three aggressiveness modes: Quality, LowBitrate, Aggressive, VeryAggressive) via `webrtc-vad 0.4`. Helpers: `SpeechSegment`, `rms_db()`, `pcm_to_i16_mono()`, `pcm_to_f32()`.
- **Wake word detection** (feature `wake-word`) — `WakeWordDetector` trait + `WakeWordDetection` event. `EnergyTriggerDetector` — zero-dependency energy-burst trigger (fires when audio energy exceeds a dB threshold for N consecutive 30 ms frames). Optional `wake-word-rustpotter` feature adds `RustpotterDetector` (pure-Rust DTW/ONNX, `.rpw` model files). Optional `wake-word-porcupine` feature adds `PorcupineDetector` (Picovoice, builtin keywords + custom `.ppn` files).
- **Voice assistant pipeline** (feature `voice-assistant`) — `VoiceAssistant` orchestrates the full listen → wake word → VAD-gated capture → STT → handler → TTS → playback loop. `VoiceAssistantBuilder` for composing components. `VoiceAssistantHandler` async trait (`on_wake_word`, `on_speech`, `on_error`). `VoiceAssistantConfig` (silence threshold/duration, max record duration, listen timeout, STT/TTS options, device selection). `AssistantState` enum (Idle/Listening/Processing/Speaking). `listen_once()` for single-shot capture + transcription without handler callbacks.
- **Camera capture** (feature `camera`) — Cross-platform webcam/camera frame capture via `nokhwa` (V4L2 on Linux, AVFoundation on macOS, Media Foundation on Windows). `CameraCapture` async trait, `NokhwaCapture` impl with `spawn_blocking` bridge, `list_cameras()`, `open_camera(index, format)`, automatic MJPEG→RGB decoding. Types: `CameraDevice`, `CameraFrame`, `CameraFormat`, `Resolution`, `FrameRate`, `PixelFormat`, `CameraError`.
- **Raw USB access** (feature `usb`) — Device enumeration and async bulk/control/interrupt transfers via `nusb` (pure Rust, no libusb system dependency). `UsbHandle::open()` auto-discovers bulk endpoints from the interface descriptor. Types: `UsbDevice`, `UsbClass` (full USB-IF class code map), `UsbSpeed`, `UsbError`. `list_usb_devices()` reads string descriptors (manufacturer, product, serial) with graceful permission-error fallback.
- **`brainwires-hardware` renamed from `brainwires-audio`** — Unified hardware abstraction crate. GPIO moved from `brainwires-autonomy`; Bluetooth and Network hardware added. `brainwires-autonomy` re-exports GPIO via `pub use brainwires_hardware::gpio` for backward compatibility.
- **Deprecated `brainwires-audio`** — Stub crate at `deprecated/brainwires-audio`; re-exports `brainwires-hardware` with `audio` feature. Final release for ecosystem continuity.

#### Autonomy (`brainwires-autonomy`)

- **Autodream memory consolidation** (feature `dream`) — 4-phase consolidation cycle: orient → gather → consolidate → prune. Types: `DreamConsolidator`, `DemotionPolicy` (age/importance/budget thresholds), `DreamSummarizer` (LLM-powered compression), `FactExtractor` (5 categories: entities, relationships, events, preferences, habits), `DreamMetrics`, `DreamReport`, `DreamTask` (scheduled via `AutonomyScheduler`).

#### Extras — Voice Assistant (`extras/voice-assistant/`)

- **`voice-assistant`** binary — Personal voice assistant built on the framework. Mic capture → optional energy wake trigger → VAD-gated speech accumulation → OpenAI Whisper STT → LLM response (OpenAI chat completions) → OpenAI TTS playback. CLI flags: `--config <path.toml>`, `--list-devices`, `--wake-word <model>`, `--verbose`. TOML config covers STT model, TTS voice, silence tuning, wake word model, LLM model/system prompt, and device names. Clean Ctrl-C shutdown via `tokio::signal`.

#### Extras — BrainClaw Suite (`extras/brainclaw/`)

- **`brainclaw`** (daemon) — Self-hosted personal AI assistant. Multi-provider support (Anthropic, OpenAI, Google, Ollama, Groq, Together, Fireworks, Bedrock, Vertex AI), per-user agent sessions, TOML config (`~/.brainclaw/brainclaw.toml`), native/email/calendar feature flags.
- **`brainwires-gateway`** — WebSocket/HTTP channel hub. `InboundHandler` trait for custom message processing; built-in `AgentInboundHandler` bridging channel events to `ChatAgent` sessions. WebChat browser UI at `/chat` with WebSocket at `/chat/ws`. Admin API (`/admin/*`) with Bearer token auth. Admin browser dashboard at `GET /admin/ui` (single-file dark-themed SPA; sections: Dashboard, Channels, Sessions, Cron Jobs, Identity, Broadcast). Webhook endpoint (`POST /webhook`) with HMAC-SHA256 verification. Media pipeline: attachment download, image description, audio transcription, size validation. Audit logger: structured JSON ring buffer via `tracing`. Metrics: atomic counters for messages, tool calls, errors, rate limits, spoofing blocks, and per-channel breakdowns. `/model` slash command for per-session model switching (`/model list`, `/model <name>`, `/model default`).
- **`brainwires-discord-channel`** — Discord bot adapter (serenity). Reference `Channel` trait implementation. Optional MCP tool server mode (`--mcp`).
- **`brainwires-telegram-channel`** — Telegram bot adapter (teloxide). `Channel` trait implementation, bidirectional gateway relay, optional MCP tool server (`--mcp`).
- **`brainwires-slack-channel`** — Slack adapter using Socket Mode (reqwest, no public URL required). `Channel` trait implementation, optional MCP tool server (`--mcp`).
- **`brainwires-mattermost-channel`** — Mattermost adapter using Mattermost WebSocket API. `Channel` trait implementation with send/edit/delete/history/react. Filtering: self-messages, channel allowlist, @mention requirement, team scoping. Optional MCP tool server (`--mcp`). Capabilities: `RICH_TEXT | THREADS | REACTIONS | TYPING_INDICATOR | EDIT_MESSAGES | DELETE_MESSAGES | MENTIONS`.
- **`brainwires-signal-channel`** — Signal messenger adapter via `signal-cli-rest-api`. WebSocket push mode with polling fallback. `Channel` trait implementation. Filtering: self-messages, sender/group allowlists, @mention/keyword trigger for groups. Optional MCP tool server (`--mcp`): `send_message`, `add_reaction`. Capabilities: `REACTIONS`.
- **`brainwires-skill-registry`** — HTTP skill registry server. SQLite with FTS5 full-text search. Endpoints: publish, search (query + tag filter), get manifest (latest or by version), download package. Auto-creates schema on first run.

#### Core Types (`brainwires-core`)

- **`ChatOptions::model`** — New `model: Option<String>` field. When `Some`, all providers (Anthropic, OpenAI, OpenAI Responses, Gemini, Ollama, and OpenAI-compatible) substitute this model for their configured default on that request. Enables per-request and per-session model switching without recreating the provider. `ChatOptions` gains a `.model()` builder method.

### Fixed

#### Facade (`brainwires`)

- Removed `brainwires-proxy` from the `full` feature flag. Extras are consumers of the framework, not framework dependencies; external consumers (such as `brainwires-cli`) do not have extras in their workspace. The `proxy` feature remains available as an explicit opt-in.

### Refactored

- **BrainClaw workspace** — BrainClaw is now a self-contained Cargo workspace at `extras/brainclaw/`, excluded from the root workspace via `[workspace].exclude`. Members use path dependencies back to `crates/` for framework libraries.
- **Docker Dockerfile** — Moved `extras/docker/Dockerfile.sandbox` to `crates/brainwires-code-interpreters/docker/` where it belongs alongside the crate it supports.
- **`brainwires-mcp-server` extracted** — MCP server framework code was split out of `brainwires-agent-network` into its own publishable crate. `brainwires-agent-network` now depends on `brainwires-mcp-server`; consumers that only need to build MCP servers no longer need to pull in the full networking stack.
- **`brainwires-channels` optional dep** — `brainwires-channels`' dependency on `brainwires-agent-network` is now optional, gated behind the `agent-network` feature flag (conversion module).

## [0.6.0] - 2026-03-23

### Changed

#### A2A Protocol (`brainwires-a2a`, `deno/a2a`)
- **BREAKING:** Updated A2A protocol implementation from v0.3 to v1.0.
- **Part type redesigned:** Replaced discriminated union (`kind: text/file/data`) with unified flat struct (`text`/`raw`/`url`/`data` as optional oneof fields + `mediaType`, `filename`).
- **Enum values → SCREAMING_SNAKE_CASE:** Role (`ROLE_USER`, `ROLE_AGENT`), TaskState (`TASK_STATE_SUBMITTED`, `TASK_STATE_WORKING`, etc.) per ProtoJSON specification.
- **Removed `kind` field** from `Message`, `Task`, and streaming event objects.
- **Stream events use wrapper pattern:** `StreamResponse` with `task`/`message`/`statusUpdate`/`artifactUpdate` wrapper fields instead of `kind`-based discrimination.
- **SecurityScheme and OAuthFlows** changed from `type`-discriminated to wrapper-based oneOf pattern.
- **JSON-RPC method names** updated to PascalCase (`message/send` → `SendMessage`, etc.).
- **REST:** `GET /tasks/{id}:subscribe` changed to `POST`.
- **`SendMessageConfiguration.blocking`** renamed to `returnImmediately`.
- **`PushNotificationConfig.id`** renamed to `configId`, added `createdAt`.
- **`AgentCard.supportedInterfaces`** is now required.
- **New error codes:** `ExtensionSupportRequired` (-32008), `VersionNotSupported` (-32009).

#### Code Interpreters (`brainwires-code-interpreters`)
- Disabled Python/RustPython feature to resolve `libsqlite3-sys` version conflict with `brainwires-cognition`.

## [0.5.0] - 2026-03-15

### Added

#### Autonomy (`brainwires-autonomy`)
- **Crash recovery** (feature `crash-handler`): Detect crashed processes → AI-powered diagnostics → automatic fix → rebuild → relaunch. Persistent recovery state tracking across restarts.
- **CI/CD orchestrator** (feature `cicd`): GitHub Issues → investigate → fix → PR → merge pipeline. Webhook config, variable interpolation, event logging.
- **Cron scheduler** (feature `scheduler`): Recurring autonomous tasks with cron-expression triggers and configurable failure policies (retry, skip, abort).
- **File system reactor** (feature `reactor`): Watch directories with glob-based rules, debounced event dispatch, and rate limiting.
- **Service management** (feature `services`): Manage systemd units, Docker containers, and OS processes with hardcoded deny-list safety and allow-list enforcement.
- **GPIO hardware control** (feature `gpio`): Pin manager with allow-list policies, PWM configuration, and auto-release timeouts.
- 117 tests across all 6 new features; each feature flag compiles independently.

#### Examples
- **16 examples across 9 crates**: permissions (`policy_engine`, `trust_audit`), MDAP (`voting_consensus`, `task_decomposition`), skills (`skill_registry`), code-interpreters (`multi_language`), A2A (`agent_card`), cognition (`prompting_techniques`), autonomy (`safety_guard`), agent-network (`middleware_chain`), and 6 agent coordination patterns (`contract_net`, `saga_compensation`, `task_queue_scheduling`, `optimistic_concurrency`, `three_state_model`, `validation_loop`).
- **10 examples for brainwires-autonomy**: `health_monitor`, `session_metrics`, `crash_recovery`, `self_improve_strategies`, `git_workflow_pipeline`, `cicd_orchestrator`, `cron_scheduler`, `fs_reactor`, `service_manager`, `gpio_pins`.

#### Documentation
- **BYO database guide** (`databases/README.md`): Step-by-step guide for implementing custom `StorageBackend` and `VectorDatabase` backends, with trait method documentation and integration patterns.

#### Crate Merges (19 → 18 crates)
- **`brainwires-mdap`** merged into `brainwires-agents` behind the `mdap` feature flag. The standalone `brainwires-mdap` crate is now deprecated; use `brainwires-agents = { version = "0.5", features = ["mdap"] }` instead.

#### Build & CI (`xtask`)
- **`package-count` command**: `cargo xtask package-count [--dry-run]` counts workspace members (crates vs extras) and updates stale count references in `.md` files. Skips CHANGELOG.md, deprecated directories, code blocks, and historical arrow lines.
- **Deprecated crate publishing**: `publish.sh` now publishes deprecated stub crates (e.g. `brainwires-mdap`) after all workspace crates, with non-fatal error handling.

#### Testing
- **472 integration tests across 6 crates**: agent-network (47), agents (53), audio (93), code-interpreters (142), skills (82), wasm (55).

#### Code Quality
- Resolved all 16 `check-stubs` false-positive warnings by rewording doc comments and adding `todo_scanner.rs` to the skip list.

### Changed

#### Providers (`brainwires-providers`)
- Updated default models: Anthropic now defaults to latest Claude model, OpenAI to latest GPT model.

#### Build & Publishing
- `publish.sh` enhanced with smarter version tagging logic to handle patch bumps correctly.
- Version replacement logic improved to handle doc comments in Rust files.
- README version example updated to 0.4.

#### Documentation
- `brainwires-autonomy` README rewritten with new features, feature flags, examples, and safety documentation.

## [0.4.1] - 2026-03-15

### Added

#### Storage (`brainwires-storage`)
- **`PostgresDatabase` StorageBackend impl** (1900+ lines across all 3 backends):
  - `FieldValue`→`ToSql` type conversion for all 9 field types (including `pgvector::Vector` for embedding columns).
  - `vector_search` via pgvector `<=>` (cosine distance) operator with parameterized SQL.
  - `row_to_record` parser using `tokio_postgres` column type metadata (`Type::TEXT`, `Type::INT4`, `Type::FLOAT8`, `Type::BOOL`, etc.) with automatic pgvector detection for unknown types.
  - Helper functions `field_values_to_params` and `params_as_refs` for ergonomic boxed `ToSql` parameter handling.
  - Full `create_table`, `insert`, `query`, `update`, `delete`, `vector_search` implementations via the shared `PostgresDialect` SQL generator.
- **`MySqlDatabase` backend** via `mysql_async` (~490 lines):
  - Full `StorageBackend` implementation with connection pooling (`mysql_async::Pool`).
  - Connectivity verification on construction (ping + disconnect handshake).
  - Vector columns stored as JSON arrays; `vector_search` performs client-side cosine similarity since MySQL lacks native vector types.
  - SQL generation via the shared `MySqlDialect`.
  - New `mysql-backend` feature flag with `mysql_async` dependency.
- **`SurrealDatabase` backend** via `surrealdb` 2.x SDK (~1160 lines):
  - Both `StorageBackend` and `VectorDatabase` trait implementations.
  - Native MTREE KNN vector search with cosine distance using SurrealDB's vector indexing.
  - `with_config()` constructor for explicit credentials; default `new()` uses `root`/`root`.
  - Client-side BM25 scoring for hybrid (vector + keyword) queries via shared `bm25_helpers`.
  - Glob-based file path filtering via shared `glob_utils`.
  - `DatabaseStats`, `ChunkMetadata`, and `SearchResult` type support for full RAG compatibility.
  - New `surrealdb-backend` feature flag with `surrealdb` dependency.

#### Build & CI (`xtask`)
- **Smart version bumping**: Full workspace-aware version bump system with:
  - `--crates` flag parsing and bump mode detection (full vs patch).
  - Workspace dependency graph construction and cascade logic (bumping a crate also bumps its dependents).
  - Auto-detection of changed crates from `git diff` for selective patch-mode bumping.
  - Reset of explicit version overrides on minor/major bumps.
  - Selective patch-mode version bumping for targeted releases.
  - Wired up full + patch mode execution paths.
- **`check-stubs` command**: Scans all `.rs` files for hard blockers (`todo!()`, `unimplemented!()`) and soft markers (`FIXME`, `HACK`, `XXX`, `STUB`, `STOPSHIP`, `"not implemented"`). Skips test code, uses word-boundary detection to avoid false positives. Supports `--strict` (markers = errors) and `--verbose` flags.
- **CHANGELOG stamping**: `bump-version` now renames `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` and inserts a fresh empty `## [Unreleased]` section above it.

### Removed

#### Storage (`brainwires-storage`)
- Removed `MySqlDatabase` and `SurrealDatabase` stub backends (contained `todo!()` placeholders), replaced by real implementations (see Added above).
- SQL dialect files (`sql/mysql.rs`, `sql/surrealdb.rs`) retained for future use.

### Changed

#### Storage (`brainwires-storage`)
- `databases/mod.rs` updated with conditional module exports for `mysql` and `surrealdb` behind their respective feature flags.
- `lib.rs` updated to re-export new database modules.
- `sql/mod.rs` documentation updated to reference all three SQL dialect implementations.
- README updated with MySQL and SurrealDB backend entries in the database matrix.

#### Dependencies
- Added `mysql_async` (feature `mysql-backend`) for MySQL/MariaDB connection pooling.
- Added `surrealdb` (feature `surrealdb-backend`) for SurrealDB 2.x SDK integration.

#### Documentation
- Updated `PUBLISHING.md` with smart version bump instructions and `check-stubs` checklist wording.

#### Code Quality
- Applied formatting improvements across the workspace for consistency and readability.

## [0.4.0] - 2026-03-14

### Breaking Changes

#### Storage (`brainwires-storage`)
- **Unified database layer**: Merged `clients/` (7 VectorDatabase impls) and `stores/backends/` (StorageBackend impl) into a single `databases/` module. One struct per database, one shared connection, implementing `StorageBackend` and/or `VectorDatabase`.
- Removed `clients/` module entirely — all database implementations now live in `databases/<name>/`.
- Removed `stores/backend.rs`, `stores/backends/`, `stores/lance_client.rs` — merged into `databases/lance/`.
- Renamed all database structs: `LanceVectorDB` → `LanceDatabase`, `QdrantVectorDB` → `QdrantDatabase`, `PostgresVectorDB` → `PostgresDatabase`, `PineconeVectorDB` → `PineconeDatabase`, `MilvusVectorDB` → `MilvusDatabase`, `WeaviateVectorDB` → `WeaviateDatabase`, `NornicVectorDB` → `NornicDatabase`.
- `LanceBackend` merged into `LanceDatabase` — implements both `StorageBackend` and `VectorDatabase` on a single `lancedb::Connection`.
- PostgreSQL backend switched from `sqlx` to `tokio-postgres` + `deadpool-postgres` to avoid `libsqlite3-sys` version conflict with `rusqlite`.

#### Cognition (`brainwires-cognition`)
- `RagClient` now stores `Arc<dyn VectorDatabase>` instead of concrete database types. Added `with_vector_db()` constructor for external injection.
- `BrainClient` rewritten to use `Arc<dyn StorageBackend>` instead of raw LanceDB/arrow APIs. Added `with_backend()` constructor.
- `u64` fields in PKS/BKS cache now cast through `i64` for `rusqlite` 0.38 compatibility.

### Added

#### Storage (`brainwires-storage`)
- **`databases/` module** — unified database layer with:
  - `traits.rs`: `StorageBackend` + `VectorDatabase` traits (always available, no feature gate)
  - `types.rs`: `FieldDef`, `FieldType`, `FieldValue`, `Record`, `ScoredRecord`, `Filter` types
  - `capabilities.rs`: `BackendCapabilities` struct for runtime feature detection
  - `sql/`: Shared SQL generation layer with `SqlDialect` trait + `PostgresDialect`, `MySqlDialect`, `SurrealDialect` implementations
  - `lance/`: `LanceDatabase` (both traits, embedded LanceDB)
  - `postgres/`: `PostgresDatabase` (VectorDatabase, via tokio-postgres + pgvector)
  - `qdrant/`: `QdrantDatabase` (VectorDatabase)
  - `pinecone/`: `PineconeDatabase` (VectorDatabase, REST API)
  - `milvus/`: `MilvusDatabase` (VectorDatabase, REST API)
  - `weaviate/`: `WeaviateDatabase` (VectorDatabase, REST API)
  - `nornicdb/`: `NornicDatabase` (VectorDatabase, multi-transport: REST/Bolt/gRPC)
- New feature flags: `postgres-backend` (alongside existing `lance-backend`, `qdrant-backend`, `pinecone-backend`, `weaviate-backend`, `milvus-backend`, `nornicdb-*`).
- `async-trait` is now a required (non-optional) dependency — core traits are always available regardless of feature flags.
- 112 tests: 18 SQL dialect tests, Lance CRUD/vector-search/capabilities/shared-connection tests, 2 integration tests (trait object CRUD, backend capabilities).

#### Cognition (`brainwires-cognition`)
- `RagClient::with_vector_db()` — construct with any `Arc<dyn VectorDatabase>` for backend-agnostic RAG.
- `BrainClient::with_backend()` — construct with any `Arc<dyn StorageBackend>` for backend-agnostic knowledge storage.

### Changed

#### Storage (`brainwires-storage`)
- Domain stores (`MessageStore`, `ConversationStore`, `TaskStore`, `PlanStore`, `SummaryStore`, `FactStore`, `ImageStore`, `TierMetadataStore`, `AgentStateStore`) now default to `LanceDatabase` instead of the removed `LanceBackend`.
- `PersistentTaskManager` and `TieredMemory` updated to use `LanceDatabase`.
- README rewritten with unified database backends section, trait implementation matrix, connection sharing examples, and feature flag reference.
- Module-level and crate-level documentation updated to reflect new architecture.

#### Dependencies
- Replaced `sqlx` with `tokio-postgres` 0.7 + `deadpool-postgres` 0.14 for PostgreSQL backend (eliminates `libsqlite3-sys` conflict).
- `pgvector` features changed from `["sqlx"]` to `["postgres"]`.
- Removed unused `sqlx-sqlite` patch from workspace `[patch.crates-io]`.

### Removed

#### Storage (`brainwires-storage`)
- `clients/` module (7 files + tests) — replaced by `databases/`.
- `stores/backend.rs` — split into `databases/traits.rs` + `databases/types.rs`.
- `stores/backends/` — merged into `databases/lance/`.
- `stores/lance_client.rs` — legacy `LanceClient` replaced by `LanceDatabase`.

---

### Added

#### Agent Network (`brainwires-agent-network`)
- **5-layer protocol stack** for pluggable agent networking: Identity → Transport → Routing → Discovery → Application.
- **Identity layer**: `AgentIdentity`, `AgentCard` (capabilities, protocols, metadata, endpoint), `ProtocolId`, `SigningKey`/`VerifyingKey` (ChaCha20-Poly1305 with SHA-256 key derivation).
- **Transport layer**: `Transport` trait with 5 implementations:
  - `IpcTransport` (feature `ipc-transport`) — Unix-socket with optional ChaCha20-Poly1305 encryption.
  - `RemoteTransport` (feature `remote-transport`) — HTTP POST with `tokio::broadcast` receive channel.
  - `TcpTransport` (feature `tcp-transport`) — length-prefixed JSON over TCP with Nagle disabled.
  - `PubSubTransport` (feature `pubsub-transport`) — in-process topic-based messaging via `tokio::broadcast`.
  - `A2aTransport` (feature `a2a-transport`) — A2A protocol via `brainwires-a2a` client.
- **Routing layer**: `Router` trait with `DirectRouter`, `BroadcastRouter`, `ContentRouter`, and `PeerTable` for peer/topic tracking.
- **Discovery layer**: `Discovery` trait with `ManualDiscovery` (in-memory) and `RegistryDiscovery` (HTTP REST, feature `registry-discovery`).
- **Application layer**: `NetworkManager` and `NetworkManagerBuilder` tying all layers together with `send()`, `broadcast()`, and event subscription.
- Core network types: `MessageEnvelope`, `MessageTarget` (Direct/Broadcast/Topic), `Payload` (Json/Binary/Text), `NetworkEvent`, `NetworkError`, `TransportType`, `ConnectionState`.
- New feature flags: `ipc-transport` (default), `remote-transport` (default), `tcp-transport`, `pubsub-transport`, `a2a-transport`, `mesh` (includes `tcp-transport`), `registry-discovery`, `full`.
- 74 new tests across all protocol stack layers.

### Changed

#### Agent Network (`brainwires-agent-network`)
- Renamed `src/transport.rs` (MCP-specific `ServerTransport`) to `src/mcp_transport.rs` to avoid conflict with the new `transport/` module. `ServerTransport` and `StdioServerTransport` are still re-exported from the crate root.
- Updated `mesh/` module with deprecation notices pointing to the new protocol-layer equivalents.
- Default features now include `ipc-transport` and `remote-transport`.

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

#### Hardware (`brainwires-hardware`)
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

#### Hardware (`brainwires-hardware`)
- Proper error handling for non-UTF-8 model paths in `WhisperStt`.

#### RAG (`brainwires-rag`)
- Fixed use-after-move of `symbol_name` in `find_references`.
- Git search results now return the actual commit date instead of hardcoded `0`.
- Dirty flag is now cleared immediately after embeddings + cache are flushed to disk in both full and incremental indexing paths.

[0.4.1]: https://github.com/Brainwires/brainwires-framework/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Brainwires/brainwires-framework/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Brainwires/brainwires-framework/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Brainwires/brainwires-framework/releases/tag/v0.2.0
[0.1.0]: Untagged initial release
