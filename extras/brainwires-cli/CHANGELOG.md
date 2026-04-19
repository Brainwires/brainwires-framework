# Changelog

All notable changes to Brainwires CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added (agents)
- **Worktree isolation primitive** — new `src/agent/worktree.rs` exposes a
  `WorktreeGuard` that creates a scratch `git worktree` under
  `~/.brainwires/worktrees/<uuid>/` and cleans up on drop. Agents that want
  isolation can spawn with their working directory pointed at the guard's
  path. Full `Agent({isolation: "worktree"})` wiring into the agent-spawn
  lifecycle is still TBD — this ships the RAII primitive so that future
  pass has something to build on.
- `prune_worktree_orphans()` helper for startup GC of leaked worktrees.
- `BRAINWIRES_HOME` env var now overrides `dot_brainwires_dir()` —
  parallels `BRAINWIRES_MEMORY_ROOT`; useful for tests and non-standard
  layouts.

### Added (skills)
- **Subagent + Script execution modes** land for real. Subagent reuses
  the framework's `prepare_subagent` system prompt and runs inside the
  current agent context with tool scoping (true TaskAgent isolation
  still routes through `/spawn`). Script mode injects an explicit
  "execute this Rhai script via `execute_script`" instruction and
  guarantees `execute_script` is present in the scoped tool list.
- **SkillRouter auto-suggest** — after every user message, the TUI
  runs a keyword match against discovered skills and emits a console
  hint like `💡 Skill 'code-review' may help — invoke with /code-review`
  when confidence ≥ 0.75. Non-intrusive; the user still has to invoke.
- **Skill tool scope extended to MDAP mode** — `pending_skill_tool_scope`
  now filters the `AgentContext.tools` passed to `OrchestratorAgent::execute_mdap`.
  IPC mode still can't enforce scope over the wire (the remote session
  owns its own ToolExecutor); it now surfaces an explicit one-line
  notice instead of a silent clear.

### Added (tui)
- **Six global keybindings remappable**, up from two — added
  `task_viewer`, `reverse_search`, `sub_agent_viewer`, `file_explorer`
  to the `settings.keybindings.global` table. Per-mode dispatch swapped
  from `event.is_<action>()` → `self.keybindings.matches("<action>", &event)`
  at all four call sites.

### Added (tests)
- `EnvVarGuard` RAII helper in `src/utils/mod.rs::test_util` restores the
  previous value of an env var on drop, preventing cross-test leakage
  from tests that had to mutate `$HOME` / `BRAINWIRES_MEMORY_ROOT`.

### Added (skills)
- **`/skill <name>` honors `allowed_tools`** — the invoked skill's body is
  injected as a **system** message (was user-role) and the next AI turn's
  tool set is filtered to the skill's declared `allowed_tools`. Scope is
  one-shot and cleared on the next response. IPC and MDAP paths log a
  warning and clear the scope unfiltered — follow-up.
- **Execution modes**: `Inline` fully implemented; `Subagent`/`Script`
  log a notice + fall back to Inline so the skill still has effect
  (full Subagent spawn / Script orchestration are future passes).
- **Level-3 resources** (`scripts/`, `references/`, `assets/`) now
  appear in `/skill:show` via `SkillRegistry::get_resources`.

### Added (tui)
- **Interactive `/shell`** — drop into a live `bash` (or `$SHELL`) from
  inside the TUI. Terminal is fully handed over (raw mode off, alt-screen
  off, mouse capture off) for the shell's lifetime, then restored on
  return. Unix-only; Windows prints a clear "not supported" message.
- **Remappable global keybindings** — `settings.keybindings.global` lets
  users rebind the top-level global shortcuts (`console_view`,
  `plan_mode_toggle`). Per-mode hardcoded keys still work. Defaults
  match the current behavior exactly so unconfigured users see no
  change.

### Changed (refactor)
- `src/tui/app/message_processing/command_handler.rs` (2456 lines) split
  into a directory module with one file per topic: `mod.rs` (dispatch +
  mdap + context + tools-mode), `knowledge.rs`, `profile.rs`,
  `agents.rs`, `skills.rs`. Zero behavior change.
- Cleaned clippy warnings from passes 3–5: nested `if let` →  `&& let`,
  `.min().max()` → `.clamp()`, counter loop → `enumerate()`,
  `#![allow(clippy::await_holding_lock)]` on the memory tests module
  (the env-var lock is process-global and held intentionally).

### Added (docs)
- `docs/harness/settings.md` — new "Keybindings" section covering action
  names, key-spec grammar, and an example.

