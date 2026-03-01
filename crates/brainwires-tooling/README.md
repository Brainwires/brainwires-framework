# brainwires-tooling

[![Crates.io](https://img.shields.io/crates/v/brainwires-tooling.svg)](https://crates.io/crates/brainwires-tooling)
[![Documentation](https://docs.rs/brainwires-tooling/badge.svg)](https://docs.rs/brainwires-tooling)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Composable tool infrastructure for the Brainwires Agent Framework**

The tooling layer that gives agents their capabilities: file operations, shell execution, git integration, web access, code search, validation, transactions, and more. Designed for composability — register only the tools you need, or use `ToolRegistry::with_builtins()` for everything.

## Overview

`brainwires-tooling` provides:

- **Built-in tool implementations** — Bash, file ops, git, web, code search, validation
- **Composable ToolRegistry** — Register individual tools or categories; supports initial + deferred loading
- **ToolExecutor trait** — Object-safe abstraction for tool dispatch with pre-execution hooks
- **Transaction manager** — Two-phase commit staging for atomic file operations
- **Sanitization** — Prompt injection detection, sensitive data redaction, content source wrapping
- **Error classification** — Taxonomy-based error categorization with retry strategy recommendations
- **Orchestration** — Rhai script engine for multi-step tool pipelines (feature-gated)
- **Smart routing** — Context-aware tool selection based on query analysis (feature-gated)

```
┌─────────────────────────────────────────────────────────────────┐
│                        brainwires-tooling                       │
├─────────────┬───────────────┬───────────────┬───────────────────┤
│  Registry   │   Executor    │  Sanitization │  Error Taxonomy   │
│  ─────────  │   ─────────   │  ───────────  │  ──────────────   │
│  Composable │  Object-safe  │  Injection    │  7 categories     │
│  container  │  dispatch     │  detection    │  Retry strategies │
│  16 cats    │  Pre-hooks    │  Redaction    │  Pattern matching │
├─────────────┴───────────────┴───────────────┴───────────────────┤
│                      Always Available                           │
│  ┌──────┐ ┌──────────┐ ┌─────┐ ┌─────┐ ┌────────┐ ┌────────┐  │
│  │ Bash │ │ File Ops │ │ Git │ │ Web │ │ Search │ │ Valid. │  │
│  └──────┘ └──────────┘ └─────┘ └─────┘ └────────┘ └────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                      Feature-Gated                              │
│  ┌──────────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │ Orchestrator │ │ Code Exec│ │ RAG/Sem. │ │ Smart Router │   │
│  │  (rhai)      │ │ (interp.)│ │ (rag)    │ │ (smart-rtr.) │   │
│  └──────────────┘ └──────────┘ └──────────┘ └──────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                    Transaction Manager                          │
│            Two-phase commit  ·  Atomic staging                  │
│            Auto-cleanup      ·  Copy+delete fallback            │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-tooling = { version = "0.1", features = ["native"] }
```

Register tools and execute:

```rust
use brainwires_tooling::{ToolRegistry, ToolExecutor, BashTool, FileOpsTool, GitTool};

// Compose only the tools you need
let mut registry = ToolRegistry::new();
registry.register_tools(BashTool::get_tools());
registry.register_tools(FileOpsTool::get_tools());
registry.register_tools(GitTool::get_tools());

// Or use all built-in tools
let registry = ToolRegistry::with_builtins();

// Look up tool definitions
if let Some(tool) = registry.get("read_file") {
    println!("Found: {}", tool.description);
}

// Search for tools by keyword
let matches = registry.search_tools("file");
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | File ops, bash, git, web, search, validation (requires OS) |
| `wasm` | No | WASM-compatible subset (no filesystem/process access) |
| `orchestrator` | No | Rhai script engine for multi-step tool pipelines |
| `orchestrator-wasm` | No | Orchestrator compiled for WASM targets |
| `rag` | No | RAG-powered semantic codebase search |
| `interpreters` | No | Sandboxed multi-language code execution |
| `smart-router` | No | Context-aware tool selection and routing |
| `full` | No | All optional features (`orchestrator` + `rag` + `interpreters` + `smart-router`) |

## Architecture

### ToolRegistry

A composable container for tool definitions with 16 categories:

```
FileOps · Git · Web · Bash · Search · Validation · CodeExec
Orchestrator · SemanticSearch · ToolSearch · SmartRouter
MCP · Agent · Context · System · Other
```

Supports initial/deferred tool loading — agents start with a focused toolset and discover additional tools at runtime via the `tool_search` meta-tool.

### ToolExecutor

Object-safe trait for abstracting tool dispatch:

```rust
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value,
        context: &ToolContext,
    ) -> anyhow::Result<ToolResult>;
}
```

Pre-hooks (`ToolPreHook`) allow intercepting tool calls for permission checks, logging, or transformation before execution.

### Built-in Tools

**Bash** — Shell command execution with output modes (full, head, tail, filter, count, smart), stderr handling, and interactive command rejection.

**File Ops** — 8 operations: `read_file`, `write_file`, `edit_file`, `patch_file`, `list_directory`, `search_files`, `delete_file`, `create_directory`.

**Git** — 11 operations: `git_status`, `git_diff`, `git_log`, `git_stage`, `git_unstage`, `git_commit`, `git_push`, `git_pull`, `git_fetch`, `git_discard`, `git_branch`.

**Web** — URL fetching with content extraction.

**Search** — Regex-based code search that respects `.gitignore` rules.

**Validation** — Duplicate detection, build verification (cargo/npm), syntax checking with configurable timeouts.

### Transaction Manager

Two-phase commit for atomic file operations:

```rust
use brainwires_tooling::TransactionManager;

let tm = TransactionManager::new("/path/to/staging");

// Stage writes (nothing touches target files yet)
tm.stage("key1", b"file contents", "/target/path.rs").await?;

// Atomic commit (rename, with copy+delete fallback)
tm.commit().await?;

// Or rollback (deletes staged files, targets untouched)
tm.rollback().await?;
```

Auto-cleanup on drop ensures no leaked staging files.

### Sanitization

Defense-in-depth for agent safety:

```rust
use brainwires_tooling::{
    is_injection_attempt, contains_sensitive_data,
    redact_sensitive_data, sanitize_external_content,
};

// Detect prompt injection (30+ patterns)
if is_injection_attempt(user_input) {
    // reject or flag
}

// Detect API keys, tokens, credentials, PII
if contains_sensitive_data(output) {
    let safe = redact_sensitive_data(output);
    // "sk-abc123..." → "[REDACTED: api_key]"
}
```

### Error Classification

Taxonomy based on the AgentDebug paper (arxiv:2509.25370):

```rust
use brainwires_tooling::{classify_error, ToolErrorCategory, RetryStrategy};

let outcome = classify_error("connection refused");
// → ToolErrorCategory::Transient
// → RetryStrategy::ExponentialBackoff { base_ms: 1000, max_retries: 3 }
```

Seven categories: `Transient`, `InputValidation`, `ExternalService`, `Permission`, `Logic`, `Resource`, `Unknown` — each mapped to an appropriate retry strategy.

### Orchestrator

Rhai-based script engine for composing multi-step tool pipelines:

```rust
use brainwires_tooling::OrchestratorTool;

// Define workflows as Rhai scripts
// Supports sandboxed execution with resource limits
```

Requires the `orchestrator` feature. See `examples/` for complete workflows.

## Integration

`brainwires-tooling` is used by:

- **brainwires-agents** — Task agents use `ToolExecutor` for all tool dispatch and `ToolRegistry` for tool discovery
- **brainwires-agents** (reasoning feature) — Reasoning router uses tool categories for smart delegation
- **brainwires-wasm** — WASM orchestrator uses the `wasm` feature subset
- **brainwires-seal** — Learning module integrates tool execution for experience capture
- **brainwires (facade)** — Re-exports tooling types for unified API access

## License

MIT — see [LICENSE](../../LICENSE) for details.
