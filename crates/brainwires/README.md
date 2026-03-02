# brainwires

[![Crates.io](https://img.shields.io/crates/v/brainwires.svg)](https://crates.io/crates/brainwires)
[![Documentation](https://img.shields.io/docsrs/brainwires)](https://docs.rs/brainwires)
[![License](https://img.shields.io/crates/l/brainwires.svg)](LICENSE)

Unified facade crate for the Brainwires Agent Framework — build any AI application in Rust.

## Overview

`brainwires` is the single entry point for the entire framework. It re-exports 17 sub-crates as feature-gated modules and provides a `prelude` that pulls in the most commonly needed types. Add one dependency, enable the features you need, and you're ready to go.

`brainwires-core` (messages, tools, providers, tasks, errors) is **always available** — no feature flag required. Everything else is opt-in.

```text
                              ┌─────────────┐
                              │  brainwires  │  (facade)
                              └──────┬──────┘
           ┌──────────┬──────────┬───┴───┬──────────┬──────────┐
           │          │          │       │          │          │
    ┌──────▼──┐ ┌─────▼───┐ ┌───▼───┐ ┌─▼────┐ ┌──▼───┐ ┌───▼────┐
    │  core   │ │ tooling │ │ agents│ │ mcp  │ │ mdap │ │storage │
    │ (always)│ │         │ │       │ │      │ │      │ │        │
    └─────────┘ └─────────┘ └───────┘ └──────┘ └──────┘ └────────┘
           ┌──────────┬──────────┬───────┬──────────┬──────────┐
           │          │          │       │          │          │
    ┌──────▼──┐ ┌─────▼───┐ ┌───▼───┐ ┌─▼────┐ ┌──▼───┐ ┌───▼────┐
    │prompting│ │permiss- │ │  rag  │ │seal  │ │relay │ │provid- │
    │         │ │  ions   │ │       │ │      │ │      │ │  ers   │
    └─────────┘ └─────────┘ └───────┘ └──────┘ └──────┘ └────────┘
           ┌──────────┬──────────┬───────┬──────────┐
           │          │          │       │          │
    ┌──────▼──┐ ┌─────▼───┐ ┌───▼───┐ ┌─▼────┐ ┌──▼───┐
    │ skills  │ │  eval   │ │ proxy │ │ a2a  │ │ mesh │
    └─────────┘ └─────────┘ └───────┘ └──────┘ └──────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires = "0.1"  # default features: tools + agents
```

Then import via the prelude:

```rust
use brainwires::prelude::*;

let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::user("Hello!"),
];

let options = ChatOptions::deterministic(1024);
let response = provider.chat(&messages, None, &options).await?;
```

## Features

| Feature | Default | Activates | Description |
|---------|---------|-----------|-------------|
| `tools` | **yes** | `brainwires-model-tools` | File, bash, git, search, web, and validation tools |
| `agents` | **yes** | `brainwires-agents` | Agent runtime, communication hub, task manager, validation loop |
| `storage` | no | `brainwires-storage` | LanceDB-backed tiered memory (hot/warm/cold) |
| `mcp` | no | `brainwires-mcp` | MCP client for connecting to external MCP servers |
| `mdap` | no | `brainwires-mdap` | Multi-Dimensional Adaptive Planning with k-agent voting |
| `prompting` | no | `brainwires-prompting` | Prompt generation, technique library, temperature optimizer |
| `knowledge` | no | `brainwires-prompting/knowledge` | Behavioral + personal knowledge caches (implies `prompting`) |
| `permissions` | no | `brainwires-permissions` | Capability profiles, trust levels, policy engine, audit logging |
| `orchestrator` | no | `brainwires-model-tools/orchestrator` | Tool orchestration layer (implies `tools`) |
| `rag` | no | `brainwires-rag` | Semantic code search with vector + BM25 hybrid search |
| `interpreters` | no | `brainwires-code-interpreters` | Sandboxed JavaScript and Python code execution |
| `providers` | no | `brainwires-providers` | AI providers (Anthropic, OpenAI, Google, Ollama) |
| `reasoning` | no | `brainwires-agents/reasoning` | Extended reasoning support (implies `agents`) |
| `seal` | no | `brainwires-seal` | Self-Evolving Autonomous Learner |
| `relay` | no | `brainwires-relay` | Remote relay / bridge for IPC and remote control |
| `skills` | no | `brainwires-skills` | Pluggable skills system |
| `eval` | no | `brainwires-eval` | Evaluation framework for benchmarking agents |
| `proxy` | no | `brainwires-proxy` | AI proxy framework |
| `a2a` | no | `brainwires-a2a` | Agent-to-Agent protocol |
| `mesh` | no | `brainwires-mesh` | Mesh networking for distributed agents |
| `mcp-server` | no | `rmcp` + `schemars` + `tokio-util` | Re-exports for building MCP servers |
| `llama-cpp-2` | no | `brainwires-providers/llama-cpp-2` | Local LLM inference (implies `providers`) |

### Convenience Features

| Feature | Enables | Use Case |
|---------|---------|----------|
| `agent-full` | `agents` + `permissions` + `prompting` + `tools` | Complete agent workflow with permissions |
| `learning` | `seal` + `knowledge` + `brainwires-seal/knowledge` | Full learning subsystem with knowledge integration |
| `full` | Everything | Kitchen sink — all sub-crates and cross-crate features |

## Prelude

`use brainwires::prelude::*` brings in the most commonly needed types, grouped by subsystem:

**Core** (always available):
`Message`, `Role`, `ContentBlock`, `ChatResponse`, `StreamChunk`, `Usage`, `Tool`, `ToolUse`, `ToolResult`, `ToolContext`, `ToolInputSchema`, `Provider`, `ChatOptions`, `Task`, `TaskStatus`, `TaskPriority`, `PlanMetadata`, `PlanStatus`, `PermissionMode`, `EntityType`, `EdgeType`, `GraphNode`, `GraphEdge`, `EmbeddingProvider`, `VectorStore`, `WorkingSet`, `FrameworkError`, `FrameworkResult`

**Tools** (`tools` feature):
`BashTool`, `FileOpsTool`, `GitTool`, `SearchTool`, `WebTool`, `ValidationTool`, `ToolRegistry`, `ToolCategory`, `ToolErrorCategory`, `RetryStrategy`

**Agents** (`agents` feature):
`AgentRuntime`, `AgentExecutionResult`, `run_agent_loop`, `CommunicationHub`, `FileLockManager`, `TaskManager`, `TaskQueue`, `ValidationConfig`, `AccessControlManager`, `GitCoordinator`, `PlanExecutorAgent`

**Storage** (`storage` feature):
`TieredMemory`

**MCP** (`mcp` feature):
`McpClient`, `McpConfigManager`, `McpServerConfig`

**MDAP** (`mdap` feature):
`Composer`, `MdapEstimate`, `MicroagentConfig`, `FirstToAheadByKVoter`

**Knowledge** (`knowledge` feature):
`BehavioralKnowledgeCache`, `PersonalKnowledgeCache`, `BehavioralTruth`, `TruthCategory`

**Prompting** (`prompting` feature):
`PromptGenerator`, `PromptingTechnique`, `TechniqueLibrary`, `TemperatureOptimizer`, `TaskClusterManager`

**Permissions** (`permissions` feature):
`AgentCapabilities`, `PolicyEngine`, `TrustLevel`, `TrustManager`, `AuditLogger`, `PermissionsConfig`

## Usage Examples

### Agent Workflow

```toml
[dependencies]
brainwires = { version = "0.1", features = ["agent-full"] }
```

```rust
use brainwires::prelude::*;

// Set up the agent runtime
let hub = CommunicationHub::new();
let lock_manager = FileLockManager::new();
let runtime = AgentRuntime::new(hub, lock_manager);

// Define validation checks
let validation = ValidationConfig {
    checks: vec![ValidationCheck::FileExistence, ValidationCheck::Syntax],
    working_directory: "/my/project".into(),
    max_retries: 3,
    enabled: true,
    working_set_files: vec![],
};
```

### MCP Server with RAG

```toml
[dependencies]
brainwires = { version = "0.1", features = ["rag", "mcp-server"] }
```

```rust
use brainwires::rag::mcp_server::RagMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    RagMcpServer::serve_stdio().await?;
    Ok(())
}
```

### RAG Pipeline

```toml
[dependencies]
brainwires = { version = "0.1", features = ["rag"] }
```

```rust
use brainwires::rag::RagClient;

let client = RagClient::new(None).await?;
client.index("/path/to/project", None, None).await?;

let results = client.query("authentication logic", 10, 0.7).await?;
for result in results {
    println!("{}: {:.2}", result.file_path, result.score);
}
```

### Learning System

```toml
[dependencies]
brainwires = { version = "0.1", features = ["learning"] }
```

```rust
use brainwires::prelude::*;

let cache = BehavioralKnowledgeCache::new();
let truth = BehavioralTruth::new("always_use_async", TruthCategory::Pattern);
cache.store(truth);
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
