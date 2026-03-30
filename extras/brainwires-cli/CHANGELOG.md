# Changelog

All notable changes to Brainwires CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added (storage)
- `MySqlDatabase` — real MySQL/MariaDB backend via `mysql_async` (implements `StorageBackend`)
- `SurrealDatabase` — real SurrealDB backend via official `surrealdb` SDK (implements both `StorageBackend` + `VectorDatabase` with native MTREE vector search)
- `PostgresDatabase` now implements `StorageBackend` in addition to `VectorDatabase`
- Restored `mysql-backend` and `surrealdb-backend` feature flags with actual dependencies

### Removed (storage)
- Removed previous `todo!()` stub implementations (replaced with real code)

### 2026-01-12

- feat: Add PKS integration for implicit fact detection and behavioral inference (`5f964bc`)

### 2026-01-04

- refactor: migrate tui-extension to ratatui-interact submodule (`fb313cc`)

### 2026-01-01

- chore: Update subproject commit for thalora-web-browser (`3318325`)

### 2025-12-31

- feat: Integrate local LLM enhancements across various modules (`8e532f3`)

- feat: Implement Skill Registry and Router for managing and activating skills (`70a339a`)
- feat: Add local LLM support and skill execution framework (`1f2ea7b`)

- feat(local_llm): Implement local LLM provider with model registry and inference capabilities (`d5283ba`)
- feat: Add project commands for symbol definition, references, and call graph (`afd588e`)
- refactor: Update local LLM provider to use new inference method and improve thread safety (`7d91b12`)

- feat: Add Thalora Headless Browser TODO Completion Plan (`aa84db2`)

- fix: Clarify Thalora section in IDEAS.md regarding lock files updates (`9df989a`)

- feat: Add Thalora build output example to IDEAS.md (`e399124`)

- fix: Remove broken message response streaming note and TUI search issue from IDEAS.md (`594ca4e`)
- Implement priority command queue with retry logic and telemetry metrics (`58e639e`)
- feat: Add command priority and retry policy, implement search highlighting in TUI (`e6d84a7`)

- feat: Implement remote command handling with security policies (`9f2e706`)

### 2025-12-30

- feat: Update subproject commit reference in tool-orchestrator (`ab8865d`)

- feat: Add note on Bridge to Bridge communications for multi-agent coordination in IDEAS.md (`1e86069`)

- feat: Enhance focus cycling and add status bar visibility management for better navigation (`6876b5c`)

### 2025-12-29

- feat: Update subproject commits and enhance IDEAS.md with agent skills documentation (`7981857`)

- feat: Enhance focus cycling in event handling for better navigation (`0207eef`)

- feat: Implement transition_to_normal_after_streaming method to manage app mode transitions (`e96c1ba`)

- feat: Add conversation printing on exit when preserve_chat_on_exit is enabled (`42f4b41`)

- feat: Improve event handler termination to ensure proper cleanup of EventStream (`dd93e03`)

- feat: Implement exit dialog with Ctrl+C handling and UI integration (`0e8515f`)

- feat: Add CTRL-C handler and improve message sending for agent subscriptions (`9b55b40`)

### 2025-12-27

- feat: Enhance message handling to prevent duplicates and improve session management (`0d7e828`)

- feat: Add message handling for user input and assistant responses in App (`b0634bf`)

- feat: Enhance heartbeat and registration to include agent details and immediate sync trigger (`4dff59b`)

- feat: Add initial register message handling in RealtimeClient upon subscription (`30c7c51`)

### 2025-12-26

- feat: Implement graceful shutdown mechanism in RemoteBridge and manager (`c978f80`)

- feat: Update message handling to differentiate user and assistant messages in RemoteBridge (`fd8b6f4`)

- feat: Implement history sync request for agents in RemoteBridge (`dc1e39c`)

- feat: Add initial heartbeat delay in RemoteBridge to ensure frontend subscription (`73e5aac`)

- feat: Implement command processing for Realtime commands in RemoteBridge and RealtimeClient (`563d022`)

- feat: Add background session support for TUI and enhance session management (`ed82528`)

### 2025-12-25

- feat(ipc): implement ChaCha20-Poly1305 encryption for IPC messages (`cae23d4`)

### 2025-12-24

- feat: Implement session token management for secure IPC authentication (`2b780fe`)
- feat: Implement secure API key storage using system keyring and update session management (`5a8a4d3`)
- feat: Enhance directory management with secure permissions for data, config, and cache directories (`eaea032`)

- feat: Implement Supabase Realtime WebSocket client for backend communication (`327367c`)

- Refactor: Remove LanceDB implementation and related tests (`f8fa74a`)

