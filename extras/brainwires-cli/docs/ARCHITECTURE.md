# Brainwires CLI Architecture

This document describes the high-level architecture of the brainwires-cli application.

## Overview

Brainwires CLI is an AI-powered agentic command-line tool for autonomous coding assistance. It combines multi-agent orchestration, Model Context Protocol (MCP) integration, infinite context memory, and extensive tool execution capabilities.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLI Layer (clap)                               │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────────┐    │
│  │  chat   │ │  auth   │ │ history │ │ attach  │ │    mcp-server       │    │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └──────────┬──────────┘    │
└───────┼──────────┼──────────┼──────────┼────────────────────┼───────────────┘
        │          │          │          │                    │
        v          v          v          v                    v
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Application Core                                  │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────────┐   │
│  │   Agent    │ │  Provider  │ │   Tools    │ │     MCP Server         │   │
│  │   Layer    │ │   Layer    │ │   Layer    │ │       Layer            │   │
│  └─────┬──────┘ └─────┬──────┘ └─────┬──────┘ └───────────┬────────────┘   │
│        │              │              │                    │                │
│        v              v              v                    v                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Storage & Context Layer                      │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐               │   │
│  │  │ LanceDB  │ │ Knowledge│ │ Message  │ │  Config  │               │   │
│  │  │ Storage  │ │  Graph   │ │  Memory  │ │  Store   │               │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Modules

### 1. CLI Layer (`src/cli/`)

Entry point for all user interactions. Handles:
- Command parsing with `clap`
- Multiple chat modes (interactive, TUI, batch, MCP server)
- Output formatting (full, plain, JSON)
- Session management

**Key files:**
- `mod.rs` - Command definitions
- `chat/` - Chat command implementation
- `attach.rs` - Session attachment
- `history.rs` - Conversation history management

### 2. Agent Layer (`src/agents/`)

Multi-agent orchestration system for complex task decomposition:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Agent Orchestration                          │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │ Orchestrator │───>│  TaskAgent   │───>│  TaskAgent   │       │
│  │   (Parent)   │    │  (Worker 1)  │    │  (Worker 2)  │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│         │                   │                   │                │
│         v                   v                   v                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Communication Hub (Pub/Sub)                │    │
│  └─────────────────────────────────────────────────────────┘    │
│         │                   │                   │                │
│         v                   v                   v                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              File Lock Manager (R/W Locks)              │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

**Components:**
- `task_agent.rs` - Autonomous task execution agents
- `orchestrator.rs` - Parent agent that spawns and coordinates workers
- `communication.rs` - Message hub for agent coordination
- `file_locks.rs` - Read/write file locking
- `validation_loop.rs` - Pre-completion validation checks
- `pool.rs` - Agent lifecycle management

**MDAP System (`src/mdap/`):**
Multi-Dimensional Adaptive Planning for complex tasks:
- Voting mechanism (k=3-7 agents vote on decisions)
- Task decomposition into microagent subtasks
- Presets: default, high_reliability, cost_optimized

### 3. Provider Layer (`src/providers/`)

Unified interface for AI model providers:

```
┌──────────────────────────────────────────────────────────────┐
│                    Provider Trait                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ async fn complete(&self, messages, config) -> Stream │   │
│  │ fn model_info(&self) -> ModelInfo                    │   │
│  │ fn supports_tools(&self) -> bool                     │   │
│  └──────────────────────────────────────────────────────┘   │
│                            │                                 │
│         ┌──────────────────┼──────────────────┐             │
│         v                  v                  v             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Anthropic   │  │   OpenAI     │  │   Google     │      │
│  │  Provider    │  │   Provider   │  │   Provider   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │             │
│         v                  v                  v             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Ollama    │  │    Groq      │  │   Mistral    │      │
│  │   Provider   │  │   Provider   │  │   Provider   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└──────────────────────────────────────────────────────────────┘
```

**Features:**
- Streaming responses via async streams
- Model capability detection
- Context window management
- Cost tracking per provider

