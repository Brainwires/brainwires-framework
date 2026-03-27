# Brainwires Framework

A modular Rust framework for building AI agents with multi-provider support, tool orchestration, MCP integration, and pluggable agent networking.

## Overview

The Brainwires Framework is a workspace of 20 framework crates plus 13 extras that provide everything needed to build, train, deploy, and coordinate AI agents. Each framework crate is independently publishable to crates.io and usable standalone, but they compose together through the `brainwires` facade crate for a batteries-included experience.

**[Full feature list](FEATURES.md)** | **Key capabilities:**

- **Multi-provider AI** — Anthropic, OpenAI, Google, Ollama, and local LLMs behind a unified `Provider` trait
- **Agent orchestration** — hierarchical task decomposition, multi-agent coordination with file locks, MDAP voting
- **MCP protocol** — full client and server support via `rmcp`, exposing agents as MCP tools
- **Agent networking** — 5-layer protocol stack (IPC, TCP, A2A, Pub/Sub) with pluggable transports, routing, and discovery
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
  │  │  mdap     │ │tool-system │ │ cognition │ │agent-net  │  │
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
  │  │  skills  │ │  datasets  │ │ training  │ │   audio   │   │
  │  │code-inter│ │  autonomy  │ │    a2a    │ │    wasm   │   │
  │  └──────────┘ └────────────┘ └───────────┘ └───────────┘   │
  │                                                            │
  │  ┌──────────────┐ ┌────────────────────────────────────┐   │
  │  │  channels    │ │        mcp-server                  │   │
  │  └──────────────┘ └────────────────────────────────────┘   │
  └────────────────────────────────────────────────────────────┘
