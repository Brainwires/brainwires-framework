# Brainwires Framework

[![CI](https://github.com/Brainwires/brainwires-framework/actions/workflows/ci.yml/badge.svg)](https://github.com/Brainwires/brainwires-framework/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/brainwires.svg)](https://crates.io/crates/brainwires)
[![Documentation](https://docs.rs/brainwires/badge.svg)](https://docs.rs/brainwires)
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen)](#testing)
[![Lines of Code](https://img.shields.io/badge/loc-112k-blue)](#)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)](https://github.com/Brainwires/brainwires-framework/blob/main/LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.91%2B%20%7C%202024%20edition-orange)](https://www.rust-lang.org/)

A modular Rust framework for building AI agents with multi-provider support, tool orchestration, MCP integration, and pluggable agent networking.

**Warning:** This is an early-stage project under active development. Expect breaking changes and rapid iteration as we build towards a 1.0 release.

## Overview

The Brainwires Framework is a workspace of 16 framework crates plus 25 extras (including the 7-crate `brainclaw` set) that provide everything needed to build, train, deploy, and coordinate AI agents. Each framework crate is independently publishable to crates.io and usable standalone, but they compose together through the `brainwires` facade crate for a batteries-included experience.

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
  │  │  mdap     │ │   tools    │ │ knowledge │ │  network  │  │
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
  │  │reasoning │ │  training  │ │ telemetry │ │   audio   │   │
  │  │ hardware │ │    a2a     │ │mcp-server │ │           │   │
  │  └──────────┘ └────────────┘ └───────────┘ └───────────┘   │
  └────────────────────────────────────────────────────────────┘
```

### Framework Crates

| Crate | Description |
|-------|-------------|
| [**brainwires**](crates/brainwires/README.md) | Facade crate — re-exports all other crates behind feature flags |
| [**brainwires-core**](crates/brainwires-core/README.md) | Core types, traits, and error handling shared by all crates |
| [**brainwires-providers**](crates/brainwires-providers/README.md) | Multi-provider AI interface (Anthropic, OpenAI, Google, Ollama, local LLMs) |
| [**brainwires-tools**](crates/brainwires-tools/README.md) | Tool definitions, execution, interpreters, and system primitives for AI model interactions |
| [**brainwires-agents**](crates/brainwires-agents/README.md) | Multi-agent orchestration, task decomposition, file lock coordination, skills and slash commands |
| [**brainwires-knowledge**](crates/brainwires-knowledge/README.md) | Knowledge (BKS/PKS, entity graphs), prompting (technique library, clustering), and RAG (code search, hybrid retrieval) |
| [**brainwires-storage**](crates/brainwires-storage/README.md) | Unified database layer (9 backends), embeddings, BM25 keyword search, file-context primitives |
| [**brainwires-memory**](crates/brainwires-memory/README.md) | Tiered hot/warm/cold agent memory — `MessageStore`, `SummaryStore`, `FactStore`, `MentalModelStore`, `TieredMemory` orchestration |
| [**brainwires-permissions**](crates/brainwires-permissions/README.md) | Permission policies (auto, ask, reject) for tool execution |
| [**brainwires-mcp**](crates/brainwires-mcp/README.md) | MCP client — connect to external MCP servers and use their tools |
| [**brainwires-network**](crates/brainwires-network/README.md) | Agent networking — MCP server, IPC, remote bridge, channels, WebRTC, 5-layer protocol stack |
| [**brainwires-reasoning**](crates/brainwires-reasoning/README.md) | Reasoning strategies — re-exports from core |
| [**brainwires-hardware**](crates/brainwires-hardware/README.md) | Hardware I/O — audio (STT/TTS), GPIO, Bluetooth, Matter (experimental PASE), home automation, camera/webcam, raw USB |
| [**brainwires-training**](crates/brainwires-training/README.md) | Training data pipelines, cloud fine-tuning (6 providers), and local LoRA/QLoRA/DoRA via Burn |
| [**brainwires-telemetry**](crates/brainwires-telemetry/README.md) | OutcomeMetrics, Prometheus export, billing hooks |
| [**brainwires-a2a**](crates/brainwires-a2a/README.md) | Agent-to-Agent protocol — JSON-RPC 2.0, HTTP/REST, and gRPC bindings |
| [**brainwires-mcp-server**](crates/brainwires-mcp-server/README.md) | MCP server framework — build MCP tool servers with composable middleware; `http` feature adds stateless HTTP+SSE transport, Server Cards (SEP-1649), RFC9728, and Tasks (SEP-1686); `oauth` feature adds JWT validation middleware |

### Extras

| Crate | Description |
|-------|-------------|
| [**brainwires-proxy**](extras/brainwires-proxy/README.md) | HTTP proxy for AI API request routing |
| [**brainwires-brain-server**](extras/brainwires-brain-server/README.md) | MCP server binary exposing the `brainwires-knowledge::knowledge` subsystem (BKS/PKS, thoughts, entity graphs) |
| [**brainwires-rag-server**](extras/brainwires-rag-server/README.md) | MCP server binary exposing the `brainwires-knowledge::rag` subsystem (codebase indexing + hybrid search) |
| [**agent-chat**](extras/agent-chat/README.md) | Simplified AI chat client with TUI and plain modes |
| [**reload-daemon**](extras/reload-daemon/README.md) | MCP server for killing and restarting AI coding clients |
| [**audio-demo-ffi**](extras/audio-demo-ffi/README.md) | UniFFI bindings (cdylib) exposing brainwires-hardware (audio) to C#, Kotlin, Swift, Python |
| [**audio-demo**](extras/audio-demo/README.md) | Cross-platform Avalonia GUI for TTS/STT demo across all audio providers |
| [**brainclaw**](extras/brainclaw/daemon/README.md) | Self-hosted personal AI assistant daemon — multi-provider, per-user sessions, secure gateway |
| [**brainwires-gateway**](extras/brainclaw/gateway/README.md) | WebSocket/HTTP channel hub — routes channel adapters to AI agent sessions |
| [**brainwires-discord-channel**](extras/brainclaw/mcp-discord/README.md) | Discord channel adapter — reference `Channel` trait implementation, optional MCP server mode |
| [**brainwires-telegram-channel**](extras/brainclaw/mcp-telegram/README.md) | Telegram channel adapter — teloxide-based, optional MCP server mode |
| [**brainwires-slack-channel**](extras/brainclaw/mcp-slack/README.md) | Slack channel adapter — Socket Mode (no public URL), optional MCP server mode |
| [**brainwires-skill-registry**](extras/brainclaw/mcp-skill-registry/README.md) | Skill registry HTTP server — SQLite FTS5, publish/search/download endpoints |
| [**brainclaw-mcp-github**](extras/brainclaw/mcp-github/README.md) | GitHub channel adapter — webhook receiver, REST API, MCP server mode |
| [**brainwires-memory-server**](extras/brainwires-memory-server/README.md) | Mem0-compatible memory REST API backed by Brainwires knowledge |
| [**matter-tool**](extras/matter-tool/README.md) | Brainwires-native Matter CLI (experimental) — PASE pairing, mDNS discovery, basic control |
| [**claude-brain**](extras/claude-brain/README.md) | Brainwires context management for Claude Code — persistent context across compaction |
| [**brainwires-cli**](extras/brainwires-cli/README.md) | AI-powered agentic CLI tool for autonomous coding assistance |
| [**brainwires-issues**](extras/brainwires-issues/README.md) | MCP-native issue tracking server |
| [**brainwires-scheduler**](extras/brainwires-scheduler/README.md) | MCP server for cron scheduling |
| [**brainwires-autonomy**](extras/brainwires-autonomy/README.md) | Autonomous agent operations |
| [**brainwires-wasm**](extras/brainwires-wasm/README.md) | WASM browser bindings |
| [**brainwires-billing**](extras/brainwires-billing/README.md) | Billing and cost accounting hooks for agent telemetry |
| [**brainwires-docs**](extras/brainwires-docs/README.md) | Documentation tooling and reference site generation |
| [**voice-assistant**](extras/voice-assistant/README.md) | End-to-end voice assistant binary using the `brainwires-hardware` pipeline |

## Getting Started

### Requirements

- **Rust 1.91+** (edition 2024)
- **Cargo** (comes with Rust)

> **Note:** This framework uses `edition = "2024"` which requires Rust 1.91 or newer. Check your version with `rustc --version` and update with `rustup update stable` if needed.

### Using the Facade Crate

The simplest way to use the framework is through the `brainwires` facade crate, which re-exports everything behind feature flags:

```toml
[dependencies]
brainwires = "0.10"  # defaults: tools + agents
```

Enable only what you need:

```toml
[dependencies]
brainwires = { version = "0.10", features = ["providers", "rag"] }
```

### Using Individual Crates

Each crate is independently publishable and usable:

```toml
[dependencies]
brainwires-core = "0.10"
brainwires-providers = "0.10"
brainwires-agents = "0.10"
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
        model: "claude-sonnet-4-6".into(),
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
| `tools` | Yes | Tool definitions, execution, and interpreters (`brainwires-tools`) |
| `agents` | Yes | Multi-agent orchestration, skills (`brainwires-agents`) |
| `providers` | No | AI provider integrations |
| `storage` | No | Vector storage and semantic search |
| `mcp` | No | MCP client support |
| `agent-network` | No | Agent networking — IPC, remote bridge, channels, 5-layer protocol stack (`brainwires-network`) |
| `mcp-server-framework` | No | MCP server building blocks (McpServer, McpHandler, middleware pipeline) |
| `rag` | No | RAG engine with code search |
| `audio` | No | Audio capture, STT, TTS |
| `training` | No | Model fine-tuning (cloud + local) |
| `datasets` | No | Training data pipelines (delegates to `brainwires-training`) |
| `telemetry` | No | OutcomeMetrics, Prometheus export, billing hooks |
| `reasoning` | No | Reasoning strategies (re-exports from core) |
| `mesh` | No | Mesh networking (via `agent-network` mesh feature) |
| `a2a` | No | Agent-to-Agent protocol |
| `wasm` | No | WASM core bindings |
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
  │   ├── brainwires-tools
  │   └── brainwires-knowledge (seal-knowledge feature)
  ├── brainwires-knowledge
  │   ├── brainwires-core
  │   └── brainwires-storage (knowledge feature)
  ├── brainwires-storage
  │   └── brainwires-core
  ├── brainwires-memory
  │   ├── brainwires-core
  │   └── brainwires-storage
  ├── brainwires-mcp
  │   └── brainwires-core
  ├── brainwires-network
  │   ├── brainwires-core
  │   ├── brainwires-mcp
  │   ├── brainwires-mcp-server
  │   └── brainwires-a2a (a2a-transport feature)
  ├── brainwires-mcp-server
  │   └── brainwires-mcp
  ├── brainwires-training
  │   ├── brainwires-core
  │   └── brainwires-providers (cloud feature)
  ├── brainwires-telemetry
  │   └── brainwires-core
  └── brainwires-hardware
      ├── brainwires-providers (audio feature, optional)
      └── (standalone for gpio, bluetooth, matter, home automation)
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