### 4. Tool Layer (`src/tools/`)

Extensible tool execution system:

| Tool Category | Tools | Description |
|--------------|-------|-------------|
| File Operations | `read_file`, `write_file`, `edit_file`, `list_directory` | File system manipulation |
| Shell | `bash` | Command execution |
| Git | `git_status`, `git_commit`, `git_diff` | Version control |
| Web | `web_fetch`, `web_search` | HTTP requests and search |
| Code Search | `query_codebase` | Semantic code search |
| Validation | `check_duplicates`, `verify_build`, `check_syntax` | Code quality checks |

**Key files:**
- `executor.rs` - Tool dispatch and execution
- `registry.rs` - Tool registration
- `error.rs` - Error classification and retry strategies

### 5. MCP Layer (`src/mcp/`, `src/mcp_server/`)

Model Context Protocol implementation:

```
┌──────────────────────────────────────────────────────────────────┐
│                        MCP Client                                │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Connects to external MCP servers                        │   │
│  │  Uses their tools in agent workflows                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│                        MCP Server                                │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Exposes CLI as MCP server (--mcp-server flag)           │   │
│  │  Agent management: spawn, list, status, stop, await      │   │
│  │  File lock tools: pool_stats, file_locks                 │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 6. Storage Layer (`src/storage/`)

Persistent storage for conversations and embeddings:

```
┌──────────────────────────────────────────────────────────────┐
│                   LanceDB Vector Storage                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Tiered Memory:                                      │    │
│  │  - Hot: Recent messages (in-memory)                  │    │
│  │  - Warm: Session messages (indexed)                  │    │
│  │  - Cold: Archived messages (compressed)              │    │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                 │
│  ┌─────────────────────────v─────────────────────────────┐  │
│  │  Semantic Search:                                      │  │
│  │  - FastEmbed embeddings (all-MiniLM-L6-v2)            │  │
│  │  - LRU cache for embedding memoization                │  │
│  │  - Query by content similarity                        │  │
│  └─────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
```

### 7. Knowledge Layer (framework: `brainwires-prompting` crate, `knowledge` feature)

Entity extraction and context management:

- **Entity Extraction**: Extracts files, functions, types, variables from messages
- **Relationship Graph**: Tracks co-occurrence, containment, dependencies
- **Smart Context Injection**: Retrieves relevant past messages when needed
- **Infinite Context**: Never lose important information from earlier in conversation

### 8. Auth Layer (`src/auth/`)

Authentication and session management:

- Brainwires Studio backend authentication
- Session token storage
- Direct provider API key support
- Secure keyring storage via `keyring` crate

### 9. Prompting Layer (`src/prompting/`)

Adaptive prompting system with 15+ techniques for optimizing LLM interactions:

```
┌──────────────────────────────────────────────────────────────┐
│                  Adaptive Prompting System                    │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Techniques:                                            │  │
│  │  - Chain of Thought (CoT)                               │  │
│  │  - Tree of Thoughts (ToT)                               │  │
│  │  - Self-Consistency                                     │  │
│  │  - ReAct (Reasoning + Acting)                           │  │
│  │  - Meta-Prompting                                       │  │
│  │  - Contrastive Prompting                                │  │
│  │  - Analogical Reasoning                                 │  │
│  │  - Structured Output                                    │  │
│  │  - Zero/Few-Shot Learning                               │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│  ┌────────────────────────v────────────────────────────────┐│
│  │  Best Knowledge Synthesis (BKS):                         ││
│  │  - Clusters techniques by task type                      ││
│  │  - Promotes successful patterns                          ││
│  │  - Adapts based on performance metrics                   ││
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 10. SEAL Layer (`src/seal/`)

Self-Evolving Adaptive Learning system for continuous improvement:

- **Pattern Recognition**: Learns from successful task completions
- **Strategy Evolution**: Adapts approaches based on outcomes
- **Knowledge Integration**: Incorporates domain-specific learnings
- **Performance Tracking**: Monitors and optimizes execution patterns