```

### Framework Crates

| Crate | Description |
|-------|-------------|
| [**brainwires**](crates/brainwires/README.md) | Facade crate — re-exports all other crates behind feature flags |
| [**brainwires-core**](crates/brainwires-core/README.md) | Core types, traits, and error handling shared by all crates |
| [**brainwires-providers**](crates/brainwires-providers/README.md) | Multi-provider AI interface (Anthropic, OpenAI, Google, Ollama, local LLMs) |
| [**brainwires-tool-system**](crates/brainwires-tool-system/README.md) | Tool definitions and execution for AI model interactions |
| [**brainwires-agents**](crates/brainwires-agents/README.md) | Multi-agent orchestration, task decomposition, file lock coordination |
| [**brainwires-cognition**](crates/brainwires-cognition/README.md) | Knowledge (BKS/PKS, entity graphs), prompting (technique library, clustering), and RAG (code search, hybrid retrieval) |
| [**brainwires-storage**](crates/brainwires-storage/README.md) | Unified database layer (9 backends), tiered memory, semantic search |
| [**brainwires-permissions**](crates/brainwires-permissions/README.md) | Permission policies (auto, ask, reject) for tool execution |
| [**brainwires-mcp**](crates/brainwires-mcp/README.md) | MCP client — connect to external MCP servers and use their tools |
| [**brainwires-agent-network**](crates/brainwires-agent-network/README.md) | Agent networking — MCP server, IPC, remote bridge, 5-layer protocol stack (transport, routing, discovery) |
| [**brainwires-skills**](crates/brainwires-skills/README.md) | Skill definitions and slash command registry |
| [**brainwires-code-interpreters**](crates/brainwires-code-interpreters/README.md) | Sandboxed JavaScript and Python code execution |
| [**brainwires-wasm**](crates/brainwires-wasm/README.md) | WASM bindings for browser-based agent deployment |
| [**brainwires-audio**](crates/brainwires-audio/README.md) | Audio I/O, speech-to-text, text-to-speech |
| [**brainwires-datasets**](crates/brainwires-datasets/README.md) | Training data pipelines — JSONL I/O, tokenization, dedup, format conversion |
| [**brainwires-training**](crates/brainwires-training/README.md) | Cloud fine-tuning (6 providers) and local LoRA/QLoRA/DoRA via Burn |
| [**brainwires-autonomy**](crates/brainwires-autonomy/README.md) | Self-improvement strategies, evaluation-driven optimization, supervisor agents |
| [**brainwires-a2a**](crates/brainwires-a2a/README.md) | Agent-to-Agent protocol — JSON-RPC 2.0, HTTP/REST, and gRPC bindings |
| [**brainwires-channels**](crates/brainwires-channels/README.md) | Universal messaging channel contract — `Channel` trait, message types, events, capabilities |
| [**brainwires-mcp-server**](crates/brainwires-mcp-server/README.md) | MCP server framework — build MCP tool servers with composable middleware |

### Extras

| Crate | Description |
|-------|-------------|
| [**brainwires-proxy**](extras/brainwires-proxy/README.md) | HTTP proxy for AI API request routing |
| [**brainwires-brain-server**](extras/brainwires-brain-server/README.md) | MCP server binary for brainwires-brain |
| [**brainwires-rag-server**](extras/brainwires-rag-server/README.md) | MCP server binary for brainwires-rag |
| [**agent-chat**](extras/agent-chat/README.md) | Simplified AI chat client with TUI and plain modes |
| [**reload-daemon**](extras/reload-daemon/README.md) | MCP server for killing and restarting AI coding clients |
| [**audio-demo-ffi**](extras/audio-demo-ffi/README.md) | UniFFI bindings (cdylib) exposing brainwires-audio to C#, Kotlin, Swift, Python |
| [**audio-demo**](extras/audio-demo/README.md) | Cross-platform Avalonia GUI for TTS/STT demo across all audio providers |
| [**brainclaw**](extras/brainclaw/daemon/README.md) | Self-hosted personal AI assistant daemon — multi-provider, per-user sessions, secure gateway |
| [**brainwires-gateway**](extras/brainclaw/gateway/README.md) | WebSocket/HTTP channel hub — routes channel adapters to AI agent sessions |
| [**brainwires-discord-channel**](extras/brainclaw/mcp-discord/README.md) | Discord channel adapter — reference `Channel` trait implementation, optional MCP server mode |
| [**brainwires-telegram-channel**](extras/brainclaw/mcp-telegram/README.md) | Telegram channel adapter — teloxide-based, optional MCP server mode |
| [**brainwires-slack-channel**](extras/brainclaw/mcp-slack/README.md) | Slack channel adapter — Socket Mode (no public URL), optional MCP server mode |
| [**brainwires-skill-registry**](extras/brainclaw/mcp-skill-registry/README.md) | Skill registry HTTP server — SQLite FTS5, publish/search/download endpoints |

## Getting Started

### Requirements

- **Rust 1.91+** (edition 2024)
- **Cargo** (comes with Rust)

> **Note:** This framework uses `edition = "2024"` which requires Rust 1.91 or newer. Check your version with `rustc --version` and update with `rustup update stable` if needed.

### Using the Facade Crate

The simplest way to use the framework is through the `brainwires` facade crate, which re-exports everything behind feature flags:

```toml
[dependencies]
brainwires = "0.6"  # defaults: tools + agents
```

Enable only what you need:

```toml
[dependencies]
brainwires = { version = "0.6", features = ["providers", "rag"] }
```

### Using Individual Crates

Each crate is independently publishable and usable:

```toml
[dependencies]
brainwires-core = "0.6"
brainwires-providers = "0.6"
brainwires-agents = "0.6"
```

### Minimal Example

```rust
use brainwires::prelude::*;
use brainwires::providers::{ChatProviderFactory, ProviderConfig, ProviderType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a provider via the factory
    let config = ProviderConfig {
        provider: ProviderType::Anthropic,
        model: "claude-sonnet-4-20250514".into(),
        api_key: Some("your-api-key".into()),
        base_url: None,
    };
    let provider = ChatProviderFactory::create(&config)?;

    // Send a message
    let messages = vec![Message::user("Hello, what can you do?")];
    let options = ChatOptions::default();
    let response = provider.chat(&messages, None, &options).await?;

    println!("{}", response.message.content);
    Ok(())
}
```

## Feature Flags

The `brainwires` facade crate exposes feature flags corresponding to each sub-crate:

| Feature | Default | What it enables |
|---------|---------|-----------------|
| `core` | Always | Core types and traits (not feature-gated) |
| `tools` | Yes | Tool definitions and execution |
| `agents` | Yes | Multi-agent orchestration |
| `providers` | No | AI provider integrations |
| `storage` | No | Vector storage and semantic search |
| `mcp` | No | MCP client support |
| `agent-network` | No | Agent networking (IPC, remote bridge, 5-layer protocol stack) |
| `channels` | No | Messaging channel contract types (Channel trait, message/event types) |
| `mcp-server-framework` | No | MCP server building blocks (McpServer, McpHandler, middleware pipeline) |
| `rag` | No | RAG engine with code search |
| `audio` | No | Audio capture, STT, TTS |
| `datasets` | No | Training data pipelines |
| `training` | No | Model fine-tuning (cloud + local) |
| `mesh` | No | Mesh networking (via `agent-network` mesh feature) |
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
  │   ├── brainwires-tool-system
  │   └── brainwires-cognition (seal-knowledge feature)
  ├── brainwires-cognition
  │   ├── brainwires-core
  │   └── brainwires-storage (knowledge feature)
  ├── brainwires-storage
  │   └── brainwires-core
  ├── brainwires-mcp
  │   └── brainwires-core
  ├── brainwires-agent-network
  │   ├── brainwires-core
  │   ├── brainwires-mcp
  │   ├── brainwires-mcp-server
  │   └── brainwires-a2a (a2a-transport feature)
  ├── brainwires-mcp-server
  │   └── brainwires-mcp
  ├── brainwires-channels
  │   ├── brainwires-core
  │   └── brainwires-agent-network (agent-network feature, optional)
  ├── brainwires-training
  │   ├── brainwires-core
  │   ├── brainwires-datasets
  │   └── brainwires-providers (cloud feature)
  └── brainwires-audio
      (standalone — no internal deps beyond core traits)
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
