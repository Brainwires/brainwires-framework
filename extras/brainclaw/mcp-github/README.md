# brainclaw-mcp-github

GitHub channel adapter for BrainClaw. Receives GitHub webhook events and exposes GitHub operations as an MCP tool server, bridging GitHub activity to the Brainwires gateway for AI agent processing.

## Quick Start

```bash
# Required: GitHub personal access token (or GitHub App token)
export GITHUB_TOKEN="ghp_..."

# Optional: webhook secret for signature verification
export GITHUB_WEBHOOK_SECRET="my-secret"

# Run the adapter (connects to gateway at default localhost:18789)
cargo run -p brainclaw-mcp-github -- serve
```

GitHub will need to deliver webhooks to your `WEBHOOK_ADDR` (default `0.0.0.0:9000`). Use a tunnel like `ngrok` or `cloudflared` for local development.

## CLI

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--github-token` | `GITHUB_TOKEN` | — | GitHub PAT or App token (required) |
| `--webhook-secret` | `GITHUB_WEBHOOK_SECRET` | — | HMAC-SHA256 secret for webhook verification |
| `--webhook-addr` | `WEBHOOK_ADDR` | `0.0.0.0:9000` | Address to bind the webhook HTTP server |
| `--gateway-url` | `GATEWAY_URL` | `ws://127.0.0.1:18789/ws` | Brainwires gateway WebSocket URL |
| `--gateway-token` | `GATEWAY_TOKEN` | — | Auth token for the gateway (optional) |
| `--repos` | `GITHUB_REPOS` | *(all)* | Comma-separated `owner/repo` allowlist |
| `--api-url` | `GITHUB_API_URL` | `https://api.github.com` | GitHub API base URL (for GHE) |
| `--mcp` | — | false | Also start an MCP server on stdio |

## How It Works

1. GitHub delivers webhook payloads to the HTTP server (`/webhook`)
2. Incoming payloads are HMAC-SHA256 verified (if `webhook_secret` is set)
3. Event type and repo allowlist are checked; ignored events return `200 OK`
4. Accepted events are normalised to `ChannelMessage` and forwarded via `mpsc`
5. Gateway client wraps each message in `ChannelEvent::MessageReceived` and sends it over WebSocket to the Brainwires gateway
6. Responses from the gateway are dispatched back to GitHub (post comment, add label, etc.)

## Supported Webhook Events

Configure your GitHub webhook to send these event types:

| GitHub Event | Description |
|---|---|
| `issue_comment` | Comment created or edited on an issue or PR |
| `issues` | Issue opened, closed, labeled, etc. |
| `pull_request` | PR opened, closed, merged, etc. |
| `pull_request_review_comment` | Inline review comment on a PR diff |

## MCP Server Mode

With `--mcp`, the adapter also serves as an MCP tool server over stdio:

| Tool | Description |
|------|-------------|
| `post_comment` | Post a comment on an issue or PR |
| `edit_comment` | Edit an existing comment |
| `delete_comment` | Delete a comment |
| `get_comments` | List comments on an issue or PR |
| `create_issue` | Open a new issue |
| `close_issue` | Close an existing issue |
| `add_labels` | Add labels to an issue or PR |
| `create_pull_request` | Open a new pull request |
| `merge_pull_request` | Merge a pull request |
| `add_reaction` | Add an emoji reaction to a comment |

## GitHub Webhook Setup

1. Go to your repository → **Settings** → **Webhooks** → **Add webhook**
2. Set **Payload URL** to your public endpoint (e.g. `https://your-tunnel.example.com/webhook`)
3. Set **Content type** to `application/json`
4. Set **Secret** to the same value as `GITHUB_WEBHOOK_SECRET`
5. Select **Let me select individual events** and enable: Issues, Issue comments, Pull requests, Pull request review comments
6. Save the webhook

## Conversation ID Scheme

Each GitHub issue or PR maps to a `ConversationId`:

```
platform:   "github"
channel_id: "owner/repo#<issue_number>"
server_id:  None
```

Each comment maps to a `MessageId`:

```
"owner/repo/<comment_id>"
```

This scheme ensures all comments on the same issue/PR thread are grouped into a single conversation.

## Capabilities

The GitHub adapter reports these `ChannelCapabilities`:

`REACTIONS | EDIT_MESSAGES | DELETE_MESSAGES | MENTIONS`

## GitHub App vs PAT

Both authentication methods work. For production use, a **GitHub App** is recommended:

- Scoped permissions (Issues: read/write, Pull requests: read/write)
- Higher rate limits (5000 req/hr per installation vs 5000 req/hr per PAT)
- No expiry (unlike fine-grained PATs which expire after at most 1 year)

Generate an installation token and set it as `GITHUB_TOKEN`.