### 11. Session Layer (`src/session/`)

PTY-based session persistence for long-running tasks:

```
┌──────────────────────────────────────────────────────────────┐
│                   Session Management                          │
│  ┌────────────────────┐    ┌────────────────────┐           │
│  │   SessionServer    │    │   SessionClient    │           │
│  │   (Background)     │<-->│   (Attach/Detach)  │           │
│  └────────────────────┘    └────────────────────┘           │
│            │                                                 │
│  ┌─────────v──────────────────────────────────────────────┐ │
│  │  Features:                                              │ │
│  │  - Detach/reattach to running sessions                  │ │
│  │  - PTY multiplexing for parallel tasks                  │ │
│  │  - State persistence across disconnects                 │ │
│  │  - Session recovery after crashes                       │ │
│  └─────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
```

### 12. IPC Layer (`src/ipc/`)

Inter-process communication for agent coordination:

- Unix domain sockets for local communication
- Message serialization/deserialization
- Agent metadata exchange
- Session discovery and management

### 13. Approval Layer (`src/approval/`)

Tool approval modal for human-in-the-loop workflows:

- **Auto/Ask/Reject Modes**: Configurable per tool
- **Approval History**: Track and learn from decisions
- **Bulk Approval**: Handle multiple similar requests
- **Timeout Handling**: Default actions for unattended operation

### 14. Local Inference Layer (`src/local_inference/`)

3-tier local ML inference strategy:

```
┌──────────────────────────────────────────────────────────────┐
│                  Local Inference Strategy                     │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Tier 1: In-Process (FastEmbed)                        │  │
│  │  - Embeddings for semantic search                      │  │
│  │  - Fast, no network dependency                         │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Tier 2: Local Server (Ollama)                         │  │
│  │  - Full model inference                                │  │
│  │  - Offline-capable                                     │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Tier 3: API Fallback (Cloud Providers)                │  │
│  │  - High-capability models                              │  │
│  │  - Automatic failover                                  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 15. Skills Layer (`src/skills/`)

Agent skills system for reusable capabilities:

- **Skill Registry**: Catalog of available skills
- **Skill Composition**: Combine skills for complex tasks
- **Dynamic Loading**: Load skills on demand
- **Skill Versioning**: Track and manage skill updates

### 16. Permissions Layer (`src/permissions/`)

Capability-based access control system:

```
┌──────────────────────────────────────────────────────────────┐
│                     Permission System                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  AgentCapabilities:                                     │  │
│  │  - Filesystem (read/write paths, denied paths)          │  │
│  │  - Tools (allowed categories, denied tools)             │  │
│  │  - Network (domains, rate limits)                       │  │
│  │  - Git (operations, protected branches)                 │  │
│  │  - Spawning (max children, max depth)                   │  │
│  │  - Quotas (time, tokens, tool calls)                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│  ┌────────────────────────v────────────────────────────────┐│
│  │  Profiles:                                               ││
│  │  - read_only: Read operations only                       ││
│  │  - standard_dev: Normal development workflow             ││
│  │  - full_access: All capabilities enabled                 ││
│  └────────────────────────────────────────────────────────┘ │
│                            │                                  │
│  ┌────────────────────────v────────────────────────────────┐│
│  │  Policy Engine:                                          ││
│  │  - Rule-based access control                             ││
│  │  - Priority-ordered evaluation                           ││
│  │  - Audit logging                                         ││
│  └────────────────────────────────────────────────────────┘ │
│                            │                                  │
│  ┌────────────────────────v────────────────────────────────┐│
│  │  Trust System:                                           ││
│  │  - Dynamic trust scoring (0.0-1.0)                       ││
│  │  - Violation penalties                                   ││
│  │  - Trust level derivation (Untrusted → System)           ││
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 17. Remote Layer (`src/remote.rs`, framework: `brainwires-relay` crate)

Remote relay connector for external orchestration:

