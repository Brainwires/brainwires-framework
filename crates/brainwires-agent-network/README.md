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

**Middleware trait:**

```rust
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    async fn process_request(
        &self, request: &JsonRpcRequest, ctx: &mut RequestContext,
    ) -> MiddlewareResult;

    async fn process_response(
        &self, _response: &mut JsonRpcResponse, _ctx: &RequestContext,
    ) {}
}
```

`MiddlewareResult` is either `Continue` (pass to next layer) or `Reject(JsonRpcError)` (short-circuit with error).

**ToolFilterMiddleware** supports two modes:

- `ToolFilterMiddleware::allow_only(["agent_spawn", "agent_list"])` -- only listed tools are permitted
- `ToolFilterMiddleware::deny(["bash", "write_file"])` -- listed tools are blocked, everything else allowed

Filtering applies only to `tools/call` requests; other JSON-RPC methods pass through unconditionally.

### IPC (Inter-Process Communication)

Local agent-to-agent communication over Unix domain sockets with authenticated encryption.

**Connection lifecycle:**

1. `IpcConnection::connect(socket_path)` -- plain-text connection
2. `Handshake` exchange -- session ID, token, model, working directory
3. `connection.upgrade_to_encrypted(session_token)` -- ChaCha20-Poly1305 from this point

**Encryption (`IpcCipher`):** Derives a 256-bit key from the session token via SHA-256 with domain separator `brainwires-ipc-v1:`. All post-handshake messages use ChaCha20-Poly1305 authenticated encryption. Wire format: `[nonce 12B][ciphertext + auth tag 16B]`. Encrypted messages are length-prefixed (`[4B big-endian length][encrypted blob]`).

**Reader/Writer pairs:**

| Type | Description |
|------|-------------|
| `IpcReader` / `IpcWriter` | Plain-text, newline-delimited JSON (used during handshake) |
| `EncryptedIpcReader` / `EncryptedIpcWriter` | ChaCha20-Poly1305 encrypted, length-prefixed |
| `IpcConnection` | Combines reader + writer; provides `split()` and `upgrade_to_encrypted()` |
| `EncryptedIpcConnection` | Encrypted pair; also provides `split()` |

**Protocol messages -- `ViewerMessage` (viewer/TUI to agent):**

| Category | Variants |
|----------|----------|
| Chat | `UserInput`, `Cancel`, `SyncRequest`, `Exit`, `Detach`, `Disconnect` |
| Commands | `SlashCommand`, `SetToolMode`, `QueueMessage` |
| Locks | `AcquireLock`, `ReleaseLock`, `QueryLocks`, `UpdateLockStatus` |
| Agents | `ListAgents`, `SpawnAgent`, `NotifyChildren`, `ParentSignal` |
| Plan mode | `EnterPlanMode`, `ExitPlanMode`, `PlanModeUserInput`, `PlanModeSyncRequest` |

**Protocol messages -- `AgentMessage` (agent to viewer/TUI):**

| Category | Variants |
|----------|----------|
| Streaming | `StreamChunk`, `StreamEnd` |
| Tools | `ToolCallStart`, `ToolProgress`, `ToolResult` |
| State | `ConversationSync`, `MessageAdded`, `StatusUpdate`, `TaskUpdate` |
| Lifecycle | `Error`, `Exiting`, `Ack`, `Toast` |
| Locks | `LockResult`, `LockReleased`, `LockStatus`, `LockChanged` |
| Multi-agent | `AgentSpawned`, `AgentList`, `AgentExiting`, `ParentSignalReceived` |
| Plan mode | `PlanModeEntered`, `PlanModeExited`, `PlanModeSync`, `PlanModeStreamChunk`, `PlanModeStreamEnd` |

**Agent discovery functions:**

| Function | Description |
|----------|-------------|
| `list_agent_sessions()` | List all `.sock` session IDs in sessions directory |
| `list_agent_sessions_with_metadata()` | List sessions with parsed `AgentMetadata` from `.meta.json` files |
| `is_agent_alive()` | Socket connect test with 2-second timeout |
| `cleanup_stale_sockets()` | Remove `.sock`, `.token`, `.meta.json`, `.log` files for dead sessions |
| `format_agent_tree()` | Render parent/child agent hierarchy as a formatted tree string |
| `get_child_agents()` | Get all agents whose `parent_agent_id` matches a given session |

### Authentication

Session-based authentication with optional keyring storage.

**`AuthClient`** -- HTTP client for authenticating against the Brainwires Studio backend. Validates API key format with a configurable regex pattern (default: `bw_(prod|dev|test)_[a-z0-9]{32}`), then exchanges the key for an `AuthResponse`.

**`SessionManager`** -- Persists sessions to disk as JSON with `0600` permissions. API keys are stored separately via the `KeyStore` trait (system keyring preferred) rather than in the session file. Supports legacy migration from in-file keys to keyring storage.

