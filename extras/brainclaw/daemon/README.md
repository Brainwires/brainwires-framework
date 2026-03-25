# BrainClaw

Personal AI assistant daemon built on the Brainwires Framework. A secure, self-hosted alternative to OpenClaw.

## Quick Start

```bash
# Set your provider API key
export ANTHROPIC_API_KEY="your-key"

# Run with defaults
cargo run -p brainclaw -- serve

# Or with options
cargo run -p brainclaw -- serve --provider anthropic --model claude-sonnet-4-20250514 --port 18789
```

## What BrainClaw Does

BrainClaw is a single daemon that:
1. Runs a WebSocket gateway for messaging channel adapters
2. Creates per-user AI agent sessions with tool access
3. Routes messages from channels through agents and back
4. Applies security middleware (sanitization, rate limiting, origin checks)

```text
Discord ──┐                ┌─────────────────────────────────┐
          │  WebSocket     │          BrainClaw               │
Telegram ─┼───────────────►│  Gateway → AgentInboundHandler   │
          │                │             ├─► ChatAgent (user1) │
Slack ────┘                │             └─► ChatAgent (user2) │
                           │  Security Middleware              │
                           │  Admin API / Webhooks             │
                           └─────────────────────────────────┘
```

## Configuration

BrainClaw reads from `~/.brainclaw/brainclaw.toml` (or `./brainclaw.toml`). See `brainclaw.example.toml` for all options.

```toml
[provider]
default_provider = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"

[persona]
name = "BrainClaw"

[tools]
enabled = ["bash", "files", "git", "search", "web", "validation"]

[security]
strip_system_spoofing = true
redact_secrets_in_output = true
max_messages_per_minute = 20
```

## CLI

| Command | Description |
|---------|-------------|
| `serve` | Start the BrainClaw daemon (default) |
| `config-check` | Validate configuration file |
| `version` | Show version info |

| Flag | Description |
|------|-------------|
| `--config <path>` | Config file path |
| `--host <addr>` | Override bind address |
| `--port <port>` | Override listen port |
| `--provider <name>` | Override AI provider |
| `--model <name>` | Override model |
| `--api-key <key>` | Override API key |

## Security

BrainClaw addresses known AI agent security issues (informed by OpenClaw CVEs):

| Protection | What it does |
|-----------|-------------|
| **Message sanitization** | Strips spoofed system messages from channel input |
| **Secret redaction** | Scans outbound messages for leaked API keys, SSNs, CC numbers |
| **Origin validation** | Configurable allowed WebSocket origins |
| **Rate limiting** | Per-user message and tool call budgets |
| **Token auth** | Channel adapters must authenticate to connect |
| **Session isolation** | Each user gets a separate agent session |
| **Skill signing** | Optional ed25519 verification for skill packages |

## Features

| Feature | Flag | Description |
|---------|------|-------------|
| Native tools | `native-tools` (default) | bash, files, git, search, web, validation |
| Email | `email` | IMAP/SMTP/Gmail email tools |
| Calendar | `calendar` | Google Calendar/CalDAV tools |

```bash
# Build with email and calendar support
cargo build -p brainclaw --features email,calendar
```

## Supported Providers

Anthropic (Claude), OpenAI (GPT), Google (Gemini), Ollama (local), Groq, Together AI, Fireworks, AWS Bedrock, Google Vertex AI
