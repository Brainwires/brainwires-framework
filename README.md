# Brainwires Framework

A modular Rust framework for building AI agents with multi-provider support, tool orchestration, MCP integration, and distributed mesh networking.

## Overview

The Brainwires Framework is a workspace of 22 crates (plus 1 extra) that provide everything needed to build, train, deploy, and coordinate AI agents. Each crate is independently publishable to crates.io and usable standalone, but they compose together through the `brainwires` facade crate for a batteries-included experience.

**Key capabilities:**

- **Multi-provider AI** — Anthropic, OpenAI, Google, Ollama, and local LLMs behind a unified `Provider` trait
- **Agent orchestration** — hierarchical task decomposition, multi-agent coordination with file locks, MDAP voting
- **MCP protocol** — full client and server support via `rmcp`, exposing agents as MCP tools
- **Distributed mesh** — connect agents across processes and machines with topology-aware routing
- **Training pipelines** — cloud fine-tuning (6 providers) and local LoRA/QLoRA/DoRA via Burn
- **RAG & code search** — AST-aware chunking, hybrid vector + keyword search, Git-aware indexing
- **Audio** — speech-to-text, text-to-speech, hardware capture/playback
- **Security** — encrypted storage (ChaCha20-Poly1305), permission policies, content trust tagging

## Crate Map

```text
  ┌────────────────────────────────────────────────────────────┐
  │                        brainwires                          │
  │                      (facade crate)                        │
  │                                                            │
  │  ┌───────────┐ ┌────────────┐ ┌───────────┐ ┌───────────┐  │
  │  │  agents   │ │  providers │ │  storage  │ │    mcp    │  │
  │  │  mdap     │ │model-tools │ │  prompting│ │   relay   │  │
  │  └─────┬─────┘ └──────┬─────┘ └─────┬─────┘ └─────┬─────┘  │
  │        │              │             │             │        │
  │        └──────────────┴─────────────┴─────────────┘        │
  │                            │                               │
  │                     ┌──────▼──────┐                        │
  │                     │    core     │                        │
  │                     │ permissions │                        │
  │                     └─────────────┘                        │
  │                                                            │
  │  ┌──────────┐ ┌────────────┐ ┌───────────┐ ┌───────────┐   │
  │  │   rag    │ │  datasets  │ │ training  │ │   audio   │   │
  │  │  skills  │ │    eval    │ │   seal    │ │    wasm   │   │
  │  │code-inter│ │   mesh     │ │    a2a    │ │           │   │
  │  └──────────┘ └────────────┘ └───────────┘ └───────────┘   │
  └────────────────────────────────────────────────────────────┘
```

### All Crates

| Crate | Description |
|-------|-------------|
| **brainwires** | Facade crate — re-exports all other crates behind feature flags |
| **brainwires-core** | Core types, traits, and error handling shared by all crates |
| **brainwires-providers** | Multi-provider AI interface (Anthropic, OpenAI, Google, Ollama, local LLMs) |
| **brainwires-model-tools** | Tool definitions and execution for AI model interactions |
| **brainwires-agents** | Multi-agent orchestration, task decomposition, file lock coordination |
| **brainwires-mdap** | Multi-Dimensional Adaptive Planning — k-agent voting for reliable execution |
| **brainwires-brain** | Central knowledge crate — persistent thoughts, PKS/BKS, entity graphs, relationship graphs |
| **brainwires-storage** | LanceDB vector storage, semantic search, tiered memory |
| **brainwires-prompting** | Prompt construction, task clustering, multi-source selection (delegates knowledge to brain) |
| **brainwires-permissions** | Permission policies (auto, ask, reject) for tool execution |
| **brainwires-mcp** | MCP client — connect to external MCP servers and use their tools |
| **brainwires-relay** | MCP server mode, IPC, and remote relay for agent management |
| **brainwires-rag** | RAG engine — AST-aware chunking, hybrid search, Git-aware indexing (library-only) |
| **brainwires-skills** | Skill definitions and slash command registry |
| **brainwires-code-interpreters** | Sandboxed JavaScript and Python code execution |
| **brainwires-wasm** | WASM bindings for browser-based agent deployment |
| **brainwires-seal** | Self-Evolving Agentic Learning — feedback-driven prompt improvement |
| **brainwires-mesh** | Distributed agent mesh networking with topology and routing |
| **brainwires-audio** | Audio I/O, speech-to-text, text-to-speech |
| **brainwires-datasets** | Training data pipelines — JSONL I/O, tokenization, dedup, format conversion |
| **brainwires-training** | Cloud fine-tuning (6 providers) and local LoRA/QLoRA/DoRA via Burn |
| **brainwires-proxy** | HTTP proxy for AI API request routing *(extras/)* |
| **brainwires-brain-server** | MCP server binary for brainwires-brain *(extras/)* |
| **brainwires-rag-server** | MCP server binary for brainwires-rag *(extras/)* |

