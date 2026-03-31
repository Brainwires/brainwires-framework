# brainwires-signal-channel

Signal messenger channel adapter for the Brainwires Gateway. Connects to Signal via the `signal-cli-rest-api` daemon and bridges messages to/from the gateway for AI agent processing.

## Prerequisites

A running `signal-cli` daemon in HTTP mode:

```bash
signal-cli -a +14155552671 daemon --http 127.0.0.1:8080
```

Or via Docker:

```bash
docker run -p 8080:8080 bbernhard/signal-cli-rest-api
```

## Quick Start

```bash
export SIGNAL_PHONE_NUMBER="+14155552671"
export GATEWAY_URL="ws://127.0.0.1:18789/ws"

cargo run -p brainwires-signal-channel -- serve
```

## CLI

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--api-url` | `SIGNAL_API_URL` | `http://127.0.0.1:8080` | signal-cli REST API base URL |
| `--phone-number` | `SIGNAL_PHONE_NUMBER` | *(required)* | Bot's E.164 phone number (e.g. `+14155552671`) |
| `--gateway-url` | `GATEWAY_URL` | `ws://127.0.0.1:18789/ws` | Gateway WebSocket URL |
| `--gateway-token` | `GATEWAY_TOKEN` | — | Auth token for the gateway (optional) |
| `--group-mention-required` | `GROUP_MENTION_REQUIRED` | `false` | Only respond in groups when @mentioned |
| `--bot-name` | `BOT_NAME` | — | Bot display name for @mention detection |
| `--mention-patterns` | `MENTION_PATTERNS` | — | Comma-separated trigger keywords for groups |
| `--sender-allowlist` | `SENDER_ALLOWLIST` | — | Comma-separated allowed sender numbers (empty = all) |
| `--group-allowlist` | `GROUP_ALLOWLIST` | — | Comma-separated allowed group IDs in base64 (empty = all) |
| `--poll-interval-ms` | `POLL_INTERVAL_MS` | `2000` | Polling interval in ms (used when WebSocket unavailable) |
| `--mcp` | — | `false` | Also start an MCP server on stdio |

## How It Works

1. Connects to `signal-cli-rest-api` over WebSocket (`/v1/events`) — real-time push
2. Falls back to polling (`GET /v1/receive/{number}`) if WebSocket is unavailable
3. Filters messages: self-messages, sender/group allowlists, @mention requirements
4. Forwards `ChannelEvent::MessageReceived` to the brainwires-gateway over WebSocket
5. Receives outbound `ChannelMessage` from gateway and sends via Signal REST API

## Receive Modes

| Mode | Endpoint | Description |
|------|----------|-------------|
| **WebSocket** (preferred) | `ws://host/v1/events` | Real-time push from signal-cli-rest-api |
| **Polling** (fallback) | `GET /v1/receive/{number}` | Polled at `--poll-interval-ms` interval |

## MCP Server Mode

With `--mcp`, the adapter also serves as an MCP tool server over stdio:

| Tool | Description |
|------|-------------|
| `send_message` | Send a message to a phone number (`+E164`) or group (`group.<base64id>`) |
| `add_reaction` | Add an emoji reaction to a message (ID format: `recipient:author:timestamp`) |

## Capabilities

The Signal adapter reports these `ChannelCapabilities`:

`REACTIONS`

> Signal supports reactions; rich text, threads, edits, and history are not available via the REST API.
