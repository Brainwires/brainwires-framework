# brainwires-agent-network

[![Crates.io](https://img.shields.io/crates/v/brainwires-agent-network.svg)](https://crates.io/crates/brainwires-agent-network)
[![Documentation](https://img.shields.io/docsrs/brainwires-agent-network)](https://docs.rs/brainwires-agent-network)
[![License](https://img.shields.io/crates/l/brainwires-agent-network.svg)](LICENSE)

Agent networking layer for the Brainwires Agent Framework.

## Overview

`brainwires-agent-network` provides the full networking stack for AI agents: an MCP server framework with composable middleware, encrypted IPC for local inter-agent communication, a remote bridge for backend connectivity, agent lifecycle management, and optional distributed mesh networking.

**Design principles:**

- **Trait-driven** — `McpHandler`, `KeyStore`, `AgentSpawner`, and friends decouple the framework from any concrete CLI implementation
- **Middleware-composable** — auth, rate limiting, logging, and tool filtering stack via an onion model
- **Encryption-first** — IPC sockets use ChaCha20-Poly1305 authenticated encryption by default
- **Dual-mode remote** — Supabase Realtime (preferred) with automatic HTTP polling fallback
- **Mesh-ready** — optional topology, routing, discovery, and federation for multi-node coordination

```text
              ┌───────────────────────────────────────────────────────────┐
              │              brainwires-agent-network                     │
              │                                                           │
              │  ┌─────────────────────────────────────────────────────┐  │
              │  │             MCP Server Framework                    │  │
              │  │                                                     │  │
              │  │  McpHandler ──► MiddlewareChain ──► Transport       │  │
              │  │                   (onion model)                     │  │
              │  │                                                     │  │
              │  │  ┌───────────┐  ┌─────────────────┐                │  │
              │  │  │ToolReg-   │  │ Auth │ Rate │Log │                │  │
              │  │  │  istry    │  │ Limit│ Tool │    │                │  │
              │  │  └───────────┘  └─────────────────┘                │  │
              │  └─────────────────────────────────────────────────────┘  │
              │                                                           │
              │  ┌──────────────────┐  ┌───────────────────────────┐     │
              │  │  IPC Layer       │  │  Remote Bridge            │     │
              │  │  Unix Socket     │  │  Supabase Realtime /      │     │
              │  │  + ChaCha20      │  │  HTTP Polling Fallback    │     │
              │  │  + Discovery     │  │  + Priority Queue         │     │
              │  └──────────────────┘  └───────────────────────────┘     │
              │                                                           │
              │  ┌──────────────────┐  ┌───────────────────────────┐     │
              │  │  Agent Manager   │  │  Mesh Networking          │     │
              │  │  Spawn / List /  │  │  Topology / Routing /     │     │
              │  │  Stop / Await    │  │  Discovery / Federation   │     │
              │  └──────────────────┘  └───────────────────────────┘     │
              └───────────────────────────────────────────────────────────┘
```

## Quick Start

```toml
[dependencies]
brainwires-agent-network = "0.2"
```

Minimal MCP server:

```rust
use brainwires_agent_network::prelude::*;

struct MyHandler;

#[async_trait]
impl McpHandler for MyHandler {
    fn server_info(&self) -> ServerInfo {
        ServerInfo { name: "my-server".into(), version: "0.2.0".into() }
    }

    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities::default()
    }

    fn list_tools(&self) -> Vec<McpToolDef> {
        vec![]
    }

    async fn call_tool(
        &self,
        name: &str,
        args: Value,
        ctx: &RequestContext,
    ) -> Result<CallToolResult> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::new(MyHandler)
        .with_transport(StdioServerTransport::new())
        .with_middleware(LoggingMiddleware::new());

    server.run().await?;
    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `server` | Yes | MCP server framework |
| `client` | Yes | Client for connecting to remote agent network servers |
| `mesh` | No | Distributed mesh networking (topology, routing, discovery, federation) |
| `auth-keyring` | No | Secure API key storage via system keyring |

```toml
# With mesh networking and keyring
brainwires-agent-network = { version = "0.2", features = ["mesh", "auth-keyring"] }
```

## Architecture

### MCP Server Framework

The framework follows the Model Context Protocol specification for tool-based AI server integration.

**Key types:**

- `McpHandler` — defines server identity, capabilities, and tool dispatch
- `McpServer<H>` — wires handler, middleware chain, and transport together
- `McpToolRegistry` — declarative tool registration with automatic dispatch
- `ServerTransport` / `StdioServerTransport` — request/response I/O

### Middleware

Middleware follows an **onion model**: requests flow forward through layers, responses flow back.

| Layer | Description |
|-------|-------------|
| `AuthMiddleware` | Bearer token validation |
| `RateLimitMiddleware` | Token-bucket rate limiter with per-tool limits |
| `LoggingMiddleware` | Structured request/response logging via `tracing` |
| `ToolFilterMiddleware` | Allow-list or deny-list for tool access |

### IPC (Inter-Process Communication)

Local agent-to-agent communication over Unix domain sockets with authenticated encryption.

1. `IpcConnection::connect(socket_path)` — plain-text connection
2. `Handshake` exchange — session ID, token, model, working directory
3. `connection.upgrade_to_encrypted(session_token)` — ChaCha20-Poly1305 from this point

**Agent discovery:** `list_agent_sessions()`, `is_agent_alive()`, `cleanup_stale_sockets()`, `format_agent_tree()`

### Authentication

Session-based authentication with optional keyring storage via `AuthClient` and `SessionManager`.

### Agent Manager

Trait-based agent lifecycle for MCP server mode.

```rust
#[async_trait]
pub trait AgentManager: Send + Sync {
    async fn spawn_agent(&self, config: SpawnConfig) -> Result<String>;
    async fn list_agents(&self) -> Result<Vec<AgentInfo>>;
    async fn agent_status(&self, agent_id: &str) -> Result<AgentInfo>;
    async fn stop_agent(&self, agent_id: &str) -> Result<()>;
    async fn await_agent(&self, agent_id: &str, timeout_secs: Option<u64>) -> Result<AgentResult>;
    async fn pool_stats(&self) -> Result<Value>;
    async fn file_locks(&self) -> Result<Value>;
}
```

`AgentToolRegistry` registers 10 MCP tools for agent management (`agent_spawn`, `agent_list`, `agent_status`, `agent_stop`, `agent_await`, `agent_pool_stats`, `agent_file_locks`, etc.).

### Remote Bridge

Backend connectivity with protocol negotiation, heartbeats, and priority command queuing.

- `RemoteBridge` — manages connection lifecycle (Disconnected -> Connecting -> Connected -> Authenticated)
- Dual-mode: WebSocket (Supabase Realtime) with HTTP polling fallback
- `CommandQueue` — priority-based (Critical/High/Normal/Low) with deadline tracking and retry
- `HeartbeatCollector` — detects agent lifecycle changes

### Mesh Networking (feature: `mesh`)

Distributed agent mesh networking for multi-node coordination.

**Topology management:**

| Type | Description |
|------|-------------|
| `Star` | Central hub with spoke nodes |
| `Ring` | Circular node arrangement |
| `FullMesh` | Every node connected to every other |
| `Hierarchical` | Tree-structured with parent/child relationships |
| `Custom` | User-defined topology |

**Message routing strategies:**

| Strategy | Description |
|----------|-------------|
| `DirectRoute` | Point-to-point delivery |
| `ShortestPath` | Minimum-hop routing |
| `LoadBalanced` | Distribute across available routes |
| `Broadcast` | Send to all nodes |
| `Multicast` | Send to a subset of nodes |

**Peer discovery protocols:** mDNS, Gossip, Registry, Manual

**Federation:** Cross-mesh bridging with policies (Open, AllowList, DenyList, CapabilityBased)

```rust
use brainwires_agent_network::mesh::*;

let node = MeshNode::new("10.0.0.1:8080".parse().unwrap())
    .with_capabilities(NodeCapabilities {
        max_concurrent_tasks: 10,
        supported_protocols: vec!["mcp".into()],
        available_tools: vec!["bash".into(), "file_read".into()],
        compute_capacity: 0.8,
    });
```

### Error Handling

`AgentNetworkError` maps to standard JSON-RPC error codes:

| Variant | Code | Description |
|---------|------|-------------|
| `ParseError` | -32700 | JSON-RPC parse failure |
| `MethodNotFound` | -32601 | Unknown RPC method |
| `InvalidParams` | -32602 | Invalid method parameters |
| `Internal` | -32603 | Internal error (wraps `anyhow::Error`) |
| `Transport` | -32000 | Transport-level I/O error |
| `ToolNotFound` | -32001 | Requested tool not registered |
| `RateLimited` | -32002 | Rate limit exceeded |
| `Unauthorized` | -32003 | Authentication failure |

### Framework Traits

| Trait | Purpose |
|-------|---------|
| `SessionDir` | Path resolution for IPC sessions and data storage |
| `KeyStore` | Secure credential storage (keyring, file, etc.) |
| `AuthEndpoints` | Authentication endpoint configuration |
| `AgentSpawner` | Agent process spawning |
| `AgentDiscovery` | Agent listing and stale cleanup |
| `BridgeConfigProvider` | Remote bridge configuration |

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.2", features = ["agent-network", "mesh"] }
```

Or use standalone — `brainwires-agent-network` depends only on `brainwires-core` and `brainwires-mcp`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