## Getting Started

### Requirements

- **Rust 1.88+** (edition 2024)
- **Cargo** (comes with Rust)

> **Note:** This framework uses `edition = "2024"` which requires Rust 1.88 or newer. Check your version with `rustc --version` and update with `rustup update stable` if needed.

### Using the Facade Crate

The simplest way to use the framework is through the `brainwires` facade crate, which re-exports everything behind feature flags:

```toml
[dependencies]
brainwires = "0.1"
```

Enable only what you need:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["agents", "providers", "rag"] }
```

### Using Individual Crates

Each crate is independently publishable and usable:

```toml
[dependencies]
brainwires-core = "0.1"
brainwires-providers = "0.1"
brainwires-agents = "0.1"
```

### Minimal Example

```rust
use brainwires::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a provider
    let provider = AnthropicProvider::new("your-api-key")?;

    // Send a message
    let messages = vec![Message::user("Hello, what can you do?")];
    let options = ChatOptions::default();
    let response = provider.chat(messages, options).await?;

    println!("{}", response.content);
    Ok(())
}
```

## Feature Flags

The `brainwires` facade crate exposes feature flags corresponding to each sub-crate:

| Feature | Default | What it enables |
|---------|---------|-----------------|
| `core` | Yes | Core types and traits |
| `providers` | Yes | AI provider integrations |
| `agents` | Yes | Multi-agent orchestration |
| `storage` | Yes | Vector storage and semantic search |
| `mcp` | Yes | MCP client support |
| `relay` | No | MCP server mode and IPC |
| `rag` | No | RAG engine with code search |
| `audio` | No | Audio capture, STT, TTS |
| `datasets` | No | Training data pipelines |
| `training` | No | Model fine-tuning (cloud + local) |
| `mesh` | No | Distributed agent mesh |
| `a2a` | No | Agent-to-Agent protocol |
| `wasm` | No | WASM browser bindings |
| `researcher` | No | Bundle: providers + agents + storage + rag + training + datasets |

## Building

```bash
# Build all crates (debug)
cargo build

# Build all crates (release)
cargo build --release

# Build a specific crate
cargo build -p brainwires-agents

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p brainwires-core
```

## Dependency DAG

```text
  brainwires (facade)
  ├── brainwires-agents
  │   ├── brainwires-core
  │   ├── brainwires-model-tools
  │   ├── brainwires-providers
  │   └── brainwires-permissions
  ├── brainwires-mdap
  │   └── brainwires-agents
  ├── brainwires-brain
  │   ├── brainwires-core
  │   └── brainwires-storage
  ├── brainwires-storage
  │   └── brainwires-core
  ├── brainwires-prompting
  │   ├── brainwires-core
  │   └── brainwires-brain (optional, knowledge feature)
  ├── brainwires-mcp
  │   └── brainwires-core
  ├── brainwires-relay
  │   ├── brainwires-core
  │   └── brainwires-mcp
  ├── brainwires-rag
  │   └── brainwires-core
  ├── brainwires-training
  │   ├── brainwires-core
  │   ├── brainwires-datasets
  │   └── brainwires-providers (cloud feature)
  ├── brainwires-mesh
  │   ├── brainwires-core
  │   └── brainwires-relay (a2a feature)
  └── brainwires-audio
      (standalone — no internal deps beyond core traits)
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
