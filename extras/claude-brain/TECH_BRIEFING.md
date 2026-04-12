# Claude Brain — Technical Briefing

> Single Rust binary that replaces Claude Code's default compaction with Brainwires
> research-grade tiered memory, semantic search, and knowledge extraction.

**Version:** 0.9.0
**Codebase:** 1,332 lines Rust + 580 lines shell
**Binary:** `target/release/claude-brain`

---

## Table of Contents

1. [What It Does](#what-it-does)
2. [Architecture](#architecture)
3. [Binary Modes](#binary-modes)
4. [What Happens During Compaction](#what-happens-during-compaction)
5. [Hook Lifecycle](#hook-lifecycle)
6. [MCP Tools](#mcp-tools)
7. [Storage Architecture](#storage-architecture)
8. [Framework Dependencies](#framework-dependencies)
9. [Configuration](#configuration)
10. [Installation](#installation)
11. [File Map](#file-map)
12. [Known Limitations](#known-limitations)
13. [Testing](#testing)

---

## What It Does

Claude Code's built-in compaction is dumb LLM summarization — it nukes your full conversation
and replaces it with a lossy summary. Critical decisions, code context, user preferences, and
architectural reasoning get flattened into a paragraph.

Claude Brain intercepts the compaction lifecycle via Claude Code hooks and replaces it with
Brainwires' tiered memory system:

- **Every turn** gets captured to vector-indexed storage (Stop hook)
- **Before compaction** fires, the full transcript is exported to persistent memory (PreCompact hook)
- **After compaction**, rich context is injected back — facts, prior decisions, recalled memories (PostCompact hook)
- **On session start**, relevant context from all prior sessions is loaded (SessionStart hook)
- **During the session**, 5 MCP tools let Claude actively search and store to persistent memory

The result: context survives compaction, spans sessions, and is semantically searchable.

---

## Architecture

```
                    Claude Code
                        |
           ┌────────────┼────────────┐
           │            │            │
     Hooks (stdin)   MCP (stdio)   Settings
           │            │            │
           v            v            v
    ┌──────────────────────────────────────┐
    │         claude-brain binary           │
    │                                      │
    │  hook session-start   serve (MCP)    │
    │  hook stop                           │
    │  hook pre-compact                    │
    │  hook post-compact                   │
    └──────────────┬───────────────────────┘
                   │
                   v
    ┌──────────────────────────────────────┐
    │         ContextManager               │
    │    (wraps BrainClient + config)      │
    └──────────────┬───────────────────────┘
                   │
          ┌────────┼────────┐
          v        v        v
      LanceDB   SQLite   SQLite
      (thoughts) (PKS)    (BKS)
      + vectors  facts    truths
```

**One binary, two modes:**
- `claude-brain serve` — Long-lived MCP server (stdio transport)
- `claude-brain hook <event>` — Short-lived hook handler (stdin JSON → stdout)

Same storage layer, no lock contention. MCP server is the only long-lived writer; hooks are fire-and-forget.

---

## Binary Modes

### `claude-brain serve` (default)

Starts an MCP server on stdin/stdout. Claude Code launches this automatically via `.mcp.json`.
Exposes 5 tools for active memory management during conversations. Long-lived process — stays
running for the entire Claude Code session.

### `claude-brain hook <event>`

Short-lived process. Claude Code invokes this on lifecycle events, passes JSON on stdin,
captures stdout output. Exits immediately after processing.

Events: `session-start`, `stop`, `pre-compact`, `post-compact`

### `claude-brain version`

Prints system info: modes, tools, storage paths, embedding model.

---

## What Happens During Compaction

This is the key question. Here's the exact sequence:

### Current Settings

```json
{
  "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "100000",
  "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "50"
}
```

Auto-compaction is NOT disabled — it fires when context hits ~100K tokens at 50% threshold.
This is intentional. Compaction still runs, but Brainwires wraps it.

### The Compaction Sequence

```
1. Context window fills past threshold
   └─ Claude Code triggers auto-compact (or user runs /compact)

2. PRE-COMPACT HOOK FIRES (timeout: 10s)
   ├─ Receives: session_id, transcript_path (JSONL file), cwd, trigger
   ├─ Opens the JSONL transcript file
   ├─ Reads every line, extracts (role, content) pairs
   │   ├─ Handles both string content and block-array content
   │   ├─ Only captures user + assistant messages (skips system)
   │   └─ Skips messages < 20 chars (noise)
   ├─ For EACH message:
   │   ├─ Formats as "[role] content" (truncated at 2000 chars)
   │   ├─ Generates embedding vector (all-MiniLM-L6-v2, 384d)
   │   ├─ Extracts tags from content (#hashtags)
   │   ├─ Runs PKS fact extraction (regex patterns for preferences/identity)
   │   ├─ Runs evidence checking (corroboration/contradiction against existing thoughts)
   │   └─ Stores to LanceDB thoughts table with:
   │       ├─ category: "conversation"
   │       ├─ tags: ["claude-code", "pre-compact", "session:{id}"]
   │       ├─ importance: 0.6
   │       └─ source: "pre-compact-export"
   └─ Logs: "[timestamp] PRE-COMPACT fired — N messages from transcript (trigger=auto)"

3. CLAUDE CODE RUNS ITS NORMAL COMPACTION
   └─ LLM summarizes the conversation → compact_summary
   └─ Full conversation context is replaced with the summary

4. POST-COMPACT HOOK FIRES (timeout: 10s)
   ├─ Receives: session_id, transcript_path, cwd, compact_summary
   ├─ Uses compact_summary as a semantic search query
   │   (fallback: "recent conversation context decisions")
   ├─ Searches PKS/BKS knowledge base (top 15 facts)
   ├─ Searches thought memory (top 10 results, min_score 0.5)
   ├─ Formats results as markdown:
   │   ├─ "## Key Knowledge (from Brainwires)" — PKS/BKS facts
   │   └─ "## Recalled Context (from Brainwires)" — relevant memories
   ├─ Writes to stdout → Claude Code injects as system message
   └─ Logs: "[timestamp] POST-COMPACT fired — summary N chars"

5. SESSION CONTINUES
   └─ Claude now has: compact_summary + Brainwires context restoration
   └─ All pre-compact data persists in LanceDB for future recall
```

### What This Means In Practice

**Before claude-brain:** Compaction = lossy summarization. Decisions lost. Context gone.

**After claude-brain:**
- Full conversation exported to searchable vector storage BEFORE compaction runs
- After compaction, relevant facts and memories injected back alongside the summary
- Everything remains searchable via `recall_context` and `search_memory` MCP tools
- Context accumulates across compaction events and across sessions

### The Stop Hook (Between Compactions)

Every Claude turn also gets captured independently:

```
Claude responds to user
  └─ STOP HOOK FIRES (timeout: 30s)
      ├─ Receives: session_id, stop_reason, assistant_message, user_message
      ├─ If assistant_message > 50 chars:
      │   └─ Stores "[assistant] {message}" with importance 0.5
      ├─ If user_message > 20 chars:
      │   └─ Stores "[user] {message}" with importance 0.4
      └─ Tags: ["claude-code", "auto-capture", "session:{id}", "stop:{reason}"]
```

This means even if PreCompact somehow fails, individual turns are already captured.

---

## Hook Lifecycle

### SessionStart

**When:** New Claude Code session starts
**Timeout:** 5 seconds
**Input:** `{ session_id, cwd, transcript_path, source, model }`
**Output:** Markdown context (stdout) or nothing

**Flow:**
1. Extracts project name from cwd (last path component)
2. Searches PKS/BKS for facts matching project name
3. Loads 5 most recent thoughts via `list_recent()`
4. Formats as "# Claude Brain — Session Context" with subsections
5. Writes to stdout (Claude sees it as system message)

### Stop

**When:** After every Claude turn
**Timeout:** 30 seconds
**Input:** `{ session_id, stop_reason, assistant_message, user_message }`
**Output:** Silent (logs to file only)

**Flow:**
1. Captures assistant message if > 50 chars (importance 0.5)
2. Captures user message if > 20 chars (importance 0.4)
3. Each capture: embed → store → extract PKS facts → evidence check

### PreCompact

**When:** Before compaction runs (auto or manual)
**Timeout:** 10 seconds
**Input:** `{ session_id, transcript_path, cwd, trigger }`
**Output:** Silent (logs to file only)

**Flow:**
1. Reads full JSONL transcript file
2. Extracts all user/assistant messages
3. Stores each to LanceDB with embeddings (importance 0.6)

### PostCompact

**When:** After compaction completes
**Timeout:** 10 seconds
**Input:** `{ session_id, transcript_path, cwd, compact_summary }`
**Output:** Markdown context (stdout) injected into conversation

**Flow:**
1. Uses compact_summary as semantic search query
2. Retrieves relevant PKS/BKS facts (top 15)
3. Retrieves relevant memories (top 10, min_score 0.5)
4. Formats and writes to stdout

---

## MCP Tools

Five tools exposed via MCP protocol when `claude-brain serve` runs:

### recall_context

```
Purpose: Search conversation history for context outside current window
Input:   { query: string, limit?: 10, min_score?: 0.3 }
Output:  SearchMemoryResponse (JSON)
Calls:   ContextManager::search_memory() → BrainClient::search_memory()
```

Generates embedding for query, runs ANN vector search against thoughts table,
also searches PKS keyword index. Returns ranked results with scores.

### capture_thought

```
Purpose: Persist a decision, insight, or important context
Input:   { content: string, category?: string, tags?: string[], importance?: 0.5 }
Output:  CaptureThoughtResponse (JSON)
Calls:   BrainClient::capture_thought()
```

Full pipeline: auto-detect category → extract #hashtags → generate embedding →
store to LanceDB → extract PKS facts (regex) → run evidence check (corroboration/contradiction) →
update confidence scores via EMA.

Categories: decision, person, insight, meeting_note, idea, action_item, reference, conversation, general

### search_memory

```
Purpose: Semantic search across all memory tiers
Input:   { query: string, limit?: 10, min_score?: 0.3 }
Output:  SearchMemoryResponse (JSON)
Calls:   ContextManager::search_memory() → BrainClient::search_memory()
```

Same as recall_context (both call the same underlying method).
Searches: thoughts (vector), PKS facts (keyword), BKS truths (keyword).

### search_knowledge

```
Purpose: Query PKS facts and BKS truths
Input:   { query: string, limit?: 10 }
Output:  SearchKnowledgeResponse (JSON)
Calls:   ContextManager::search_knowledge() → BrainClient::search_knowledge()
```

Keyword search against PKS (personal facts) and BKS (behavioral truths).
Filters by min_confidence 0.5.

### memory_stats

```
Purpose: Dashboard of all memory statistics
Input:   {} (empty)
Output:  MemoryStatsResponse (JSON)
Calls:   ContextManager::memory_stats() → BrainClient::memory_stats()
```

Returns: thought counts by category, recent counts (24h/7d/30d), top tags,
PKS fact counts by category + avg confidence, BKS truth counts by category.

---

## Storage Architecture

### Three-Tier Memory

| Tier | Backend | Path | Contents | Search |
|------|---------|------|----------|--------|
| **Hot** (thoughts) | LanceDB | `~/.brainwires/claude-brain/` | Embedded thoughts with vectors | ANN vector search (semantic) |
| **Warm** (PKS) | SQLite | `~/.brainwires/pks.db` | Personal facts (preferences, identity) | Keyword search |
| **Cold** (BKS) | SQLite | `~/.brainwires/bks.db` | Behavioral truths (cross-session patterns) | Keyword search |

### Thoughts Table Schema (LanceDB)

| Column | Type | Description |
|--------|------|-------------|
| vector | Vec\<f32\> (384d) | Embedding from all-MiniLM-L6-v2 |
| id | String | UUID |
| content | String | The thought text (prefixed with [role]) |
| category | String | decision/insight/conversation/general/etc. |
| tags | String (JSON array) | Auto-extracted + user-provided tags |
| source | String | manual/conversation/pre-compact-export/claude-code-turn |
| importance | f32 | 0.0-1.0 |
| created_at | i64 | Unix timestamp |
| updated_at | i64 | Unix timestamp |
| deleted | bool | Soft-delete flag |
| confidence | f32 | 0.0-1.0, updated via EMA |
| evidence_chain | String (JSON array) | IDs of corroborating/contradicting thoughts |
| reinforcement_count | u32 | Times corroborated by other thoughts |
| contradiction_count | u32 | Times contradicted |

### PKS Fact Schema (SQLite)

| Field | Description |
|-------|-------------|
| id | UUID |
| category | Identity/Preference/Capability/Context/Constraint/Relationship |
| key | Fact key (e.g., "preferred_language", "current_project") |
| value | Fact value (e.g., "Rust", "brainwires-framework") |
| context | Optional context (e.g., "backend projects") |
| confidence | 0.0-1.0 with EMA updates and time-decay |
| source | ExplicitStatement (0.9) / InferredFromBehavior (0.7) / ProfileSetup (0.85) / SystemObserved (0.6) |

### Embedding Model

- **Model:** all-MiniLM-L6-v2
- **Dimensions:** 384
- **Runtime:** FastEmbed (ONNX, fully local)
- **Caching:** LRU cache (1000 entries) via CachedEmbeddingProvider
- **No API calls** — all embeddings generated locally

### Vector Search

Search flow:
1. Query text → embedding (384d vector)
2. LanceDB ANN search on thoughts table, "vector" column
3. Distance → similarity: `score = 1.0 / (1.0 + distance)`
4. Filter by min_score, limit, optional category/source filters
5. Return ranked ScoredRecord[]

---

## Framework Dependencies

Claude-brain is a thin orchestration layer. The heavy lifting lives in the framework:

### brainwires-knowledge (features: knowledge, dream)

**BrainClient** — Central API for all storage operations:
- `capture_thought()` — Full pipeline: categorize → tag → embed → store → extract facts → evidence check
- `search_memory()` — Vector search on thoughts + keyword search on PKS/BKS
- `search_knowledge()` — PKS/BKS keyword search
- `list_recent()` — Recent thoughts by category/time
- `memory_stats()` — Dashboard statistics
- `delete_thought()` — Soft delete
- `get_thought()` — Single thought retrieval

**Evidence System:**
- On every `capture_thought()`, searches for semantically similar existing thoughts
- **Corroboration** (score >= 0.85): increases confidence via EMA (alpha=0.3)
- **Contradiction** (score >= 0.70 + negation XOR): decreases confidence
- Maintains bidirectional evidence_chain links between thoughts
- Formula: `new_confidence = 0.3 * adjustment + 0.7 * old_confidence`

**Fact Extractor** (`detect_category` + `extract_tags`):
- Keyword-based category detection (no LLM): scans for decision words, person names, insight keywords, etc.
- Hashtag extraction via regex: `#([A-Za-z][A-Za-z0-9_-]{1,30})`

**PersonalFactCollector** (PKS extraction):
- 26 regex patterns across 5 categories:
  - **Identity** (9): "my name is", "call me", "I'm a ... at", "I work for", "I'm on the ... team"
  - **Preference** (5): "I prefer", "I like using", "I'd rather", "my favorite", "I use X for"
  - **Capability** (4): "I'm proficient in", "I know", "I've been using X for Y years", "I'm an expert in"
  - **Context** (4): "I'm working on", "my project is", "I'm trying to", "today I'm"
  - **Constraint** (4): "I can't", "I'm in X timezone", "I'm limited to", "I'm not allowed to"
- All regex, no LLM. Fast but narrow — only catches explicit first-person statements.
- Confidence varies by pattern (0.65-0.9)
- Min confidence threshold: 0.7

**Dream Consolidation** (offline, not yet wired to claude-brain hooks):
- 4-phase cycle: Orient → Load → Consolidate → Prune
- Uses LLM provider to summarize old messages into compact summaries
- Extracts structured facts from summaries
- Replaces old messages with [summary + recent messages]
- DemotionPolicy controls when messages become consolidation candidates

### brainwires-storage

**LanceDB Backend:**
- Table creation with Arrow schema
- Record insert/query/delete/count
- Vector search with ANN (now with explicit column specification)
- Filter-to-SQL conversion

**EmbeddingProvider:**
- FastEmbedManager wraps ONNX model
- CachedEmbeddingProvider adds LRU caching (1000 entries)
- Supports multiple models (all-MiniLM-L6-v2 default)

### brainwires-core

- `Message` type used by dream consolidation
- Core traits and utilities

---

## Configuration

### Config File: `~/.brainwires/claude-brain.toml`

```toml
[storage]
# Defaults to ~/.brainwires/ — uncomment to override
# brain_path = "/custom/path/claude-brain"
# pks_path = "/custom/path/pks.db"
# bks_path = "/custom/path/bks.db"

[policy]
hot_max_age_hours = 24          # Hours before hot-tier candidates for consolidation
warm_max_age_days = 7           # Days before warm-tier candidates for fact extraction
hot_token_budget = 50000        # Token budget for hot tier
keep_recent = 4                 # Recent messages always kept
min_importance = 0.3            # Minimum importance for retention

[session_start]
max_facts = 20                  # Max cold-tier facts loaded at session start
max_summaries = 5               # Max warm-tier summaries loaded
max_context_tokens = 4000       # Token budget for loaded context

[capture]
extract_facts = true            # Extract PKS facts from captured turns
consolidation_threshold = 20    # Turn count before triggering consolidation
```

### Claude Code Settings: `~/.claude/settings.json`

```json
{
  "env": {
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "100000",
    "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "50"
  },
  "permissions": {
    "allow": [
      "mcp__claude-brain__memory_stats",
      "mcp__claude-brain__recall_context",
      "mcp__claude-brain__search_memory",
      "mcp__claude-brain__search_knowledge",
      "mcp__claude-brain__capture_thought"
    ]
  },
  "hooks": {
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "...claude-brain hook session-start", "timeout": 5 }] }],
    "Stop":         [{ "hooks": [{ "type": "command", "command": "...claude-brain hook stop",          "timeout": 30 }] }],
    "PreCompact":   [{ "hooks": [{ "type": "command", "command": "...claude-brain hook pre-compact",   "timeout": 10 }] }],
    "PostCompact":  [{ "hooks": [{ "type": "command", "command": "...claude-brain hook post-compact",  "timeout": 10 }] }]
  }
}
```

### MCP Config: `~/.claude/mcp.json`

```json
{
  "mcpServers": {
    "claude-brain": {
      "command": "/path/to/target/release/claude-brain",
      "args": ["serve"]
    }
  }
}
```

### Rules File: `~/.claude/rules/claude-brain.md`

Guidance for Claude on when to use each MCP tool. Installed by `install.sh`.

---

## Installation

### `install.sh`

```bash
./install.sh [install|uninstall|status] [--global] [--project-dir /path]
```

**Install (default):**
1. Builds release binary via `cargo build --release -p claude-brain`
2. Writes default TOML config to `~/.brainwires/claude-brain.toml` (if missing)
3. Merges hooks + env vars + MCP permissions into settings JSON (idempotent, via embedded Python)
4. Registers MCP server in mcp.json
5. Writes rules file to `.claude/rules/claude-brain.md`

**Uninstall:**
- Removes hooks, env vars, MCP entry, rules file
- Keeps binary and data (safe uninstall)

**Status:**
- Shows binary (exists? size?), config, hook wiring, MCP registration, data dir size, recent log entries

**Modes:**
- `--global` — Writes to `~/.claude/settings.json` + `~/.claude/mcp.json` (all projects)
- `--project-dir PATH` — Writes to `PATH/.claude/settings.local.json` + `PATH/.mcp.json`
- Neither — Defaults to framework root

---

## File Map

```
extras/claude-brain/
├── Cargo.toml                    (52 lines)   Package manifest
├── install.sh                    (498 lines)  Install/uninstall/status script
├── test-compaction.sh            (81 lines)   Test helper for compaction hooks
├── TECH_BRIEFING.md              (this file)
└── src/
    ├── lib.rs                    (6 lines)    Module re-exports
    ├── main.rs                   (131 lines)  CLI entry: serve | hook | version
    ├── config.rs                 (195 lines)  TOML config with defaults
    ├── context_manager.rs        (182 lines)  Core orchestrator (wraps BrainClient)
    ├── hook_protocol.rs          (98 lines)   Claude Code hook JSON stdin/stdout
    ├── mcp_server.rs             (223 lines)  MCP server with 5 tools
    ├── session_adapter.rs        (116 lines)  DreamSessionStore bridge (partial)
    └── hooks/
        ├── mod.rs                (4 lines)    Module re-exports
        ├── session_start.rs      (42 lines)   Load context on session start
        ├── stop.rs               (95 lines)   Capture every turn
        ├── pre_compact.rs        (161 lines)  Export transcript before compaction
        └── post_compact.rs       (79 lines)   Inject context after compaction
```

**Framework crates used:**

```
crates/brainwires-knowledge/src/knowledge/
├── brain_client.rs              BrainClient (all storage operations)
├── types.rs                     Request/Response types
├── thought.rs                   Thought struct, ThoughtCategory, ThoughtSource
├── fact_extractor.rs            detect_category(), extract_tags()
└── bks_pks/personal/
    ├── collector.rs             PersonalFactCollector (26 regex patterns)
    └── fact.rs                  PersonalFact struct

crates/brainwires-knowledge/src/dream/
├── consolidator.rs              DreamConsolidator (4-phase cycle)
├── policy.rs                    DemotionPolicy
├── summarizer.rs                LLM-based message summarization
├── fact_extractor.rs            LLM-based fact extraction from summaries
├── task.rs                      Cron-scheduled consolidation
└── metrics.rs                   DreamReport, DreamMetrics

crates/brainwires-storage/src/
├── databases/lance/
│   └── storage_backend.rs       LanceDB vector search + CRUD
└── embeddings.rs                FastEmbed + LRU cache
```

---

## Known Limitations

### Search Scores Are Low

Vector similarity scores from all-MiniLM-L6-v2 + LanceDB distance conversion typically
range 0.3-0.5 for relevant results. The default min_score was 0.6 (filtering everything).
**Fixed:** lowered to 0.3.

### PKS Fact Extraction Is Narrow

Regex-based only. Catches explicit first-person statements ("I prefer Rust", "my name is Alice")
but misses implicit preferences from behavior. Would need LLM-based extraction for richer
fact mining — that's what the Dream consolidation system does, but it's not yet wired into
the hook pipeline.

### Dream Consolidation Not Wired

The `session_adapter.rs` bridges BrainClient to DreamSessionStore, but:
- `save()` is a no-op (BrainClient lacks bulk delete by tag)
- No hook or cron triggers consolidation automatically
- `consolidate_now` MCP tool planned but not implemented

### Duplicate Storage on Compaction

If Stop hook captures turns AND PreCompact exports the full transcript, the same messages
get stored twice (once per-turn, once bulk). They get different tags ("auto-capture" vs
"pre-compact") and different importance (0.5 vs 0.6), but it's redundant storage. The
evidence system partially handles this (corroboration detection), but deduplication is imperfect.

### No Context Editing

Claude Code does not expose context editing. Hooks can only ADD to context (stdout injection).
They cannot remove, replace, or reorder messages in the conversation window. This means
Brainwires supplements compaction but cannot fully replace it — compaction still runs, we just
make it less destructive.

---

## Testing

### Manual Testing

```bash
# Test session-start hook
echo '{"session_id":"test","cwd":"/home/user/project"}' | \
  claude-brain hook session-start

# Test stop hook
echo '{"assistant_message":"Here is the implementation...","session_id":"test"}' | \
  claude-brain hook stop

# Test search via MCP (requires MCP client)
# Use the MCP tools directly in Claude Code session

# Check stats
# Use memory_stats MCP tool in session
```

### Compaction Testing

```bash
cd extras/claude-brain/
./test-compaction.sh setup    # Sets tiny 20K window, 30% trigger
# Open new Claude Code session, read large files, watch compaction fire
./test-compaction.sh watch    # Tail the hook log
./test-compaction.sh restore  # Restore production settings
```

### Hook Log

All hook events logged to `~/.brainwires/claude-brain-hooks.log`:

```
[2026-04-12 10:00:00 UTC] SESSION-START fired — cwd=/home/user/project session=abc123
[2026-04-12 10:01:30 UTC] STOP fired — assistant_message 1500 chars
[2026-04-12 10:05:00 UTC] PRE-COMPACT fired — 45 messages from transcript (trigger=auto)
[2026-04-12 10:05:02 UTC] POST-COMPACT fired — summary 800 chars
```
