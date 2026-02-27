# brainwires-bridge-client

[![Crates.io](https://img.shields.io/crates/v/brainwires-bridge-client.svg)](https://crates.io/crates/brainwires-bridge-client)
[![Documentation](https://img.shields.io/docsrs/brainwires-bridge-client)](https://docs.rs/brainwires-bridge-client)
[![License](https://img.shields.io/crates/l/brainwires-bridge-client.svg)](LICENSE)

MCP bridge client for spawning and communicating with brainwires MCP server processes.

## Overview

`brainwires-bridge-client` is a lightweight client library that spawns a Brainwires MCP server as a child process and communicates with it over stdin/stdout using the JSON-RPC 2.0 protocol. It provides both low-level request/response methods and high-level ergonomic helpers for agent management.

**Design principles:**

- **Process-based** — spawns MCP servers as child processes, communicates via piped stdio
- **Stateful protocol** — enforces MCP lifecycle (initialize before tool calls)
- **Flexible parsing** — tolerates multiple response formats from the server
- **Minimal dependencies** — only `tokio`, `serde`, and `brainwires-mcp`

```text
  ┌───────────────────────────────────────────┐
  │           BridgeClient                    │
  │                                           │
  │  connect() ──► spawn child process        │
  │                    │                      │
  │                    ▼                      │
  │  initialize() ──► MCP handshake           │
  │                    │                      │
  │                    ▼                      │
  │  call_tool() ──► JSON-RPC 2.0 ──► stdin   │
  │                                    │      │
  │  result     ◄── JSON-RPC 2.0 ◄── stdout   │
  │                                           │
  │  ┌─────────────────────────────────────┐  │
  │  │  High-Level Agent Helpers           │  │
  │  │  spawn_agent / await_agent /        │  │
  │  │  list_agents / stop_agent           │  │
  │  └─────────────────────────────────────┘  │
  │                                           │
  │  shutdown() ──► close stdin, wait exit    │
  └───────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-bridge-client = "0.1"
```

Spawn an MCP server and call a tool:

```rust
use brainwires_bridge_client::BridgeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BridgeClient::connect("brainwires").await?;
    client.initialize().await?;

    let tools = client.list_tools().await?;
    println!("Available tools: {}", tools);

    let result = client.call_tool("agent_list", serde_json::json!({})).await?;
    println!("Agents: {}", result);

    client.shutdown().await?;
    Ok(())
}
```

## Architecture

### Client Lifecycle

`BridgeClient` manages the full lifecycle of a child MCP server process.

**Lifecycle phases:**

| Phase | Method | Description |
|-------|--------|-------------|
| Connect | `connect()` / `connect_with_args()` | Spawn child process, pipe stdio |
| Initialize | `initialize()` | MCP protocol handshake (version, capabilities) |
| Use | `call_tool()` / `list_tools()` / `send_request()` | JSON-RPC method calls |
| Shutdown | `shutdown()` | Close stdin, wait for process exit |

**`BridgeClient` methods:**

| Method | Requires Init | Description |
|--------|:------------:|-------------|
| `connect(binary)` | No | Spawn with default args `["chat", "--mcp-server"]` |
| `connect_with_args(binary, args)` | No | Spawn with custom args |
| `initialize()` | No | MCP handshake, sets `initialized = true` |
| `send_request(method, params)` | No | Low-level JSON-RPC request |
| `call_tool(name, args)` | Yes | Call an MCP tool |
| `list_tools()` | Yes | List available tools |
| `shutdown(self)` | No | Graceful shutdown with 5s timeout |
| `is_initialized()` | No | Check initialization state |

### Agent Operations

High-level helpers for common agent management workflows, built on `call_tool()`.

**`AgentConfig`** — optional spawn configuration:

| Field | Type | Description |
|-------|------|-------------|
| `max_iterations` | `Option<u32>` | Provider call iteration limit |
| `enable_validation` | `Option<bool>` | Run quality checks before completion |
| `build_type` | `Option<String>` | Build system for validation (`"cargo"`, `"typescript"`) |
| `enable_mdap` | `Option<bool>` | Enable multi-agent voting |
| `mdap_preset` | `Option<String>` | MDAP preset (`"default"`, `"high_reliability"`) |

All fields are optional — only set values are sent to the server.

**Agent methods on `BridgeClient`:**

| Method | MCP Tool | Returns |
|--------|----------|---------|
| `spawn_agent(desc, dir, config)` | `agent_spawn` | `String` (agent ID) |
| `await_agent(id, timeout)` | `agent_await` | `AgentResult` |
| `list_agents()` | `agent_list` | `Vec<AgentInfo>` |
| `stop_agent(id)` | `agent_stop` | `()` |
| `get_agent_status(id)` | `agent_status` | `AgentInfo` |

**`AgentResult`:**

```rust
pub struct AgentResult {
    pub agent_id: String,
    pub success: bool,
    pub iterations: u32,
    pub summary: String,
    pub raw_output: String,
}
```

**`AgentInfo`:**

```rust
pub struct AgentInfo {
    pub agent_id: String,
    pub status: String,
    pub task_description: String,
}
```

### Protocol Module

Helper functions for building MCP protocol messages:

| Function | Description |
|----------|-------------|
| `build_initialize_request(id)` | MCP initialize with protocol version `"2024-11-05"` |
| `build_initialized_notification()` | Post-init notification (no response expected) |
| `build_tools_list_request(id)` | `tools/list` request |
| `build_tools_call_request(id, name, args)` | `tools/call` request |
| `parse_response(line)` | Deserialize JSON-RPC response line |
| `extract_result(response)` | Extract result or convert error |

### Error Handling

**`BridgeClientError` variants:**

| Variant | Description |
|---------|-------------|
| `SpawnFailed(io::Error)` | Child process failed to start |
| `ProcessExited` | Child process terminated unexpectedly |
| `Protocol(String)` | Protocol-level error |
| `JsonRpc { code, message }` | JSON-RPC error with code and message |
| `Timeout(u64)` | Operation timed out (seconds) |
| `NotInitialized` | `call_tool` / `list_tools` called before `initialize()` |
| `Io(io::Error)` | I/O error |
| `Json(serde_json::Error)` | JSON serialization error |

## Usage Examples

### Connect with Custom Arguments

```rust
use brainwires_bridge_client::BridgeClient;

let mut client = BridgeClient::connect_with_args(
    "/usr/local/bin/brainwires",
    &["chat", "--mcp-server", "--model", "claude-sonnet"],
).await?;

client.initialize().await?;
```

### Spawn and Await an Agent

```rust
use brainwires_bridge_client::{BridgeClient, AgentConfig};

let mut client = BridgeClient::connect("brainwires").await?;
client.initialize().await?;

let agent_id = client.spawn_agent(
    "Implement LRU cache in src/cache.rs",
    "/my/project",
    AgentConfig {
        max_iterations: Some(20),
        enable_validation: Some(true),
        build_type: Some("cargo".into()),
        ..Default::default()
    },
).await?;

let result = client.await_agent(&agent_id, Some(300)).await?;
println!(
    "Agent {} finished in {} iterations: {}",
    result.agent_id, result.iterations, result.summary
);
```

### List and Monitor Agents

```rust
use brainwires_bridge_client::BridgeClient;

let mut client = BridgeClient::connect("brainwires").await?;
client.initialize().await?;

let agents = client.list_agents().await?;
for agent in &agents {
    println!("{}: {} ({})", agent.agent_id, agent.task_description, agent.status);
}

if let Some(agent) = agents.first() {
    let status = client.get_agent_status(&agent.agent_id).await?;
    println!("Current status: {}", status.status);
}
```

### Low-Level JSON-RPC

```rust
use brainwires_bridge_client::BridgeClient;

let mut client = BridgeClient::connect("brainwires").await?;
client.initialize().await?;

// Call any JSON-RPC method directly
let result = client.send_request(
    "tools/call",
    Some(serde_json::json!({
        "name": "agent_pool_stats",
        "arguments": {}
    })),
).await?;

println!("Pool stats: {}", result);
```

### Spawn with MDAP

```rust
use brainwires_bridge_client::{BridgeClient, AgentConfig};

let mut client = BridgeClient::connect("brainwires").await?;
client.initialize().await?;

let agent_id = client.spawn_agent(
    "Implement Dijkstra's shortest path algorithm",
    "/my/project",
    AgentConfig {
        max_iterations: Some(25),
        enable_validation: Some(true),
        build_type: Some("cargo".into()),
        enable_mdap: Some(true),
        mdap_preset: Some("high_reliability".into()),
    },
).await?;

let result = client.await_agent(&agent_id, Some(600)).await?;
```

### Graceful Shutdown

```rust
use brainwires_bridge_client::BridgeClient;

let mut client = BridgeClient::connect("brainwires").await?;
client.initialize().await?;

// ... do work ...

// Closes stdin, waits up to 5 seconds for process exit
client.shutdown().await?;
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["bridge-client"] }
```

Or use standalone — `brainwires-bridge-client` depends only on `brainwires-mcp` for protocol types.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