```
┌──────────────────────────────────────────────────────────────┐
│                   Remote Control Bridge                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  WebSocket Protocol:                                    │  │
│  │  - Bidirectional message streaming                      │  │
│  │  - Heartbeat/keep-alive                                 │  │
│  │  - Reconnection with state sync                         │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  HTTP REST API:                                         │  │
│  │  - Status queries                                       │  │
│  │  - Command submission                                   │  │
│  │  - Result retrieval                                     │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Telemetry:                                             │  │
│  │  - Resource usage metrics                               │  │
│  │  - Task progress updates                                │  │
│  │  - Error reporting                                      │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 18. Commands Layer (`src/commands/`)

Slash command system for quick actions:

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/clear` | Clear conversation history |
| `/mode` | Switch between modes (chat, code, etc.) |
| `/model` | Change the active model |
| `/project:index` | Index codebase for RAG |
| `/project:query` | Query indexed codebase |
| `/project:stats` | Show RAG statistics |

**Custom Commands**: Users can define custom commands in configuration.

### 19. TUI Layer (`src/tui/`)

Terminal user interface using `ratatui`:

```
┌──────────────────────────────────────────────────────────────┐
│                     TUI Architecture                          │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  App State (src/tui/app/):                              │  │
│  │  - Conversation history                                 │  │
│  │  - Tool execution status                                │  │
│  │  - Input handling                                       │  │
│  │  - Modal dialogs                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Event Handlers (src/tui/app/events/):                  │  │
│  │  - Core event dispatch                                  │  │
│  │  - Viewer handlers (console, shell, fullscreen)         │  │
│  │  - Picker handlers (session, tool, file)                │  │
│  │  - Dialog handlers (help, suspend, exit, approval)      │  │
│  │  - Modal handlers (nano editor, git SCM)                │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Components:                                            │  │
│  │  - File explorer with tree view                         │  │
│  │  - Built-in nano-style editor                           │  │
│  │  - Git SCM panel                                        │  │
│  │  - Find/replace dialog                                  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## Data Flow

### Chat Message Flow

```
User Input
    │
    v
┌──────────────┐
│  CLI Parser  │
└──────┬───────┘
       │
       v
┌──────────────┐    ┌──────────────┐
│   Context    │───>│   Provider   │
│   Builder    │    │   (Stream)   │
└──────────────┘    └──────┬───────┘
                          │
                          v
                   ┌──────────────┐
                   │ Tool Calls?  │
                   └──────┬───────┘
                          │
           ┌──────────────┴──────────────┐
           v                             v
    ┌──────────────┐             ┌──────────────┐
    │     No       │             │     Yes      │
    │  Stream out  │             │ Execute Tool │
    └──────────────┘             └──────┬───────┘
                                       │
                                       v
                                ┌──────────────┐
                                │ Tool Result  │
                                │  to Context  │
                                └──────┬───────┘
                                       │
                                       └──> Loop back to Provider
```

### Agent Spawning Flow

```
MCP Client Request
    │
    v
┌──────────────────┐
│  MCP Server      │
│  (agent_spawn)   │
└────────┬─────────┘
         │
         v
┌──────────────────┐
│  Create TaskAgent│
│  with Config     │
└────────┬─────────┘
         │
         v
┌──────────────────┐
│  Register with   │
│  Agent Pool      │
└────────┬─────────┘
         │
         v
┌──────────────────┐    ┌──────────────────┐
│  Agent Execute   │───>│  Tool Execution  │
│  Loop            │<───│  with Locks      │
└────────┬─────────┘    └──────────────────┘
         │
         v
┌──────────────────┐
│  Validation Loop │
│  (if enabled)    │
└────────┬─────────┘
         │
         v
┌──────────────────┐
│  Return Result   │
│  to MCP Client   │
└──────────────────┘
```

## Module Dependencies

```
                              ┌─────────────┐
                              │    cli      │
                              └──────┬──────┘
                                     │
       ┌─────────────┬───────────────┼───────────────┬─────────────┐
       v             v               v               v             v
┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐
│   agent    │ │  commands  │ │  providers │ │    tui     │ │   remote   │
└──────┬─────┘ └────────────┘ └──────┬─────┘ └────────────┘ └────────────┘
       │                             │
       v                             v
┌────────────┐               ┌────────────┐
│   agents   │               │   tools    │
└──────┬─────┘               └──────┬─────┘
       │                            │
       ├────────────┬───────────────┤
       │            │               │
       v            v               v
┌────────────┐ ┌────────────┐ ┌────────────┐
│    mdap    │ │ permissions│ │  approval  │
└────────────┘ └────────────┘ └────────────┘
       │            │               │
       └────────────┴───────────────┘
                    │
                    v
            ┌────────────┐
            │  storage   │
            └──────┬─────┘
                   │
    ┌──────────────┼──────────────┐
    v              v              v
┌────────┐   ┌────────────┐  ┌────────────┐
│knowledge│  │   config   │  │   session  │
└────────┘   └────────────┘  └────────────┘
    │                             │
    v                             v
┌────────────┐               ┌────────────┐
│    seal    │               │    ipc     │
└────────────┘               └────────────┘
```

### Module Descriptions

| Module | Purpose |
|--------|---------|
| `cli` | Entry point, command parsing |
| `commands` | Slash command system |
| `providers` | AI model provider abstraction |
| `tui` | Terminal user interface |
| `remote` | Remote relay connector |
| `agent` | Background agent process |
| `agents` | Multi-agent orchestration |
| `tools` | Tool execution system |
| `mdap` | Multi-dimensional adaptive planning |
| `permissions` | Capability-based access control |
| `approval` | Human-in-the-loop approval |
| `storage` | Persistent storage (LanceDB) |
| `knowledge` | Entity extraction, context graphs (now in `brainwires-prompting` crate) |
| `config` | Configuration management |
| `session` | PTY session persistence |
| `seal` | Self-evolving adaptive learning |
| `ipc` | Inter-process communication |
| `prompting` | Adaptive prompting techniques |
| `skills` | Reusable agent capabilities |
| `local_inference` | Local ML inference |

## Error Handling

The codebase uses a unified error type (`AppError`) with categorization:

```rust
pub enum AppError {
    // Agent/task errors
    Agent(String),
    Mdap(String),

    // Tool execution
    Tool(String),
    ToolNotFound(String),
    ToolTimeout { tool: String, timeout_secs: u64 },

    // Storage/persistence
    Storage(String),

    // Authentication
    Auth(String),
    AuthRequired(String),

    // Configuration
    Config(String),
    ConfigMissing(String),

    // Network/API
    Provider(String),
    ProviderRateLimit { provider: String, retry_after_secs: u64 },
    Connection(String),
    Timeout(String),

    // Permissions
    PermissionDenied(String),

    // Other
    Internal(String),
    FileNotFound(String),
    Io(String),
    Cancelled,
}
```

Errors support:
- `is_retryable()` - Whether operation can be retried
- `is_auth_error()` - Authentication-related errors
- `retry_after_secs()` - Suggested retry delay for rate limits

## Configuration

Configuration is stored in `~/.brainwires/`:

| File | Purpose |
|------|---------|
| `config.json` | User preferences, default model, permissions |
| `session.json` | Authentication tokens |
| `mcp_servers.json` | Registered MCP servers |

API keys are stored securely in the system keyring.

## Performance Considerations

1. **LRU Cache for Embeddings**: Avoids re-embedding identical messages
2. **Async Streams**: Real-time response streaming without buffering
3. **File Locks**: Minimal critical section duration
4. **Tiered Storage**: Hot/warm/cold memory tiers for efficiency
5. **Parallel Tool Execution**: When tools are independent

## Testing Strategy

- **Unit Tests**: Core logic in each module
- **Property Tests**: Invariants with random inputs (`proptest`)
- **Concurrent Tests**: Multi-agent coordination, lock contention
- **Integration Tests**: End-to-end MCP server workflows

See `tests/` directory for test files.
