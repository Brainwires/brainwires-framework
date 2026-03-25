# brainwires-gateway

Always-on WebSocket gateway that routes messages between messaging channel adapters and AI agent sessions.

## Quick Start

```bash
cargo run -p brainwires-gateway -- serve
```

Or with options:

```bash
cargo run -p brainwires-gateway -- serve --host 0.0.0.0 --port 18789
```

## Architecture

The gateway is the hub of a personal AI assistant. Channel adapters (Discord, Telegram, etc.) connect via WebSocket, and the gateway routes messages to per-user agent sessions.

```text
Discord Adapter ──┐
                  │  WebSocket
Telegram Adapter ─┼──────────► Gateway ──► AgentInboundHandler
                  │              │             │
Slack Adapter ────┘              │             ├─► ChatAgent (user A)
                                 │             ├─► ChatAgent (user B)
                          Admin API            └─► ChatAgent (user C)
                          Webhooks
```

## CLI

| Command | Description |
|---------|-------------|
| `serve` | Start the gateway server (default) |
| `serve --host <addr>` | Bind address (default: 127.0.0.1) |
| `serve --port <port>` | Listen port (default: 18789) |
| `version` | Show version info |

## Channel Connection Protocol

1. Adapter opens WebSocket to `ws://gateway:18789/ws`
2. Sends `ChannelHandshake` (type, version, capabilities, auth token)
3. Gateway validates and responds with `ChannelHandshakeResponse`
4. If accepted, bidirectional `ChannelEvent` JSON messages flow

## Admin API

| Endpoint | Description |
|----------|-------------|
| `GET /admin/health` | Health check with uptime and connection counts |
| `GET /admin/channels` | List connected channel adapters |
| `GET /admin/sessions` | List active user sessions |
| `POST /admin/broadcast` | Send message to all channels |

## Webhook Endpoint

`POST /webhook` accepts JSON `ChannelEvent` payloads for external event injection (CI/CD, payment alerts, etc.).

## Security Middleware

| Middleware | Purpose |
|-----------|---------|
| **Origin Validator** | WebSocket origin checks (configurable allowed origins) |
| **Message Sanitizer** | Strips spoofed system messages, redacts leaked secrets in outbound |
| **Rate Limiter** | Per-user message and tool call budgets |
| **Auth** | Token-based channel adapter authentication |

## Extensibility

The gateway uses a trait-based router (`InboundHandler`). Inject your own handler to customize message processing:

```rust
use brainwires_gateway::{Gateway, InboundHandler, AgentInboundHandler};

// Use the built-in agent handler
let handler = AgentInboundHandler::new(sessions, channels, provider, executor, options);
let gateway = Gateway::with_handler(config, Arc::new(handler));

// Or implement your own
struct MyHandler;
#[async_trait]
impl InboundHandler for MyHandler {
    async fn handle_inbound(&self, channel_id: Uuid, event: &ChannelEvent) -> Result<()> {
        // custom logic
    }
}
```