### 2025-12-19

- refactor(remote): use immediate POST for stream data instead of heartbeat queue (`2f9f645`)

- feat(cli): add kill command and fix exit behavior (`0209c01`)

- fix(remote): include queued messages in heartbeat (`68e0b9e`)

- fix(sessions): allow exit command to cleanup stale sessions (`e935805`)

- feat: Enhance IPC integration and session management in TUI (`5d92f11`)
- feat(tui): enhance session management and background support in TUI documentation (`139f674`)
- feat(remote): stream agent output and send history on subscribe (`7bbd1bb`)

### 2025-12-17

- refactor: reorganize ideas for clarity and remove redundant entries (`0cc9267`)

- refactor: reorder test ideas for clarity and focus on remote control functionality (`c97f5e4`)

- feat(remote): add remote control functionality and documentation (`f2c1534`)

- feat(remote): add remote bridge commands and management functionality (`4d7fa8c`)

- feat(remote): implement remote control bridge with WebSocket communication (`a6d181c`)

- feat: implement graceful viewer disconnection and agent depth limit (`bfd5707`)

- feat: implement approval system for tool execution (`d8af2c3`)

- feat: add hotkey configuration dialog (`0c439a5`)

- feat: Remove index.html file as part of project restructuring (`dcc8779`)

- feat: Enhance tool execution and system prompt handling (`788a1a8`)

- feat: Implement multi-agent system commands and locking mechanism (`207c5b0`)

### 2025-12-16

- feat: Implement multi-agent communication with ListAgents and SpawnAgent requests (`cbdbb48`)

- feat: Add is_pty_session flag to App struct and adjust scroll handling for PTY mode (`8d7ede1`)

- feat: Optimize scroll handling in run_app function and remove debug logging (`3e2da60`)

- feat: Adjust scroll calculation for conversation view and add debug logging for line count after re-render (`112b3b5`)

- feat: Implement PTY session handling and AI response resumption logic (`7ca6545`)
- feat: Add pending scroll to bottom functionality after loading a session (`1fe35fa`)
- feat: Enhance conversation creation and management with message count tracking (`ac9f3f4`)
- feat: Add support for scrolling to bottom on resize in PTY mode and enhance session loading logging (`44d2920`)

### 2025-12-15

- Refactor session management and background process handling (`5801051`)

- feat: Implement Unix Socket IPC utilities for communication between TUI viewer and Agent (`47e85f0`)
- feat: Enhance session management by passing attacher PID for cleanup on exit (`a465d5a`)
- feat: Implement custom SIGHUP handler for silent exit and clean up terminal control logic (`f6bb95b`)

- feat: Add background session management commands and implement session handling (`1f66f16`)

- feat: Implement suspend/background dialog with event handling and UI integration (`3999c64`)

- feat: Update web search tool to prioritize DuckDuckGo and refine engine selection description (`eb10051`)

- Refactor web search tool to use Thalora for headless browsing (`27a5cf6`)

- chore: Update dependencies for thalora-web-browser submodule (`2afdabe`)

- feat: Update dependencies and enhance MCP transport error handling (`64c85fc`)

- feat: Enhance error handling in MCP transport and tool execution results (`cc7498a`)

- feat: Ensure prompt starts on a new line in terminal restoration functions (`a7e0c54`)

### 2025-12-14

- feat: Add ideas for system pause/restart options and SSH MCP server functionality (`083d44c`)

### 2025-12-13

- feat: Remove outdated idea for in-TUI help system (`5c713a6`)
- feat: Implement interactive help dialog with state management and UI rendering (`08ccf11`)

- feat: Implement Personal Knowledge System (PKS) for user-specific facts (`3f7f4e4`)

- Refactor find/replace dialog handling and UI (`336b3c1`)

### 2025-12-12

- feat: Add command line argument for reloading the most recent conversation on startup (`693d51a`)

- Implement Behavioral Knowledge System (BKS) with learning and management features (`6482228`)

- feat: Add idea for monitoring tool action prevention and process state management (`664274a`)

### 2025-12-10

- feat: Implement find and replace functionality with dialog support (`d1405a1`)

- feat: Add conversation view style toggle and implement rendering for journal and classic styles (`50c2d2d`)

- feat: Implement CleanTextWidget for improved text rendering in console and conversation views (`e058809`)

### 2025-12-09

- feat: Update fullscreen toggle keybindings and UI prompts for consistency (`5777842`)

- feat: Add current date to system prompt for enhanced context awareness (`b0ba301`)

- feat: Add full-screen input mode with enhanced event handling and rendering (`8b3fe85`)

- feat: Improve scroll offset calculation for center-tracking cursor behavior (`0438a49`)

