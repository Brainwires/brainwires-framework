# brainwires-telegram-channel

Telegram channel adapter for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework). Connects a Telegram bot to the brainwires-gateway via WebSocket, forwarding messages bidirectionally. Also exposes Telegram operations as MCP tools.

## Quick Start

### 1. Create a Telegram Bot

1. Open Telegram and message [@BotFather](https://t.me/BotFather)
2. Send `/newbot` and follow the prompts
3. Copy the bot token (e.g., `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11`)

### 2. Run the Adapter

```bash
# Via environment variable
export TELEGRAM_BOT_TOKEN="your-bot-token"
cargo run -p brainwires-telegram-channel -- serve

# Via CLI flag
cargo run -p brainwires-telegram-channel -- serve --telegram-token "your-bot-token"

# With gateway connection
cargo run -p brainwires-telegram-channel -- serve \
  --telegram-token "your-bot-token" \
  --gateway-url ws://127.0.0.1:18789/ws \
  --gateway-token "optional-auth-token"

# With MCP server on stdio
cargo run -p brainwires-telegram-channel -- serve \
  --telegram-token "your-bot-token" \
  --mcp
```

## CLI Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--telegram-token` | `TELEGRAM_BOT_TOKEN` | (required) | Telegram bot token from BotFather |
| `--gateway-url` | `GATEWAY_URL` | `ws://127.0.0.1:18789/ws` | Gateway WebSocket URL |
| `--gateway-token` | `GATEWAY_TOKEN` | (none) | Optional gateway auth token |
| `--mcp` | — | `false` | Enable MCP tool server on stdio |

## Subcommands

- **`serve`** — Start the Telegram adapter (connects to Telegram + gateway)
- **`version`** — Show version and system information

## MCP Tools

When `--mcp` is enabled, the following tools are available:

| Tool | Description |
|------|-------------|
| `send_message` | Send a message to a Telegram chat |
| `edit_message` | Edit a previously sent message |
| `delete_message` | Delete a message |
| `send_typing` | Show typing indicator |
| `add_reaction` | Add emoji reaction to a message |

## Channel Capabilities

This adapter reports the following capabilities to the gateway:

- Rich Text (Telegram MarkdownV2)
- Media Upload
- Reactions
- Typing Indicator
- Edit Messages
- Delete Messages
- Mentions

## Architecture

```
Telegram Bot API  <-->  teloxide dispatcher  <-->  event_tx/rx  <-->  Gateway WebSocket
                                                                 |
                                                            MCP Server (stdio)
```

The adapter runs three concurrent tasks:
1. **Teloxide dispatcher** — receives Telegram updates and converts them to `ChannelEvent`
2. **Gateway client** — forwards events to the gateway and relays outbound messages back
3. **MCP server** (optional) — exposes tools for direct programmatic access

## License

MIT OR Apache-2.0