**Key types:**

| Type | Description |
|------|-------------|
| `AuthSession` | User profile, Supabase config, key name, backend URL, timestamp |
| `UserProfile` | `user_id`, `username`, `display_name`, `role` |
| `SupabaseConfig` | Supabase project `url` and `anon_key` |
| `AuthResponse` | Backend response: user profile + Supabase config + key name |

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

**Connection lifecycle states:** `Disconnected` -> `Connecting` -> `Connected` -> `Authenticated` -> `ShuttingDown`

Dual-mode transport: Supabase Realtime WebSocket (preferred) with HTTP polling fallback for restricted environments.

**Protocol negotiation:**

1. CLI sends `ProtocolHello` with `supported_versions` and `capabilities`
2. Backend responds with `ProtocolAccept` selecting a version and enabled capabilities

`ProtocolCapability` variants: `Streaming`, `Tools`, `Presence`, `Compression`, `Attachments`, `Priority`, `Telemetry`.

**`CommandQueue`** -- Priority queue backed by `BinaryHeap` with FIFO ordering within the same level.

| Priority | Value | Description |
|----------|-------|-------------|
| `Critical` | 0 | Emergency stop, security -- bypasses queue depth limit |
| `High` | 1 | User-initiated actions |
| `Normal` | 2 | Default |
| `Low` | 3 | Background tasks |

Each entry tracks a deadline (`Option<Instant>`), retry attempt count, and an optional `RetryPolicy` with configurable `max_attempts`, exponential `backoff_multiplier`, and `initial_delay_ms`. Default max depth: 1000 commands.

**`HeartbeatCollector`** -- Periodically calls `collect()` to return `HeartbeatData` (agent list, system load, hostname, OS, version). `detect_changes()` diffs against the previous collection and emits `Vec<AgentEvent>` with types: `Spawned`, `Exited`, `Busy`, `Idle`, `StateChanged`.

### Mesh Networking (feature: `mesh`)

Distributed agent mesh networking for multi-node coordination.

**`MeshNode`** -- A node in the mesh identified by UUID, with an address, lifecycle state, capabilities, and arbitrary metadata.

Node states:

| State | Description |
|-------|-------------|
| `Initializing` | Starting up, not yet ready |
| `Active` | Accepting work |
| `Draining` | Finishing in-flight tasks before shutdown |
| `Disconnected` | Lost connectivity |
| `Failed` | Unrecoverable failure |

**`NodeCapabilities`:**

| Field | Type | Description |
|-------|------|-------------|
| `max_concurrent_tasks` | `usize` | Concurrency limit |
| `supported_protocols` | `Vec<String>` | e.g. `["a2a", "mcp"]` |
| `available_tools` | `Vec<String>` | Tool names on this node |
| `compute_capacity` | `f64` | Abstract power score (higher = more powerful) |

**Topology management (`MeshTopology` trait):**

| Type | Description |
|------|-------------|
| `Star` | Central coordinator with spoke nodes |
| `Ring` | Circular ring, each node connects to the next |
| `FullMesh` | Every node connected to every other |
| `Hierarchical` | Tree-like parent/child relationships |
| `Custom(String)` | User-defined topology with explicit adjacency |

**Message routing (`MessageRouter` trait):**

| Strategy | Description |
|----------|-------------|
| `DirectRoute` | Point-to-point delivery |
| `ShortestPath` | Minimum-hop routing |
| `LoadBalanced` | Distribute across available routes |
| `Broadcast` | Send to all nodes |
| `Multicast(Vec<Uuid>)` | Send to a specific subset of nodes |

`RouteEntry` describes a single routing-table row: `destination`, `next_hop`, `cost` (metric), and `ttl` (max hops remaining).

**Peer discovery (`PeerDiscovery` trait):**

| Protocol | Description |
|----------|-------------|
| `Mdns` | Multicast DNS for zero-config local-network discovery |
| `Gossip` | Decentralized gossip-based peer exchange |
| `Registry` | Centralized registry service lookup |
| `Manual` | Statically configured peer list |

**Federation (`FederationGateway` trait):**

Cross-mesh bridging controlled by `FederationPolicy`:

| Policy | Description |
|--------|-------------|
| `Open` | Any peer may join |
| `AllowList(Vec<Uuid>)` | Only listed node IDs accepted |
| `DenyList(Vec<Uuid>)` | All except listed IDs accepted |
| `CapabilityBased(Vec<String>)` | Peers must advertise required capabilities (e.g. `"inference"`) |

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

## Usage Examples

### MCP Server with Auth and Rate Limiting

