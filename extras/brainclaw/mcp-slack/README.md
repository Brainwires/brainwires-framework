# brainwires-slack-channel

Slack channel adapter for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework). Connects Slack workspaces to the brainwires-gateway using Socket Mode (WebSocket-based, no public URL needed) and optionally serves as an MCP tool server.

## Quick Start

```bash
# Set your tokens
export SLACK_BOT_TOKEN="xoxb-your-bot-token"
export SLACK_APP_TOKEN="xapp-your-app-token"

# Run the adapter
cargo run -p brainwires-slack-channel -- serve

# With MCP server enabled
cargo run -p brainwires-slack-channel -- serve --mcp

# With explicit gateway URL
cargo run -p brainwires-slack-channel -- serve \
  --gateway-url ws://127.0.0.1:18789/ws \
  --gateway-token my-secret
```

## Slack App Setup

### 1. Create a Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps) and click **Create New App**
2. Choose **From scratch**, give it a name, and select your workspace

### 2. Enable Socket Mode

1. Go to **Settings > Socket Mode**
2. Toggle **Enable Socket Mode** to on
3. Create an app-level token with the `connections:write` scope
4. Save the token (starts with `xapp-`) -- this is your `SLACK_APP_TOKEN`

### 3. Configure Event Subscriptions

1. Go to **Features > Event Subscriptions**
2. Toggle **Enable Events** to on
3. Under **Subscribe to bot events**, add:
   - `message.channels` -- messages in public channels
   - `message.groups` -- messages in private channels
   - `message.im` -- direct messages
   - `message.mpim` -- group direct messages
   - `reaction_added` -- reaction events
   - `reaction_removed` -- reaction removal events

### 4. Configure OAuth & Permissions

1. Go to **Features > OAuth & Permissions**
2. Under **Bot Token Scopes**, add:
   - `channels:history` -- read public channel messages
   - `channels:read` -- list public channels
   - `chat:write` -- send messages
   - `groups:history` -- read private channel messages
   - `groups:read` -- list private channels
   - `im:history` -- read DM messages
   - `im:read` -- list DMs
   - `mpim:history` -- read group DM messages
   - `reactions:read` -- read reactions
   - `reactions:write` -- add reactions
   - `users:read` -- resolve user display names
3. Install (or reinstall) the app to your workspace
4. Copy the **Bot User OAuth Token** (starts with `xoxb-`) -- this is your `SLACK_BOT_TOKEN`

### 5. Invite the Bot

Invite the bot to channels where it should operate:
```
/invite @your-bot-name
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SLACK_BOT_TOKEN` | Yes | Bot user OAuth token (`xoxb-...`) |
| `SLACK_APP_TOKEN` | Yes | App-level token for Socket Mode (`xapp-...`) |
| `GATEWAY_URL` | No | Gateway WebSocket URL (default: `ws://127.0.0.1:18789/ws`) |
| `GATEWAY_TOKEN` | No | Authentication token for gateway handshake |

## MCP Tools

When started with `--mcp`, the adapter exposes these tools over stdio:

| Tool | Description |
|------|-------------|
| `send_message` | Send a message to a Slack channel |
| `edit_message` | Edit a previously sent message |
| `delete_message` | Delete a message |
| `get_history` | Fetch recent message history |
| `add_reaction` | Add an emoji reaction to a message |

## Architecture

```
Slack (Socket Mode WS) <-> SlackEventHandler <-> event channel <-> GatewayClient <-> brainwires-gateway
                                                                         |
                                                                    SlackChannel (Web API)
                                                                         |
                                                                    SlackMcpServer (stdio)
```

The adapter uses two connections:
- **Socket Mode WebSocket**: Receives events from Slack in real time (no public URL needed)
- **Slack Web API**: Sends messages, edits, reactions, etc. via HTTP

## Channel Capabilities

- Rich text (mrkdwn)
- Media uploads
- Threads
- Reactions
- Typing indicator (automatic in Socket Mode)
- Edit messages
- Delete messages
- Mentions