- feat: Enhance multiline input handling and prompt history navigation (`4bb16b0`)
- feat: Adjust cursor position to beginning when restoring multiline draft (`25b99b6`)

- feat: Implement proactive output management for BashTool (`fee55c6`)
- feat: Enhance multiline input handling and add toast notifications for user feedback (`160ba9c`)

- feat: Enhance TUI input handling with bracketed paste support and character index operations (`5819133`)

- feat: Implement tool support for MDAP microagents with intent expression and execution management (`e31331d`)

### 2025-12-08

- feat: Add policy engine and audit logger to ToolExecutor for enhanced permission management (`9b33b14`)
- feat: Add remote control capability for TUI from web interface (`4b5f951`)

- Add policy engine and trust factor system for dynamic access control (`13eeb90`)

- Implement comprehensive capability-based permission system for agents (`9098d53`)

- feat: Implement Git Worktree Management for Agent Isolation (`ee50484`)

- feat: Enhance MDAP microagent tool handling; implement tool execution delegation and validation logic (`67d2be2`)

- feat: Add comprehensive Copilot instructions for Brainwires CLI (`61f2e73`)
- fix: Update IDEAS.md with fixes for console mouse mode and buffer overflow issues (`ab1b845`)
- feat: Enhance system prompts and token management for agents; add model-specific output token limits (`d002e38`)

- feat: implement project-specific paths for RAG indexing and cache management (`a8e53db`)
- feat: Implement clarifying questions feature in TUI (`ab3db02`)

- feat: implement a priority-ordered wait queue for resource coordination (`ed724a3`)

- Implement cost tracking system with usage events, model pricing, and budget enforcement (`d1bd404`)

- feat: Enhance error handling and confidence scoring in tool execution (`2ed724f`)

### Added
- **Smart File Context Management**: `FileContextManager` for intelligent large file handling
  - Routes files > 8000 chars through chunking instead of full injection
  - Query-based relevance scoring for chunk selection
  - Tracks files already in context to avoid re-injection
- **Image Analysis Storage**: `ImageStore` for storing and retrieving analyzed images
  - LanceDB-backed with vector embeddings for similarity search
  - Automatic metadata extraction and description storage
- **Project RAG Consolidation**: Moved `crates/project-rag` into `src/rag/` module
  - Single source of truth for RAG functionality
  - Reduced code duplication between modules
  - All imports updated from `project_rag::` to `crate::rag::`
- **Unified Path Utilities**: Consolidated platform path handling
  - Added infallible methods (`project_data_dir()`, `project_cache_dir()`) for serde defaults
  - RAG module now re-exports from `utils::paths` for consistency
  - Added `migrate_from_project_rag()` for legacy data migration
- **Generic RRF Fusion**: Consolidated Reciprocal Rank Fusion algorithm
  - Single generic implementation works with any ID type
  - Removed duplicate implementations from storage and RAG modules
- **Single-Shot Mode** (`--prompt`): Execute a single prompt and exit immediately, perfect for scripting
  - Example: `brainwires chat --prompt "What is 2+2?"`
- **Batch Mode** (`--batch`): Process multiple prompts from stdin, one per line
  - Example: `cat questions.txt | brainwires chat --batch`
- **Quiet Mode** (`-q, --quiet`): Suppress decorative output for clean scripting
  - No welcome banners, spinners, or progress messages
  - Example: `brainwires chat --prompt "Calculate 7*8" --quiet`
- **Multiple Output Formats** (`--format`):
  - `full` (default): Rich formatting with labels and colors
  - `plain`: Just the response text, no decoration
  - `json`: Structured JSON output with metadata
  - Example: `brainwires chat --prompt "Hello" --format=json`
- Comprehensive CLI chat modes documentation in `docs/CLI_CHAT_MODES.md`

### Changed
- **Vector Database**: Removed Qdrant support, LanceDB is now the sole vector backend
  - Simpler deployment (no external service required)
  - Embedded database with consistent performance
- Enhanced `brainwires chat` command with flexible mode routing
- Updated stdin detection to work seamlessly with all chat modes
- Improved error handling in batch mode with per-prompt error reporting

### Fixed
- Spinner and progress indicator handling in non-interactive modes
- Output formatting consistency across different modes

## [0.5.0] - Previous Release

### Added
- TUI mode with full-screen terminal interface
- MCP server mode for stdio protocol
- Interactive chat with conversation management
- Tool execution with visual feedback
- Slash commands for RAG operations

### Features
- Multi-agent architecture
- Authentication with Brainwires Studio
- Rich tool system (file ops, bash, git, web)
- MCP client integration
- Cost tracking
- Planning mode
