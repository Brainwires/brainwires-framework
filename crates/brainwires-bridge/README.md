# brainwires-bridge

[![Crates.io](https://img.shields.io/crates/v/brainwires-bridge.svg)](https://crates.io/crates/brainwires-bridge)
[![Documentation](https://img.shields.io/docsrs/brainwires-bridge)](https://docs.rs/brainwires-bridge)
[![License](https://img.shields.io/crates/l/brainwires-bridge.svg)](LICENSE)

MCP server framework and agent communication backbone for Brainwires.

## Overview

`brainwires-bridge` provides three layers of functionality: an MCP server framework with composable middleware, an encrypted IPC system for local inter-agent communication, and a remote bridge for backend connectivity via Supabase Realtime or HTTP polling.

**Design principles:**

- **Trait-driven** — `McpHandler`, `KeyStore`, `AgentSpawner`, and friends decouple the framework from any concrete CLI implementation
- **Middleware-composable** — auth, rate limiting, logging, and tool filtering stack via an onion model
- **Encryption-first** — IPC sockets use ChaCha20-Poly1305 authenticated encryption by default
- **Dual-mode remote** — Supabase Realtime (preferred) with automatic HTTP polling fallback

```text
                ┌──────────────────────────────────────────────────────┐
                │                  brainwires-bridge                    │
                │                                                      │
                │  ┌────────────────────────────────────────────────┐  │
                │  │            MCP Server Framework                │  │
                │  │                                                │  │
                │  │  McpHandler ──► MiddlewareChain ──► Transport  │  │
                │  │                   (onion model)                │  │
                │  │                                                │  │
                │  │  ┌──────────┐  ┌────────────┐                 │  │
                │  │  │ToolReg-  │  │  Auth │Rate │                │  │
                │  │  │ istry    │  │ Limit│Log  │                 │  │
                │  │  └──────────┘  └────────────┘                 │  │
                │  └────────────────────────────────────────────────┘  │
                │                                                      │
                │  ┌─────────────────┐  ┌──────────────────────────┐  │
                │  │  IPC Layer      │  │  Remote Bridge           │  │
                │  │                 │  │                          │  │
                │  │  Unix Socket    │  │  Supabase Realtime /     │  │
                │  │  + ChaCha20     │  │  HTTP Polling Fallback   │  │
                │  │  + Discovery    │  │  + Priority Queue        │  │
                │  └─────────────────┘  └──────────────────────────┘  │
                │                                                      │
                │  ┌─────────────────┐  ┌──────────────────────────┐  │
                │  │  Auth System    │  │  Agent Manager           │  │
                │  │  Session +      │  │  Spawn / List / Stop /   │  │
                │  │  Keyring        │  │  Await / Pool Stats      │  │
                │  └─────────────────┘  └──────────────────────────┘  │
                └──────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-bridge = "0.1"
```

Minimal MCP server:

```rust
use brainwires_bridge::prelude::*;

struct MyHandler;

#[async_trait]
impl McpHandler for MyHandler {
    fn server_info(&self) -> ServerInfo {
        ServerInfo { name: "my-server".into(), version: "0.1.0".into() }
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
| `auth-keyring` | No | Secure API key storage via system keyring (`keyring` crate) |

IPC encryption, MCP transport, middleware, and remote bridge are always available.

```toml
# With keyring support
brainwires-bridge = { version = "0.1", features = ["auth-keyring"] }
```

## Architecture

### MCP Server Framework

The framework follows the Model Context Protocol specification for tool-based AI server integration.

**Traits:**

- `McpHandler` — defines server identity, capabilities, and tool dispatch
- `ServerTransport` — reads requests and writes responses (stdio, etc.)
- `ToolHandler` — per-tool call handler

**`McpServer<H: McpHandler>`** wires handler, middleware chain, and transport together:

```rust
let server = McpServer::new(handler)
    .with_transport(StdioServerTransport::new())
    .with_middleware(AuthMiddleware::new("token"))
    .with_middleware(RateLimitMiddleware::new(10.0));

server.run().await?;
```

**`McpToolRegistry`** provides declarative tool registration with automatic dispatch:

```rust
let mut registry = McpToolRegistry::new();
registry.register("my_tool", "Description", schema, MyToolHandler);

let tools = registry.list_tool_defs();
let result = registry.dispatch("my_tool", args, &ctx).await?;
```

### Middleware

Middleware follows an **onion model**: requests flow forward through layers, responses flow back.

**Trait:**

```rust
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    async fn process_request(
        &self,
        request: &JsonRpcRequest,
        ctx: &mut RequestContext,
    ) -> MiddlewareResult;

    async fn process_response(
        &self,
        response: &mut JsonRpcResponse,
        ctx: &RequestContext,
    ) {}
}
```

`MiddlewareResult::Continue` passes to the next layer; `MiddlewareResult::Reject(err)` short-circuits the chain.

**Built-in middleware:**

| Layer | Description |
|-------|-------------|
| `AuthMiddleware` | Bearer token validation, rejects unauthorized requests |
| `RateLimitMiddleware` | Token-bucket rate limiter with per-tool limits |
| `LoggingMiddleware` | Structured request/response logging via `tracing` |
| `ToolFilterMiddleware` | Allow-list or deny-list for tool access |

**`ToolFilterMiddleware` modes:**

| Mode | Constructor | Description |
|------|------------|-------------|
| `AllowList` | `ToolFilterMiddleware::allow_only(["tool_a", "tool_b"])` | Only listed tools are accessible |
| `DenyList` | `ToolFilterMiddleware::deny(["dangerous_tool"])` | Listed tools are blocked |

### IPC (Inter-Process Communication)

Local agent-to-agent communication over Unix domain sockets with authenticated encryption.

**Connection lifecycle:**

1. `IpcConnection::connect(socket_path)` — plain-text connection
2. `Handshake` exchange — session ID, token, model, working directory
3. `connection.upgrade_to_encrypted(session_token)` — ChaCha20-Poly1305 from this point

**`IpcCipher`** derives a 256-bit key from the session token via SHA-256:

```rust
let cipher = IpcCipher::from_session_token("session-token-here");
let encrypted = cipher.encrypt(b"plaintext")?;
let decrypted = cipher.decrypt(&encrypted)?;
```

**Reader/Writer pairs:**

| Variant | Description |
|---------|-------------|
| `IpcReader` / `IpcWriter` | Plain-text newline-delimited JSON |
| `EncryptedIpcReader` / `EncryptedIpcWriter` | Encrypted + base64-encoded JSON |
| `IpcConnection` | Combined reader + writer with `split()` and `upgrade_to_encrypted()` |

**Protocol messages — `ViewerMessage` (inbound from viewer):**

| Group | Variants |
|-------|----------|
| Chat | `UserInput`, `Cancel`, `SyncRequest`, `Exit`, `Detach`, `Disconnect` |
| Commands | `SlashCommand`, `SetToolMode`, `QueueMessage` |
| Locks | `AcquireLock`, `ReleaseLock`, `QueryLocks`, `UpdateLockStatus` |
| Agents | `ListAgents`, `SpawnAgent`, `NotifyChildren`, `ParentSignal` |
| Plan mode | `EnterPlanMode`, `ExitPlanMode`, `PlanModeUserInput`, `PlanModeSyncRequest` |

**Protocol messages — `AgentMessage` (outbound from agent):**

| Group | Variants |
|-------|----------|
| Streaming | `StreamChunk`, `StreamEnd` |
| Tools | `ToolCallStart`, `ToolProgress`, `ToolResult` |

**Agent discovery:**

| Function | Description |
|----------|-------------|
| `list_agent_sessions(dir)` | List all agent session IDs |
| `list_agent_sessions_with_metadata(dir)` | List with full `AgentMetadata` |
| `is_agent_alive(dir, id)` | Check if socket responds |
| `cleanup_stale_sockets(dir)` | Remove dead agent sockets |
| `format_agent_tree(dir, root_only)` | ASCII tree of parent/child agents |
| `get_child_agents(dir, parent_id)` | List children of a parent agent |

### Authentication

Session-based authentication with optional keyring storage.

**`AuthClient`** authenticates against a backend:

```rust
let client = AuthClient::new(
    "https://api.example.com".into(),
    "/auth/cli".into(),
    r"^bw-[a-zA-Z0-9]{32,}$",
);

client.validate_api_key_format("bw-abc...")?;
let response = client.authenticate("bw-abc...").await?;
```

**`SessionManager`** persists sessions to disk with keyring integration:

```rust
let manager = SessionManager::new(session_file, Some(Box::new(my_keyring)));

manager.save(&session, Some("api-key"))?;
let session = manager.load()?;
let api_key = manager.get_api_key()?;
```

**Key types:**

| Type | Description |
|------|-------------|
| `AuthSession` | User profile, Supabase config, backend URL, timestamp |
| `UserProfile` | User ID, username, display name, role |
| `SupabaseConfig` | Supabase URL and anonymous key |

### Agent Manager

Trait-based agent lifecycle management for MCP server mode.

**`AgentManager` trait:**

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

**`AgentToolRegistry`** registers 10 MCP tools for agent management:

| Tool | Description |
|------|-------------|
| `agent_spawn` | Spawn a new task agent with optional MDAP |
| `agent_list` | List running agents |
| `agent_status` | Get agent status by ID |
| `agent_stop` | Stop an agent by ID |
| `agent_await` | Wait for agent completion |
| `agent_pool_stats` | Pool statistics |
| `agent_file_locks` | List file locks |
| `self_improve_start` | Start autonomous self-improvement session |
| `self_improve_status` | Get improvement session status |
| `self_improve_stop` | Stop improvement session |

### Remote Bridge

Backend connectivity with protocol negotiation, heartbeats, and priority command queuing.

**`RemoteBridge`** manages the connection lifecycle:

```rust
let bridge = RemoteBridge::new(config, Some(spawner));

// State: Disconnected → Connecting → Connected → Authenticated
let state = bridge.state().await;
let mode = bridge.connection_mode().await; // Realtime or Polling
```

**Protocol negotiation:**

```rust
// Client sends ProtocolHello with supported versions and capabilities
// Server responds with ProtocolAccept selecting version and capabilities
pub enum ProtocolCapability {
    Streaming, Tools, Presence, Compression,
    Attachments, Priority, Telemetry,
}
```

**`CommandQueue`** — priority-based queue with deadline tracking and retry:

| Feature | Description |
|---------|-------------|
| Priority levels | `Critical`, `High`, `Normal`, `Low` |
| Deadline tracking | Commands expire if not processed in time |
| Exponential backoff | Configurable retry with multiplier |
| Max depth | Bounded queue prevents memory exhaustion |

**`HeartbeatCollector`** detects agent lifecycle changes:

```rust
let mut collector = HeartbeatCollector::new(sessions_dir, version);
let data: HeartbeatData = collector.collect().await?;
let events: Vec<AgentEvent> = collector.detect_changes()?;
```

### Framework Traits

Trait abstractions decouple the bridge from CLI-specific implementations:

| Trait | Purpose |
|-------|---------|
| `SessionDir` | Path resolution for IPC sessions and data storage |
| `KeyStore` | Secure credential storage (keyring, file, etc.) |
| `AuthEndpoints` | Authentication endpoint configuration |
| `AgentSpawner` | Agent process spawning |
| `AgentDiscovery` | Agent listing and stale cleanup |
| `BridgeConfigProvider` | Remote bridge configuration |

### Error Handling

**`BridgeError` variants:**

| Variant | Description |
|---------|-------------|
| `ParseError` | JSON-RPC parse failure |
| `MethodNotFound` | Unknown RPC method |
| `InvalidParams` | Invalid method parameters |
| `Internal` | Internal error (wraps `anyhow::Error`) |
| `Transport` | Transport-level I/O error |
| `ToolNotFound` | Requested tool not registered |
| `RateLimited` | Rate limit exceeded |
| `Unauthorized` | Authentication failure |

All variants map to standard JSON-RPC error codes via `to_json_rpc_error()`.

## Usage Examples

### MCP Server with Auth and Rate Limiting

```rust
use brainwires_bridge::prelude::*;

let server = McpServer::new(my_handler)
    .with_transport(StdioServerTransport::new())
    .with_middleware(AuthMiddleware::new("secret-token"))
    .with_middleware(RateLimitMiddleware::new(10.0).with_tool_limit("expensive_tool", 2.0))
    .with_middleware(LoggingMiddleware::new());

server.run().await?;
```

### Tool Registry with Dispatch

```rust
use brainwires_bridge::{McpToolRegistry, McpToolDef, ToolHandler, RequestContext};

let mut registry = McpToolRegistry::new();
registry.register(
    "greet",
    "Say hello to someone",
    serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    }),
    GreetHandler,
);

assert!(registry.has_tool("greet"));
let result = registry.dispatch("greet", args, &ctx).await?;
```

### Encrypted IPC Connection

```rust
use brainwires_bridge::ipc::{IpcConnection, Handshake};

let mut conn = IpcConnection::connect(&socket_path).await?;

// Exchange handshake in plaintext
conn.writer.write(&handshake).await?;
let response = conn.reader.read::<HandshakeResponse>().await?;

// Upgrade to encrypted channel
let encrypted = conn.upgrade_to_encrypted(&session_token);
let (reader, writer) = (encrypted.reader, encrypted.writer);
```

### Agent Discovery

```rust
use brainwires_bridge::ipc::discovery::*;

let sessions = list_agent_sessions_with_metadata(&sessions_dir)?;
let tree = format_agent_tree(&sessions_dir, true)?;
println!("{}", tree);

// Cleanup dead agents
cleanup_stale_sockets(&sessions_dir).await?;
```

### Session Management

```rust
use brainwires_bridge::auth::{SessionManager, AuthClient};

let client = AuthClient::new(backend_url, auth_endpoint, api_key_pattern);
let response = client.authenticate("bw-my-api-key").await?;

let session = SessionManager::create_session(response, backend_url, api_key);
let manager = SessionManager::new(session_file, key_store);
manager.save(&session, Some("bw-my-api-key"))?;
```

### Tool Filtering

```rust
use brainwires_bridge::ToolFilterMiddleware;

// Only allow specific tools
let filter = ToolFilterMiddleware::allow_only(["safe_tool_a", "safe_tool_b"]);

// Or block dangerous ones
let filter = ToolFilterMiddleware::deny(["rm_rf", "drop_database"]);
```

### Remote Bridge State

```rust
use brainwires_bridge::remote::{RemoteBridge, BridgeConfig, BridgeState, ConnectionMode};

let bridge = RemoteBridge::new(config, Some(spawner));

match bridge.state().await {
    BridgeState::Authenticated => {
        let mode = bridge.connection_mode().await;
        match mode {
            ConnectionMode::Realtime => println!("WebSocket connected"),
            ConnectionMode::Polling => println!("HTTP polling fallback"),
        }
    }
    _ => println!("Not ready"),
}
```

## Configuration

### BridgeConfig (Remote)

```rust
pub struct BridgeConfig {
    pub backend_url: String,
    pub api_key: String,
    pub heartbeat_interval_secs: u32,   // Default: 30
    pub reconnect_delay_secs: u32,      // Default: 5
    pub max_reconnect_attempts: u32,    // Default: 10
    pub version: String,
    pub sessions_dir: PathBuf,
    pub attachment_dir: PathBuf,
}
```

### RequestContext

```rust
let mut ctx = RequestContext::new(request_id)
    .with_client_info(ClientInfo { name, version });

ctx.set_metadata("key".into(), value);
ctx.set_initialized();
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["bridge"] }
```

Or use standalone — `brainwires-bridge` depends only on `brainwires-core` and `brainwires-mcp`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