### Added (settings)
- **Layered `settings.json`** — new user/project/local `settings.json` merge, separate from `config.json`. Sources (later wins for scalars, arrays concatenate): `~/.brainwires/settings.json` → `~/.claude/settings.json` (migrator compat) → `<project>/.brainwires/settings.json` → `<project>/.brainwires/settings.local.json`.
- **Tool-specific permissions** — Claude-Code-shaped rules under `settings.permissions.allow/deny/ask` (`Bash(ls:*)`, `Edit(src/**/*.rs)`, `mcp__server__tool`). `deny` overrides everything including `PermissionMode::Full`.
- `SettingsManager::load(cwd)` + `PermissionMatcher` with short-name aliases (`Bash` → `execute_command`, etc.).

### Added (hooks)
- **Lifecycle hooks** — `PreToolUse` / `PostToolUse` / `UserPromptSubmit` / `Stop` shell commands configured under `settings.hooks`. Exit 0 = continue, 2 = block with stderr feedback, other non-zero = soft error. Default 5 s timeout. Event JSON piped to stdin.
- `HookDispatcher` in `src/hooks/mod.rs` wired into `ToolExecutor` (Pre/Post), `chat_loop.rs` (UserPromptSubmit), and `ai_processing.rs` (Stop).

### Added (memory)
- **Per-project auto-memory** — `~/.brainwires/projects/<encoded-cwd>/memory/` with a `MEMORY.md` index plus typed files (`user` / `feedback` / `project` / `reference`). Layout matches Claude Code so existing memory dirs can be symlinked or copied.
- `memory_save` / `memory_delete` / `memory_list` agent tools; index is rewritten on every mutation so orphans prune automatically.
- System prompt injects `## Auto Memory` block; opt out with `BRAINWIRES_DISABLE_AUTO_MEMORY=1`.

### Added (tools)
- **`ask_user_question` tool** — pauses the agent and prompts the user via the TUI question panel (new `AppMode::UserQuestion`) or falls back to `dialoguer::Select`/`Input` in plain CLI mode. Non-TTY returns `{"cancelled": true}` rather than hanging.
- `monitor_read` / `monitor_list` now report `dropped_lines` so agents notice when a chatty background process outran the ring buffer.

### Added (tui)
- **Agent question modal** — one-shot prompts from the `ask_user_question` tool render via the existing question panel; answer routes back over a oneshot channel instead of feeding into the AI conversation.
- **Dynamic skill autocomplete** — typing `/<skill-name>` autocompletes against the discovered `SkillRegistry`; unknown slash commands that match a discovered skill invoke automatically as `/skill <name>`.
- **Custom status line** — `Config.status_line_command` appends the stdout of a shell command to the status bar (1 s cache, 200 ms timeout).

### Changed (config)
- First-run picker now actually defaults to Brainwires when a SaaS session exists (previously hard-coded Anthropic despite the comment).

### Added (docs)
- `docs/harness/settings.md` — full reference for layered settings, permission patterns, hook exit codes + event payloads, memory types, and the `ask_user_question` contract.

### Added (tui)
- **Collapsible Journal Tree** — the Journal view now renders conversation history as an expandable/collapsible tree instead of a flat list. Hierarchy: Turn → UserMessage / AssistantMessage (with ToolCall and SubAgentSpawn children). Navigate with `j`/`k`, expand/collapse with `l`/`h` or `Enter`/`Space`. Classic view is unchanged.
- **Sub-Agent Viewer** (`Ctrl+B`) — new `AppMode::SubAgentViewer` with a 30/70 split layout: left panel shows all running sub-agents with live status icons (⟳ Working, ✓ Completed, ✗ Failed, · Idle); right panel shows the selected agent's journal subtree. When the agent has an IPC socket (`●` badge), you can type and send messages to it directly from the right panel.
- `journal_tree.rs` — new `JournalTreeState` data model: DFS render list, lazy rebuild (`rebuild_if_stale`), cursor tracking, collapse state, `inject_subagent_activity` for live sub-agent data, and full unit tests.
- `sub_agent_viewer.rs` — new UI module for the Sub-Agent Viewer, rendering agent list and detail panels.
- `docs/SESSIONS.md` — new documentation covering session lifecycle, dual-socket architecture (`.pty.sock` vs `.sock`), `ViewerMessage`/`AgentMessage` types, CLI commands, sub-agent sessions, and LanceDB persistence.
- Updated `TUI_KEYBOARD_SHORTCUTS.md` with `Ctrl+B`, Journal tree navigation (`j/k/h/l/Enter/Space/g/G`), and Sub-Agent Viewer keybindings.

### Changed (build)
- Removed stale `[patch.crates-io]` entries for `rustpython-vm`, `rustpython-stdlib`, and `sqlx-sqlite` (dependencies were commented out; patches caused spurious cargo warnings).

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
