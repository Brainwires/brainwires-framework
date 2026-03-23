# brainwires-agent-network

[![Crates.io](https://img.shields.io/crates/v/brainwires-agent-network.svg)](https://crates.io/crates/brainwires-agent-network)
[![Documentation](https://img.shields.io/docsrs/brainwires-agent-network)](https://docs.rs/brainwires-agent-network)
[![License](https://img.shields.io/crates/l/brainwires-agent-network.svg)](LICENSE)

Agent networking layer for the Brainwires Agent Framework.

## Overview

`brainwires-agent-network` provides the full networking stack for AI agents: an MCP server framework with composable middleware, a 5-layer protocol stack for pluggable agent communication, encrypted IPC, a remote bridge for backend connectivity, agent lifecycle management, and optional distributed mesh networking.

**Design principles:**

- **Trait-driven** — `McpHandler`, `Transport`, `Router`, `Discovery`, and friends decouple the framework from any concrete implementation
- **Protocol-agnostic** — agents communicate over IPC, TCP, HTTP, WebSocket, A2A, or Pub/Sub through a uniform `Transport` trait
- **Middleware-composable** — auth, rate limiting, logging, and tool filtering stack via an onion model
- **Encryption-first** — IPC sockets use ChaCha20-Poly1305 authenticated encryption by default
- **Feature-gated** — only compile the transports and discovery mechanisms you need

```text
              ┌───────────────────────────────────────────────────────────┐
              │              brainwires-agent-network                     │
              │                                                           │
              │  ┌─────────────────────────────────────────────────────┐  │
              │  │         5-Layer Protocol Stack                      │  │
              │  │                                                     │  │
              │  │  Layer 5: Application (NetworkManager, Events)      │  │
              │  │  Layer 4: Discovery  (Manual, Registry)             │  │
              │  │  Layer 3: Routing    (Direct, Broadcast, Content)   │  │
              │  │  Layer 2: Transport  (IPC, Remote, TCP, A2A, PubSub)│  │
              │  │  Layer 1: Identity   (AgentIdentity, AgentCard)     │  │
              │  └─────────────────────────────────────────────────────┘  │
              │                                                           │
              │  ┌─────────────────────────────────────────────────────┐  │
              │  │             MCP Server Framework                    │  │
              │  │  McpHandler ──► MiddlewareChain ──► Transport       │  │
              │  │                   (onion model)                     │  │
              │  └─────────────────────────────────────────────────────┘  │
              │                                                           │
              │  ┌──────────────────┐  ┌───────────────────────────┐     │
              │  │  Agent Manager   │  │  Remote Bridge            │     │
              │  │  Spawn / List /  │  │  Supabase Realtime /      │     │
              │  │  Stop / Await    │  │  HTTP Polling Fallback    │     │
              │  └──────────────────┘  └───────────────────────────┘     │
              └───────────────────────────────────────────────────────────┘
```

## Quick Start

```toml
[dependencies]
brainwires-agent-network = "0.6"
```

Minimal MCP server:

```rust
use brainwires_agent_network::prelude::*;

struct MyHandler;

#[async_trait]
impl McpHandler for MyHandler {
    fn server_info(&self) -> ServerInfo {
        ServerInfo { name: "my-server".into(), version: "0.4.0".into() }
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
| `ipc-transport` | Yes | Unix-socket IPC transport with ChaCha20 encryption |
| `remote-transport` | Yes | Supabase Realtime / HTTP polling bridge transport |
| `tcp-transport` | No | Direct TCP peer-to-peer transport |
| `pubsub-transport` | No | In-process pub/sub transport with topic-based messaging |
| `a2a-transport` | No | A2A protocol transport (requires `brainwires-a2a`) |
| `mesh` | No | Distributed mesh networking (includes `tcp-transport`) |
| `registry-discovery` | No | HTTP-based agent registry discovery |
| `auth-keyring` | No | Secure API key storage via system keyring |
| `full` | No | All optional features enabled |

```toml
# With all transports and discovery
brainwires-agent-network = { version = "0.6", features = ["full"] }

# Just TCP and pub/sub
brainwires-agent-network = { version = "0.6", features = ["tcp-transport", "pubsub-transport"] }
```

## Architecture

### Protocol-Layer Stack

The networking layer is organized as a 5-layer protocol stack. Each layer has a well-defined trait, and concrete implementations are feature-gated.

#### Layer 1: Identity

Agent identity, capability advertisement, and cryptographic credentials.

**Key types:**

| Type | Description |
|------|-------------|
| `AgentIdentity` | UUID, name, and `AgentCard` |
| `AgentCard` | Capabilities, supported protocols, metadata, endpoint, compute capacity |
| `ProtocolId` | Protocol identifier string |
| `SigningKey` / `VerifyingKey` | ChaCha20-Poly1305 signing with SHA-256 key derivation |

```rust
use brainwires_agent_network::identity::{AgentIdentity, AgentCard};

let identity = AgentIdentity::new("my-agent")
    .with_capability("inference")
    .with_protocol("mcp")
    .with_endpoint("tcp://10.0.0.1:8080");
```

#### Layer 2: Transport

How bytes move between agents. Every transport implements the `Transport` trait.

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&mut self, target: &TransportAddress) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send(&self, envelope: &MessageEnvelope) -> Result<()>;
    async fn receive(&self) -> Result<Option<MessageEnvelope>>;
    fn transport_type(&self) -> TransportType;
    fn is_connected(&self) -> bool;
}
```

**Provided transports:**

| Transport | Feature flag | Wire format | Use case |
|-----------|-------------|-------------|----------|
| `IpcTransport` | `ipc-transport` | Length-prefixed, ChaCha20-encrypted JSON | Same-machine agents |
| `RemoteTransport` | `remote-transport` | HTTP POST with broadcast channel | Backend connectivity |
| `TcpTransport` | `tcp-transport` | Length-prefixed JSON over TCP (Nagle disabled) | Peer-to-peer mesh |
| `PubSubTransport` | `pubsub-transport` | In-process `tokio::broadcast` channels | Same-process topic messaging |
| `A2aTransport` | `a2a-transport` | A2A JSON-RPC over HTTP/WebSocket | Cross-framework interop |

**Addressing:**

```rust
pub enum TransportAddress {
    Unix(PathBuf),     // unix:///tmp/agent.sock
    Tcp(SocketAddr),   // tcp://127.0.0.1:9090
    Url(String),       // https://example.com/a2a
    Channel(String),   // channel://status-updates
}
```

#### Layer 3: Routing

Where messages go. Routers decide which transport addresses to deliver to.

```rust
#[async_trait]
pub trait Router: Send + Sync {
    async fn route(
        &self,
        envelope: &MessageEnvelope,
        peers: &PeerTable,
    ) -> Result<Vec<TransportAddress>>;
    fn strategy(&self) -> RoutingStrategy;
}
```

**Provided routers:**

| Router | Strategy | Description |
|--------|----------|-------------|
| `DirectRouter` | `Direct` | Point-to-point delivery to a single peer |
| `BroadcastRouter` | `Broadcast` | Deliver to all known peers (except sender) |
| `ContentRouter` | `ContentBased` | Route to peers subscribed to matching topics |

**`PeerTable`** tracks known peers and their transport addresses, with optional topic subscriptions for content-based routing.

#### Layer 4: Discovery

How agents find each other on the network.

```rust
#[async_trait]
pub trait Discovery: Send + Sync {
    async fn register(&self, identity: &AgentIdentity) -> Result<()>;
    async fn deregister(&self, id: &Uuid) -> Result<()>;
    async fn discover(&self) -> Result<Vec<AgentIdentity>>;
    async fn lookup(&self, id: &Uuid) -> Result<Option<AgentIdentity>>;
    fn protocol(&self) -> DiscoveryProtocol;
}
```

**Provided implementations:**

| Implementation | Feature flag | Description |
|----------------|-------------|-------------|
| `ManualDiscovery` | Always | In-memory peer list, configured programmatically |
| `RegistryDiscovery` | `registry-discovery` | HTTP REST-based agent registry |

#### Layer 5: Application (NetworkManager)

The user-facing API that ties all layers together.

```rust
use brainwires_agent_network::{
    NetworkManagerBuilder, AgentIdentity, Payload, NetworkEvent,
};
use brainwires_agent_network::transport::TcpTransport;
use brainwires_agent_network::routing::DirectRouter;
use brainwires_agent_network::discovery::ManualDiscovery;

let manager = NetworkManagerBuilder::new(identity)
    .add_transport(Box::new(TcpTransport::new()))
    .with_router(Box::new(DirectRouter))
    .add_discovery(Box::new(ManualDiscovery::new()))
    .build()
    .await?;

// Send a message
manager.send(peer_id, Payload::Text("hello".into())).await?;

// Broadcast to all peers
manager.broadcast(Payload::Json(json!({"status": "ready"}))).await?;

// Subscribe to network events
let mut events = manager.subscribe();
while let Ok(event) = events.recv().await {
    match event {
        NetworkEvent::PeerJoined(peer) => println!("New peer: {}", peer.name),
        NetworkEvent::MessageReceived(env) => println!("Got: {:?}", env.payload),
        _ => {}
    }
}
```

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

**Connection lifecycle:**

1. `IpcConnection::connect(socket_path)` -- plain-text connection
2. `Handshake` exchange -- session ID, token, model, working directory
3. `connection.upgrade_to_encrypted(session_token)` -- ChaCha20-Poly1305 from this point

**Encryption (`IpcCipher`):** Derives a 256-bit key from the session token via SHA-256 with domain separator `brainwires-ipc-v1:`. All post-handshake messages use ChaCha20-Poly1305 authenticated encryption. Wire format: `[nonce 12B][ciphertext + auth tag 16B]`.

### Authentication

Session-based authentication with optional keyring storage.

**`AuthClient`** — HTTP client for authenticating against the Brainwires Studio backend.

**`SessionManager`** — Persists sessions to disk as JSON with `0600` permissions. API keys are stored separately via the `KeyStore` trait (system keyring preferred).

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

### Remote Bridge

Backend connectivity with protocol negotiation, heartbeats, and priority command queuing.

Dual-mode transport: Supabase Realtime WebSocket (preferred) with HTTP polling fallback for restricted environments.

### Mesh Networking (feature: `mesh`)

Distributed agent mesh networking for multi-node coordination. Includes topology management (star, ring, full mesh, hierarchical), message routing strategies, peer discovery protocols, and federation gateways for cross-mesh bridging.

> **Note:** The mesh module provides trait definitions and types. The protocol-layer stack (transport, routing, discovery) provides the concrete implementations that power mesh networking.

### Message Types

**`MessageEnvelope`** — the universal message container:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique message ID |
| `sender` | `Uuid` | Sender agent ID |
| `recipient` | `MessageTarget` | Direct(Uuid), Broadcast, or Topic(String) |
| `payload` | `Payload` | Json(Value), Binary(Bytes), or Text(String) |
| `timestamp` | `DateTime<Utc>` | When the message was created |
| `ttl` | `Option<u32>` | Remaining hops before discard |
| `correlation_id` | `Option<Uuid>` | Links replies to requests |
| `transport_type` | `TransportType` | Which transport originated this message |

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

### TCP Transport Peer-to-Peer

```rust
use brainwires_agent_network::transport::{TcpTransport, TransportAddress, Transport};
use brainwires_agent_network::network::{MessageEnvelope, Payload};

let mut client = TcpTransport::new();
client.connect(&TransportAddress::Tcp("127.0.0.1:9090".parse()?)).await?;

let envelope = MessageEnvelope::direct(sender_id, peer_id, Payload::Text("ping".into()));
client.send(&envelope).await?;

let reply = client.receive().await?;
client.disconnect().await?;
```

### Pub/Sub Topic Messaging

```rust
use brainwires_agent_network::transport::{PubSubTransport, TransportAddress, Transport};
use brainwires_agent_network::network::{MessageEnvelope, Payload};

let mut transport = PubSubTransport::new();
transport.connect(&TransportAddress::Channel("events".into())).await?;

// Subscribe to a topic
let mut rx = transport.subscribe_topic("status-updates").await;

// Send a topic message
let envelope = MessageEnvelope::topic(sender_id, "status-updates", Payload::Text("ready".into()));
transport.send(&envelope).await?;
```

### A2A Transport (Cross-Framework)

```rust
use brainwires_agent_network::transport::{A2aTransport, TransportAddress, Transport};

let mut transport = A2aTransport::from_url("https://other-agent.example.com/a2a")?;
transport.connect(&TransportAddress::Url("https://other-agent.example.com/a2a".into())).await?;

transport.send(&envelope).await?;
```

### Encrypted IPC Connection

```rust
use brainwires_agent_network::ipc::{IpcConnection, ViewerMessage, AgentMessage};

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
    list_agent_sessions_with_metadata, cleanup_stale_sockets, format_agent_tree,
};

cleanup_stale_sockets(sessions_dir).await?;
let agents = list_agent_sessions_with_metadata(sessions_dir)?;
let tree = format_agent_tree(sessions_dir, Some("current-session-id"))?;
println!("{}", tree);
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.6", features = ["agent-network"] }
```

Or use standalone — `brainwires-agent-network` depends only on `brainwires-core` and `brainwires-mcp`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