```rust
use brainwires_agent_network::prelude::*;
use brainwires_agent_network::middleware::{
    AuthMiddleware, RateLimitMiddleware, ToolFilterMiddleware,
};

let server = McpServer::new(MyHandler)
    .with_transport(StdioServerTransport::new())
    .with_middleware(AuthMiddleware::bearer("my-secret-token"))
    .with_middleware(RateLimitMiddleware::new(100)) // 100 req/min
    .with_middleware(ToolFilterMiddleware::deny(["bash"]));

server.run().await?;
```

### Encrypted IPC Connection

```rust
use brainwires_agent_network::ipc::{IpcConnection, ViewerMessage, AgentMessage};

// Client connects and upgrades to encrypted channel
let conn = IpcConnection::connect(socket_path).await?;
let (mut reader, mut writer) = conn.upgrade_to_encrypted(session_token).split();

writer.write(&ViewerMessage::UserInput {
    content: "Hello".into(),
    context_files: vec![],
}).await?;

let response: Option<AgentMessage> = reader.read().await?;
```

### Agent Discovery

```rust
use brainwires_agent_network::ipc::{
    list_agent_sessions_with_metadata, is_agent_alive,
    cleanup_stale_sockets, format_agent_tree, get_child_agents,
};

// Clean up dead sessions, then list what remains
cleanup_stale_sockets(sessions_dir).await?;
let agents = list_agent_sessions_with_metadata(sessions_dir)?;

// Print the agent tree
let tree = format_agent_tree(sessions_dir, Some("current-session-id"))?;
println!("{}", tree);

// Get children of a specific agent
let children = get_child_agents(sessions_dir, "parent-session-id")?;
```

### Session Management

```rust
use brainwires_agent_network::auth::{AuthClient, SessionManager, AuthSession};

let client = AuthClient::new(
    "https://brainwires.studio".into(),
    "/api/cli/auth".into(),
    r"^bw_(prod|dev|test)_[a-z0-9]{32}$",
);

// Validate and authenticate
client.validate_api_key_format(api_key)?;
let response = client.authenticate(api_key).await?;

// Persist session with optional keyring
let manager = SessionManager::new(session_path, Some(keyring));
let session = SessionManager::create_session(response, backend_url, api_key);
manager.save(&session, Some(api_key))?;
```

### Tool Filtering

```rust
use brainwires_agent_network::middleware::ToolFilterMiddleware;

// Only allow agent management tools
let allow = ToolFilterMiddleware::allow_only([
    "agent_spawn", "agent_list", "agent_status", "agent_stop",
]);

// Block dangerous tools
let deny = ToolFilterMiddleware::deny(["bash", "write_file", "delete_file"]);
```

### Remote Bridge State

```rust
use brainwires_agent_network::remote::{
    BridgeConfig, CommandQueue, HeartbeatCollector,
};
use brainwires_agent_network::remote::protocol::{
    CommandPriority, PrioritizedCommand, RetryPolicy,
};

// Priority command queue
let mut queue = CommandQueue::default(); // max depth 1000
queue.enqueue(PrioritizedCommand {
    command: my_command,
    priority: CommandPriority::High,
    deadline_ms: Some(5000),
    retry_policy: Some(RetryPolicy {
        max_attempts: 3,
        backoff_multiplier: 2.0,
        initial_delay_ms: 100,
    }),
})?;

// Heartbeat collection
let mut collector = HeartbeatCollector::new(sessions_dir, "0.2.0".into());
let data = collector.collect().await?;
let events = collector.detect_changes()?;
```

### Mesh: Star Topology with Content-Based Routing

```rust
use brainwires_agent_network::mesh::*;
use uuid::Uuid;

// Define a node
let node = MeshNode {
    id: Uuid::new_v4(),
    address: "10.0.0.1:8080".into(),
    state: NodeState::Active,
    capabilities: NodeCapabilities {
        max_concurrent_tasks: 10,
        supported_protocols: vec!["mcp".into()],
        available_tools: vec!["bash".into(), "file_read".into()],
        compute_capacity: 0.8,
    },
    last_seen: chrono::Utc::now().to_rfc3339(),
    metadata: HashMap::new(),
};

// Route entries for content-based routing
let route = RouteEntry {
    destination: target_node_id,
    next_hop: hub_node_id,
    cost: 1.0,
    ttl: 10,
};
```

### Mesh: Federation Between Meshes

```rust
use brainwires_agent_network::mesh::FederationPolicy;

// Only accept peers that support inference
let policy = FederationPolicy::CapabilityBased(
    vec!["inference".into(), "embedding".into()],
);

// Or explicitly allow specific nodes
let policy = FederationPolicy::AllowList(vec![trusted_node_id]);
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.2", features = ["agent-network", "mesh"] }
```

Or use standalone — `brainwires-agent-network` depends only on `brainwires-core` and `brainwires-mcp`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
