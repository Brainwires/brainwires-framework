# brainwires-discord-channel

Discord channel adapter for the Brainwires Gateway. Connects to Discord as a bot and bridges messages to/from the gateway for AI agent processing.

This is a **reference implementation** — use it as a template for building adapters for other platforms (Telegram, Slack, etc.).

## Quick Start

```bash
# Set your Discord bot token
export DISCORD_TOKEN="your-bot-token"

# Run the adapter (connects to gateway at default localhost:18789)
cargo run -p brainwires-discord-channel -- serve
```

## CLI

| Flag | Description |
|------|-------------|
| `--discord-token` | Discord bot token (or `DISCORD_TOKEN` env var) |
| `--gateway-url` | Gateway WebSocket URL (default: `ws://127.0.0.1:18789/ws`) |
| `--gateway-token` | Auth token for the gateway (optional) |
| `--bot-prefix` | Command prefix for the bot (optional) |
| `--mcp` | Also start an MCP server on stdio |

## How It Works

1. Connects to Discord via serenity (Discord API library)
2. Opens WebSocket to the brainwires-gateway
3. Sends `ChannelHandshake` with Discord capabilities
4. Forwards Discord events as `ChannelEvent` to the gateway
5. Receives responses from the gateway and posts them to Discord

## MCP Server Mode

With `--mcp`, the adapter also serves as an MCP tool server over stdio:

| Tool | Description |
|------|-------------|
| `send_message` | Send a message to a Discord channel |
| `edit_message` | Edit a previously sent message |
| `delete_message` | Delete a message |
| `get_history` | Get conversation history |
| `list_channels` | List available channels/servers |
| `send_typing` | Show typing indicator |
| `add_reaction` | React to a message |

## Discord Bot Setup

1. Create a Discord application at https://discord.com/developers/applications
2. Create a bot and copy the token
3. Enable the **Message Content** privileged intent
4. Invite the bot to your server with message read/write permissions

## Capabilities

The Discord adapter reports these `ChannelCapabilities`:

`RICH_TEXT | MEDIA_UPLOAD | THREADS | REACTIONS | TYPING_INDICATOR | EDIT_MESSAGES | DELETE_MESSAGES | MENTIONS | EMBEDS`
