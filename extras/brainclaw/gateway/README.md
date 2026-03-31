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

All admin endpoints require an `Authorization: Bearer <token>` header when `admin_token` is configured.

| Endpoint | Description |
|----------|-------------|
| `GET /admin/ui` | **Browser dashboard** — single-page admin UI (dark theme) |
| `GET /admin/health` | Health check with uptime and connection counts |
| `GET /admin/metrics` | Token usage, tool calls, errors, rate limits, per-channel message counts |
| `GET /admin/channels` | List connected channel adapters |
| `GET /admin/sessions` | List active user sessions |
| `POST /admin/broadcast` | Send message to all (or filtered) channels |
| `GET /admin/cron` | List cron jobs *(requires cron store)* |
| `POST /admin/cron` | Create a cron job |
| `GET /admin/cron/:id` | Get a single cron job |
| `PUT /admin/cron/:id` | Update a cron job |
| `DELETE /admin/cron/:id` | Delete a cron job |
| `GET /admin/identity` | List canonical identity groups *(requires identity store)* |
| `POST /admin/identity/link` | Link two platform identities |
| `DELETE /admin/identity/unlink` | Unlink a platform identity |

### Admin UI

Open `http://your-gateway:18789/admin/ui` in a browser for a graphical dashboard covering all of the above endpoints. Sections: **Dashboard** (live stats + auto-refresh), **Channels**, **Sessions**, **Cron Jobs** (full CRUD with modal), **Identity** (link/unlink), **Broadcast**. Bearer token auth is handled in-browser via `sessionStorage`.

## Webhook Endpoint

`POST /webhook` accepts JSON `ChannelEvent` payloads for external event injection (CI/CD, payment alerts, etc.).

When `webhook_secret` is configured, requests must include an `X-Webhook-Signature` header containing the hex-encoded HMAC-SHA256 of the request body.

## Security

| Layer | What it does |
|-------|-------------|
| **Message Sanitizer** | Detects spoofed system messages on inbound, redacts secrets (API keys, SSNs, CC numbers) on outbound |
| **Rate Limiter** | Per-user message and tool call budgets, enforced per request |
| **Origin Validator** | WebSocket origin whitelist (configurable, wildcard subdomain support) |
| **Channel Auth** | Token-based channel adapter authentication at handshake |
| **Admin Auth** | Bearer token authentication on all /admin/* endpoints |
| **Webhook HMAC** | HMAC-SHA256 signature verification on webhook payloads |
| **Audit Logger** | Structured JSON audit trail — auth failures, rate limits, spoofing attempts, tool calls, session lifecycle |
| **Metrics** | Atomic counters for messages, tool calls, errors, rate limits, spoofing blocks, per-channel breakdowns |

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
