# Brainwires CLI - Features & Architecture

> **The Bleeding-Edge AI Agent CLI/TUI**
>
> Implementing state-of-the-art research from arXiv papers for autonomous coding assistance.

**Version**: 0.5.0 | **Language**: Rust (Edition 2024) | **LOC**: ~55,700

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Agent Framework](#core-agent-framework)
3. [Multi-Agent Coordination](#multi-agent-coordination)
4. [SEAL - Self-Evolving Agentic Learning](#seal---self-evolving-agentic-learning)
5. [MDAP - Massively Decomposed Agentic Processes](#mdap---massively-decomposed-agentic-processes)
6. [Infinite Context System](#infinite-context-system)
7. [RAG - Retrieval-Augmented Generation](#rag---retrieval-augmented-generation)
8. [Tool System](#tool-system)
9. [MCP Integration](#mcp-integration)
10. [TUI/CLI Interface](#tuicli-interface)
11. [Storage & Persistence](#storage--persistence)
12. [Roadmap & Research Pipeline](#roadmap--research-pipeline)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           BRAINWIRES CLI                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐     │
│  │   CLI/TUI   │   │  MCP Server │   │ Batch Mode  │   │ Single-Shot │     │
│  │  Interface  │   │    Mode     │   │             │   │    Mode     │     │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘     │
│         │                  │                  │                  │           │
│         └──────────────────┴──────────────────┴──────────────────┘           │
│                                    │                                         │
│  ┌─────────────────────────────────▼─────────────────────────────────────┐  │
│  │                     ORCHESTRATOR AGENT                                 │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │  │
│  │  │    SEAL     │  │    MDAP     │  │    Task     │  │   Entity    │   │  │
│  │  │  Processor  │  │  Executor   │  │   Manager   │  │  Extractor  │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│  ┌─────────────────────────────────▼─────────────────────────────────────┐  │
│  │                        TOOL EXECUTOR                                   │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐   │  │
│  │  │FileOps │ │  Bash  │ │  Git   │ │  Web   │ │CodeExec│ │Semantic│   │  │
│  │  │        │ │        │ │        │ │ Search │ │        │ │ Search │   │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘ └────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│  ┌─────────────────────────────────▼─────────────────────────────────────┐  │
│  │                      STORAGE LAYER (LanceDB)                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │  │
│  │  │Conversation │  │Relationship │  │   Pattern   │  │   Vector    │   │  │
│  │  │   Store     │  │    Graph    │  │    Store    │  │   Search    │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Metrics

| Metric | Value |
|--------|-------|
| Rust Files | 237 |
| Lines of Code | ~55,700 |
| Dependencies | 50+ crates |
| Supported Languages (AST) | 12 |
| Build Targets | 7 platforms |

---

## Core Agent Framework

### Orchestrator Agent

The brain of Brainwires - coordinates all agent activities with a maximum of 25 iterations per task.

```
src/agents/                          # CLI-local files
├── orchestrator.rs      # Main orchestration
├── task_agent.rs        # Individual task execution
├── worker.rs            # Worker agent implementation
└── pool.rs              # Agent lifecycle management

brainwires::agents (framework crate) # Re-exported via src/agents/mod.rs
├── file_locks           # Cross-process synchronization
├── access_control       # Permission management
├── validation_loop      # Pre-completion validation
└── confidence           # Response confidence scoring
```

**Capabilities**:
- Multi-step task decomposition and execution
- Tool execution coordination with approval workflows
- SEAL integration for enhanced context understanding
- Entity extraction and tracking throughout conversations
- Confidence scoring on agent responses

### Agent Pool & Workers

Parallel execution through managed worker pools:

- **Manager**: Coordinates agent lifecycle
- **Pool**: Thread pool for concurrent execution
- **Workers**: Execute individual subtasks
- **Communication**: Inter-agent messaging

---

## Multi-Agent Coordination

> **Based on**: SagaLLM, Multi-Agent Coordination Survey, Hierarchical Multi-Agent Systems Taxonomy (arXiv)

Advanced coordination system for parallel agent operations with conflict resolution, resource management, and fault tolerance.

```
src/agents/
├── saga.rs              # Compensating transactions (rollback)
├── state_model.rs       # Three-State Model (App/Op/Dep)
├── validation_agent.rs  # Pre/post/inter-agent validation
├── optimistic.rs        # Optimistic concurrency
├── contract_net.rs      # Task bidding & allocation
├── market_allocation.rs # Urgency-based resource allocation
├── worktree.rs          # Git worktree isolation
├── wait_queue.rs        # Priority wait queue
├── operation_tracker.rs # Heartbeat-based liveness
├── resource_checker.rs  # Cross-resource conflict detection
└── git_coordination.rs  # Git operation sequencing
```

### Saga Transactions (Compensating Rollback)

When multi-step operations fail mid-way, the system automatically executes compensation actions in reverse order to restore consistency.

```
Agent starts: [Edit file A] → [Edit file B] → [Build] → [Commit]
                   ✓              ✓           ✗ FAIL

Compensation:     ← Restore B    ← Restore A
```

**Compensation Actions**:

| Operation | Compensation |
|-----------|--------------|
| FileWrite | Restore from checkpoint or delete |
| FileEdit | Restore original content |
| GitStage | `git reset HEAD <files>` |
| GitCommit | `git reset --soft HEAD~1` |
| GitBranch | Delete created branch |
| Build | None (idempotent) |

### Three-State Model

Separates concerns into three distinct state domains for better debugging and validation:

| State | Purpose | Tracked Data |
|-------|---------|--------------|
| **Application** | Domain-level resources | Files, artifacts, git state |
| **Operation** | Execution logging | Operation history, timing, agent actions |
| **Dependency** | Resource relationships | Constraint graphs, blocking dependencies |

**Dependency Graph** (petgraph-based):
- `BlockedBy`: A needs B to complete first
- `Produces`: Completing A makes B available
- `ConflictsWith`: A and B cannot run concurrently
- `Reads`/`Writes`: Data flow relationships

### Validation Agent

Independent validation at three stages:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Pre-Validate │ → │   Execute    │ → │ Post-Validate │
│   (preconditions) │     │   Operation  │     │   (postconditions) │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                    ┌───────▼───────┐
                    │ Inter-Agent   │
                    │   Validation  │
                    │  (conflicts)  │
                    └───────────────┘
```

**Built-in Rules**:
- `file_exists_for_edit`: File must exist before editing
- `no_conflicting_locks`: No other agent holds conflicting lock
- `artifacts_invalidated_after_edit`: Build artifacts marked invalid after source edit
- `no_deadlock`: Resource acquisition must not cause deadlock
- `git_coordination`: Git operations must not conflict across agents

### Optimistic Concurrency

For read-heavy workloads, allows parallel execution with conflict detection at commit time:

```rust
// Agent A reads file (version 5)
let token_a = controller.begin_optimistic("agent-a", "file.rs").await;

// Agent B also reads file (version 5)
let token_b = controller.begin_optimistic("agent-b", "file.rs").await;

// Agent A commits first (version becomes 6)
controller.commit_optimistic(token_a, "hash-a").await?; // ✓ Success

// Agent B tries to commit (expected version 5, actual 6)
controller.commit_optimistic(token_b, "hash-b").await?; // ✗ Conflict!
```

**Resolution Strategies**:

| Strategy | Behavior |
|----------|----------|
| LastWriterWins | Overwrite silently |
| FirstWriterWins | Reject later commits |
| Merge | Line-by-line or JSON merge |
| Escalate | Notify user/orchestrator |

### Contract-Net Protocol (Task Bidding)

Agents bid on tasks based on capability and availability:

```
┌───────────┐    Announce Task    ┌─────────┐
│  Manager  │ ─────────────────→ │ Agent A │
│           │                     │ Agent B │
│           │                     │ Agent C │
│           │ ←───────────────── │         │
│           │     Submit Bids     │         │
│           │                     │         │
│           │ ─────────────────→ │ Agent B │
│           │   Award Contract    │ (Winner)│
└───────────┘                     └─────────┘
```

**Bid Evaluation Strategies**:
- `HighestScore`: Best capability match × availability
- `FastestCompletion`: Lowest estimated duration
- `LoadBalancing`: Least loaded agent wins
- `Custom`: User-defined evaluation function

### Market-Based Resource Allocation

Dynamic priority bidding with urgency scoring:

```
Urgency Multiplier = base × factors

Factors:
├── User waiting?     → ×2.0
├── Deadline < 1min?  → ×3.0
├── Critical path?    → ×1.5
└── Holding resources → ×(1.0 + 0.2×count)

Max multiplier: 10.0
```

**Budget System**:
- Each agent has a budget that replenishes over time
- Higher urgency = higher bid = faster access
- Prevents starvation through budget replenishment
- Supports first-price and second-price auctions

### Git Worktree Isolation

Agents work in isolated git worktrees to prevent conflicts:

```
project/
├── .git/                      # Main repository
├── src/                       # Main working tree
└── .brainwires/worktrees/
    ├── agent-abc123/          # Agent A's isolated copy
    │   └── src/
    └── agent-def456/          # Agent B's isolated copy
        └── src/
```

**Features**:
- Automatic worktree creation per agent
- Branch isolation (each agent on own branch)
- Merge coordination back to main
- Automatic cleanup on agent termination

### Communication Protocol

24 new message types for coordination:

| Category | Messages |
|----------|----------|
| **Saga** | SagaStarted, SagaStepCompleted, SagaCompleted, SagaCompensating, SagaCompensated, SagaFailed |
| **Contract-Net** | TaskAnnouncement, TaskBid, TaskAwarded, TaskAccepted, TaskDeclined, TaskCompleted |
| **Market** | ResourceBid, BidAccepted, BidRejected, ResourceAllocated, ResourceReleased |
| **Worktree** | WorktreeCreated, WorktreeDeleted, WorktreeReady, MergeRequested, MergeCompleted, MergeConflict |
| **Validation** | ValidationRequest, ValidationResult |
| **Optimistic** | OptimisticConflict |

### Liveness-Based Locking

Replaces fixed timeouts with active heartbeat checking:

```
Agent A acquires build lock
    ↓
Spawns `cargo build` (attaches process ID)
    ↓
OperationTracker auto-heartbeats every 5 seconds
    ↓
Agent B queries lock status:
  { holder: "agent-a", alive: true, status: "cargo build running", duration: 5m }
    ↓
Agent B registers in wait queue
    ↓
Build completes → lock auto-released → Agent B notified
```

**Stale Lock Recovery**:
- Heartbeat stops → 3 missed heartbeats → lock stale
- Process ID attached → verify with `kill(pid, 0)`
- Next agent in wait queue acquires automatically

### Bidirectional Resource Checking

Prevents conflicts between file edits and build operations:

| Agent Wants To | Build Running | Test Running | File Write Lock Held |
|----------------|---------------|--------------|----------------------|
| **Start Build** | Wait | Wait | **Wait** (files being edited) |
| **Start Test** | Wait | Wait | **Wait** (files being edited) |
| **Write File** | **Wait** (build in progress) | **Wait** (test in progress) | Normal lock contention |
| **Git Commit** | **Wait** (build in progress) | **Wait** (test in progress) | **Wait** (files being edited) |

### Test Coverage

70+ unit tests covering:
- Saga compensation rollback
- Three-state model consistency
- Validation rule evaluation
- Optimistic conflict detection
- Contract-net bid evaluation
- Market allocation pricing
- Worktree lifecycle management

---

## SEAL - Self-Evolving Agentic Learning

> **Paper**: Self-Evolving Agentic Learning for Knowledge-Based Conversational QA

SEAL enables the agent to learn from interactions without model retraining.

```
src/seal/
├── coreference.rs    # Pronoun resolution (26KB)
├── query_core.rs     # Semantic query extraction (33KB)
├── learning.rs       # Pattern learning (29KB)
└── reflection.rs     # Post-execution analysis (30KB)
```

### Components

#### 1. Coreference Resolution

Resolves anaphoric references to concrete entities:

```
User: "Fix it and run tests"
      ↓
SEAL: "Fix [main.rs] and run tests"
```

**Salience Scoring**:
| Factor | Weight |
|--------|--------|
| Recency | 0.35 |
| Graph Centrality | 0.20 |
| Type Match | 0.20 |
| Frequency | 0.15 |
| Syntactic Prominence | 0.10 |

#### 2. Semantic Query Cores

Extracts S-expression-like structured queries from natural language:

```
Input: "What uses main.rs?"
Query Core: (JOIN DependsOn ?dependent "main.rs")
```

**Question Types**: Definition, Location, Dependency, Count, Superlative, Enumeration, Boolean

#### 3. Self-Evolving Learning

**Local Memory** (per-session):
- Entity tracking with mention history
- Coreference resolution log
- Query execution history
- Focus stack for active context

**Global Memory** (cross-session):
- Pattern library with success statistics
- Resolution patterns that worked
- Template library by question type

#### 4. Reflection Module

Post-execution error detection and correction:

| Error Type | Suggested Fix |
|------------|---------------|
| EmptyResult | RetryWithQuery |
| ResultOverflow | NarrowScope |
| EntityNotFound | ResolveEntity |
| RelationMismatch | ExpandScope |

### SEAL Processing Pipeline

```
User Input
    ↓
Coreference Resolution (resolve "it", "the file")
    ↓
Query Core Extraction (create structured query)
    ↓
Learning Coordinator (check learned patterns)
    ↓
Query Execution
    ↓
Reflection Module (validate & correct)
    ↓
Record Outcome (update learning)
```

---

## MDAP - Massively Decomposed Agentic Processes

> **Paper**: "Solving a Million-Step LLM Task with Zero Errors" (MAKER Framework)

MDAP enables zero-error execution through voting and decomposition.

```
src/mdap/
├── voting.rs         # First-to-ahead-by-k consensus (19KB)
├── red_flags.rs      # Output validation (24KB)
├── decomposition.rs  # Task breakdown strategies
├── scaling.rs        # Cost/probability estimation (14KB)
├── microagent.rs     # Individual voting agents (16KB)
├── metrics.rs        # Execution telemetry (18KB)
└── composer.rs       # Result composition (20KB)
```

### Core Algorithms

#### Algorithm 1: generate_solution
```
for each step s:
    action, state = do_voting(state, model, k)
    append action to results
return results
```

#### Algorithm 2: do_voting (First-to-ahead-by-k)
```
V = {} # vote counts
while true:
    y = get_vote(state, model)
    V[y] += 1
    if V[y] >= k + max(V[other]):
        return y
```

#### Algorithm 3: get_vote (with Red-flagging)
```
while true:
    response = sample(model)
    if no_red_flags(response):
        return extract(response)
```

### Red-Flagging System

Strictly discards outputs signaling unreliability:

| Red Flag | Trigger |
|----------|---------|
| Token Overflow | Response > 750 tokens |
| Invalid Format | Malformed output |
| Self-Correction | "Wait", "Actually", "Let me reconsider" |
| Confused Reasoning | Contradictory statements |

### Configuration Presets

| Preset | k | Success Rate | Max Samples |
|--------|---|--------------|-------------|
| Default | 3 | 95% | 50 |
| High Reliability | 5 | 99% | 100 |
| Cost Optimized | 2 | 90% | 30 |

---

## Infinite Context System

Never lose context - even in hours-long coding sessions.

### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│              Full Conversation History                        │
│            (LanceDB with 384-dim embeddings)                  │
│  ┌────┬────┬────┬────┬────┬────┬────┬────┬────┐             │
│  │ M1 │ M2 │ M3 │ M4 │ M5 │ M6 │ M7 │ M8 │ M9 │             │
│  └────┴────┴────┴────┴────┴────┴────┴────┴────┘             │
│        ▲ Semantic Search                                      │
├────────┼─────────────────────────────────────────────────────┤
│        │        Active Context (Sent to API)                  │
│  ┌─────────────────┐  ┌────┬────┬────┬────┐                  │
│  │ [Summary M1-M5] │  │ M6 │ M7 │ M8 │ M9 │                  │
│  └─────────────────┘  └────┴────┴────┴────┘                  │
│                                                               │
│     recall_context tool searches compacted messages           │
└──────────────────────────────────────────────────────────────┘
```

### Tiered Memory

| Tier | Content | Access Speed |
|------|---------|--------------|
| **Hot** | Recent messages, code, decisions | Instant |
| **Warm** | Compressed older messages | Fast |
| **Cold** | Archived with semantic search | Search |

### Entity Extraction

Automatically extracts from conversation:

| Entity Type | Examples |
|-------------|----------|
| File | `src/main.rs`, `config.json` |
| Function | `fn process_data`, `function handleClick` |
| Type | `struct User`, `class Config` |
| Variable | Named identifiers |
| Concept | api, authentication, database |
| Error | Error types and messages |
| Command | cargo, npm, git commands |

### Relationship Graph

Tracks connections between entities:

- **Contains**: File contains function/type
- **References**: Entity references another
- **DependsOn**: Dependency relationship
- **Modifies**: Modification relationship
- **Defines**: Definition relationship
- **CoOccurs**: Mentioned together

### Context Builder

Automatic context enhancement:
1. Analyzes if historical context needed
2. Searches relevant past messages (semantic)
3. Injects only high-relevance content (>75%)
4. Respects token budget

---

## RAG - Retrieval-Augmented Generation

Semantic codebase search with hybrid retrieval.

```
src/rag/
├── client/          # RAG client API
├── embedding/       # FastEmbed (all-MiniLM-L6-v2)
├── vector_db/       # LanceDB backend
├── bm25_search/     # Tantivy full-text
├── indexer/         # Tree-sitter AST chunking
├── git/             # Commit history search
└── cache/           # Persistent hash cache
```

### Features

| Feature | Description |
|---------|-------------|
| **Hybrid Search** | Vector similarity + BM25 keyword matching |
| **AST-Aware Chunking** | Tree-sitter for smart code boundaries |
| **12 Languages** | Rust, Python, JS/TS, Go, Java, Swift, C/C++, C#, Ruby, PHP |
| **Git History** | Semantic search over commits |
| **Incremental Indexing** | Only re-index changed files |

### Performance

- **Embedding Model**: all-MiniLM-L6-v2 (384 dimensions)
- **Search Latency**: 20-30ms
- **Indexing Speed**: ~1000 files/minute
- **Chunk Size**: 50 lines with overlap

---

## Tool System

20+ integrated tools for autonomous operation.

```
src/tools/
├── executor.rs        # Main execution engine (63KB)
├── registry.rs        # Tool registry (14KB)
├── smart_router.rs    # Intelligent selection (12KB)
├── file_ops.rs        # File operations
├── bash.rs            # Shell execution
├── git.rs             # Git operations (30KB)
├── code_exec.rs       # Code execution (22KB)
├── web_search.rs      # Internet search (19KB)
├── semantic_search.rs # Vector search (32KB)
└── task_manager.rs    # Task lifecycle (17KB)
```

### Tool Categories

| Category | Tools |
|----------|-------|
| **File Operations** | Create, read, write, delete, search |
| **Shell** | Bash command execution |
| **Version Control** | Git status, diff, commit, push, branch |
| **Code Execution** | Python, JavaScript, Rhai, Lua |
| **Web** | Search, fetch, scrape (via Thalora) |
| **Search** | Semantic, BM25, hybrid |
| **Planning** | Plan CRUD, branching, templates |
| **Tasks** | Create, start, complete, dependencies |

### Tool Orchestrator

Rhai-based programmatic tool calling (Anthropic pattern):

```
crates/tool-orchestrator/
└── Rhai scripting for complex workflows
```

### Permission Modes

| Mode | Description |
|------|-------------|
| **Auto** | Execute without approval |
| **Interactive** | Prompt for dangerous operations |
| **Deny** | Block tool execution |

---

## MCP Integration

Model Context Protocol for extensibility.

### MCP Client

Connect to external MCP servers:

```
src/mcp/
├── client.rs        # Server connections
├── config.rs        # Server configuration
├── tool_adapter.rs  # Tool adaptation
└── transport.rs     # Stdio protocol
```

### MCP Server Mode

Expose Brainwires as an MCP server:

```bash
brainwires chat --mcp-server
```

---

## TUI/CLI Interface

Full-featured terminal experience with background session support.

```
src/tui/
├── app/           # Application state
├── ui/            # Rendering components
├── events.rs      # Event-driven input/IPC handling (20KB)
└── console.rs     # Buffer management

src/ipc/
├── mod.rs         # IPC module
├── protocol.rs    # Agent-Viewer message types
└── socket.rs      # Unix socket utilities
```

### Agent-Viewer Architecture

The TUI uses a split architecture:
- **Agent**: Background process holding all session state (conversation, MCP, tokio runtime)
- **TUI Viewer**: Thin terminal client that can attach/detach without losing state

```
┌─────────────────┐     IPC      ┌──────────────────────┐
│  TUI (viewer)   │◄────────────►│  Agent Process       │
│  - rendering    │              │  - conversation      │
│  - input        │              │  - tokio runtime     │
│  - can detach   │              │  - MCP connections   │
└─────────────────┘              │  - all session state │
                                 └──────────────────────┘
```

### Session Management Commands

| Command | Description |
|---------|-------------|
| `brainwires chat` | Start new session (spawns Agent + TUI) |
| `brainwires sessions` | List backgrounded sessions |
| `brainwires attach [id]` | Attach TUI to existing Agent |
| `brainwires exit <id>` | Terminate a backgrounded session |

**TUI Shortcuts**:
- `Ctrl+Z`: Open background/suspend dialog
- `Ctrl+C`: Quit and shut down Agent

### Event-Driven IPC

The TUI uses an event-driven architecture - IPC messages from the Agent flow through the same event channel as keyboard and terminal events, eliminating polling lag.

### Chat Modes

| Mode | Usage |
|------|-------|
| **Interactive** | `brainwires chat` |
| **TUI** | `brainwires chat --tui` |
| **Single-Shot** | `brainwires chat --prompt "..."` |
| **Batch** | `cat prompts.txt \| brainwires chat --batch` |
| **Quiet** | `brainwires chat -q` |
| **MCP Server** | `brainwires chat --mcp-server` |

### Output Formats

| Format | Description |
|--------|-------------|
| `full` | Rich formatting with colors |
| `plain` | Response text only |
| `json` | Structured metadata |

### Slash Commands

```
/project:index [path]          # Index codebase
/project:query <query>         # Semantic search
/task:add <description>        # Add task
/task:start <id>               # Start task
/plan:branch <name>            # Create sub-plan
/template:use <name> [vars]    # Use plan template
```

---

## Storage & Persistence

All local, privacy-respecting storage.

```
src/storage/
├── lance_client.rs       # LanceDB wrapper (20KB)
├── conversation_store.rs # Conversations
├── message_store.rs      # Messages
├── task_store.rs         # Tasks (22KB)
├── plan_store.rs         # Plans (18KB)
├── pattern_store.rs      # SEAL patterns (17KB)
├── relationship_graph.rs # Entity graph (27KB)
├── tiered_memory.rs      # Hot/warm/cold (14KB)
├── document_store.rs     # Documents (28KB)
└── image_store.rs        # Images (22KB)
```

### Databases

| Database | Purpose |
|----------|---------|
| **LanceDB** | Vector storage, search, persistence |
| **SQLite** | Cross-process locking, structured data |
| **Tantivy** | BM25 full-text search |

### Data Locations

- **Project Data**: `.brainwires/` in project root
- **Global Data**: `~/.brainwires/` for user config
- **XDG Compliant**: Follows XDG directory specification

---

## Roadmap & Research Pipeline

### In Progress

| Feature | Priority | Status |
|---------|----------|--------|
| Mid-Session Model Switch | High | In Progress |
| Multimodal Input (Images) | Medium | Planned |
| IDE Extensions (VSCode) | Lower | Planned |

### Research Implementation Pipeline

From [agent-reliability-papers.md](./research/agent-reliability-papers.md):

#### High Priority

| Paper | Innovation | Expected Impact |
|-------|------------|-----------------|
| **RASC** | Dynamic sampling with early stopping | 70% fewer samples |
| **Ranked Voting** | Instant-runoff, Borda count | Better accuracy |
| **CISC** | Confidence-weighted voting | Smarter aggregation |

#### Medium Priority

| Paper | Innovation | Application |
|-------|------------|-------------|
| **GAP** | Graph-based parallel execution | Task decomposition |
| **GoalAct** | Hierarchical skill decomposition | 12% success improvement |
| **AgentDebug** | Error taxonomy & recovery | 24% accuracy boost |
| **MAST** | Multi-agent failure patterns | SEAL integration |

#### Planned Features

| Feature | Description |
|---------|-------------|
| **Routines** | AI-created action sequences |
| **Agent-2-Agent** | Inter-agent collaboration |
| **SSH/SCP Shell** | Remote folder operations |
| **Clipboard Support** | Image/file paste |
| **Cloud Integration** | Remote task execution |

---

## Technical Specifications

### Build Configuration

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### Platform Support

| Platform | Architecture |
|----------|--------------|
| Linux | x86_64, aarch64, armv7 |
| macOS | x86_64, aarch64 (Universal) |
| Windows | x86_64 |

### Key Dependencies

| Category | Crates |
|----------|--------|
| **Async** | tokio, futures, async-trait |
| **CLI/TUI** | clap, ratatui, crossterm |
| **AI/ML** | fastembed, rmcp |
| **Data** | lancedb, arrow, tantivy |
| **Parsing** | tree-sitter (12 languages) |
| **Git** | git2 |

---

## Getting Started

```bash
# Install
cargo install --path .

# Authenticate
brainwires auth login

# Start chatting
brainwires chat

# Index your codebase
> /project:index .

# Search semantically
> /project:query "authentication implementation"
```

---

## Philosophy

**Bleeding-Edge Implementation**
- Implement papers within weeks of arXiv publication
- Prioritize techniques with measured improvements
- Focus on reliability over raw capability

**Local-First**
- All data stays on your machine
- No cloud dependencies for core features
- Privacy by default

**Developer Experience**
- Multiple interaction modes for every workflow
- Rich tooling for autonomous operation
- Transparent execution with detailed metrics

---

*Built with Rust. Powered by research. Designed for developers.*
