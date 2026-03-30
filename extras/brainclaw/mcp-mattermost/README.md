# brainwires-mattermost-channel

Mattermost channel adapter for the Brainwires Gateway. Connects to a Mattermost server via the WebSocket API and bridges messages to/from the gateway for AI agent processing.

## Quick Start

```bash
export MATTERMOST_ACCESS_TOKEN="your-bot-access-token"
export MATTERMOST_SERVER_URL="https://your.mattermost.instance"

cargo run -p brainwires-mattermost-channel -- serve
```

## CLI

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--server-url` | `MATTERMOST_SERVER_URL` | *(required)* | Mattermost server base URL |
| `--access-token` | `MATTERMOST_ACCESS_TOKEN` | *(required)* | Bot user access token |
| `--bot-user-id` | `MATTERMOST_BOT_USER_ID` | *(required)* | Bot's Mattermost user ID |
| `--gateway-url` | `GATEWAY_URL` | `ws://127.0.0.1:18789/ws` | Gateway WebSocket URL |
| `--gateway-token` | `GATEWAY_TOKEN` | — | Auth token for the gateway (optional) |
| `--team-id` | `MATTERMOST_TEAM_ID` | — | Restrict to a specific team (optional) |
| `--group-mention-required` | `GROUP_MENTION_REQUIRED` | `false` | Only respond in channels when @mentioned |
| `--bot-username` | `BOT_USERNAME` | — | Bot username for @mention detection |
| `--mention-patterns` | `MENTION_PATTERNS` | — | Comma-separated trigger keywords for channels |
| `--channel-allowlist` | `CHANNEL_ALLOWLIST` | — | Comma-separated allowed channel IDs (empty = all) |
| `--mcp` | — | `false` | Also start an MCP server on stdio |

## How It Works

1. Connects to Mattermost WebSocket API (`/api/v4/websocket`) for real-time events
2. Filters messages: self-messages, direct message detection, channel allowlists, @mention requirements
3. Forwards `ChannelEvent::MessageReceived` to the brainwires-gateway over WebSocket
4. Receives outbound `ChannelMessage` from gateway and posts via Mattermost REST API

## MCP Server Mode

With `--mcp`, the adapter also serves as an MCP tool server over stdio:

| Tool | Description |
|------|-------------|
| `send_message` | Post a message to a Mattermost channel or direct message |
| `edit_message` | Edit a previously posted message |
| `delete_message` | Delete a message |
| `get_history` | Retrieve recent posts from a channel |
| `add_reaction` | Add an emoji reaction to a post |

## Mattermost Bot Setup

1. Log into Mattermost as a System Admin
2. Go to **Integrations → Bot Accounts → Add Bot Account**
3. Give the bot a username and role
4. Copy the generated **Access Token**
5. Note the bot's **User ID** (visible in the bot account settings)
6. Add the bot to the teams/channels it should have access to

## Capabilities

The Mattermost adapter reports these `ChannelCapabilities`:

`RICH_TEXT | THREADS | REACTIONS | TYPING_INDICATOR | EDIT_MESSAGES | DELETE_MESSAGES | MENTIONS`
