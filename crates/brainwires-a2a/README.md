# brainwires-a2a

[![Crates.io](https://img.shields.io/crates/v/brainwires-a2a.svg)](https://crates.io/crates/brainwires-a2a)
[![Documentation](https://img.shields.io/docsrs/brainwires-a2a)](https://docs.rs/brainwires-a2a)
[![License](https://img.shields.io/crates/l/brainwires-a2a.svg)](LICENSE)

A2A (Agent-to-Agent) protocol implementation for the Brainwires Agent Framework.

## Overview

`brainwires-a2a` implements [Google's Agent-to-Agent protocol](https://google.github.io/A2A/) for interoperable AI agent communication. It provides the types and transport needed for agents to discover each other via Agent Cards, submit tasks, exchange messages, and deliver artifacts — regardless of the underlying framework.

**Design principles:**

- **Standards-based** — follows the A2A specification for cross-framework agent interop
- **Card-driven discovery** — agents advertise capabilities via structured Agent Cards
- **Task lifecycle** — submit, query, and cancel tasks with well-defined state transitions
- **Pluggable auth** — API key, OAuth2, JWT, and Bearer token schemes out of the box
- **Async-first** — all I/O is async via Tokio

```text
  ┌─────────────────────────────────────────────────────────┐
  │                     brainwires-a2a                       │
  │                                                         │
  │  ┌────────────┐     ┌────────────┐     ┌────────────┐  │
  │  │ AgentCard  │────▶│   Task     │────▶│  Artifact  │  │
  │  │ Skills     │     │ TaskState  │     │  (output)  │  │
  │  │ Capabilities│    │ Messages   │     │            │  │
  │  └────────────┘     └────────────┘     └────────────┘  │
  │         │                  │                            │
  │         ▼                  ▼                            │
  │  ┌────────────┐     ┌────────────┐                     │
  │  │ AuthConfig │     │ Transport  │                     │
  │  │ AuthScheme │     │ HTTP + SSE │                     │
  │  └────────────┘     └────────────┘                     │
  └─────────────────────────────────────────────────────────┘

  Flow: AgentCard discovery → Task submission → Message exchange → Artifact delivery
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-a2a = "0.1"
```

Create an Agent Card and submit a task:

```rust
use brainwires_a2a::{
    AgentCard, AgentCapabilities, AgentSkill, Task, TaskState,
    Message, Part, AuthConfig, AuthScheme,
};

// Define an agent's capabilities
let card = AgentCard {
    name: "code-reviewer".into(),
    description: "Reviews pull requests for correctness and style".into(),
    url: "https://agents.example.com/code-reviewer".into(),
    capabilities: AgentCapabilities {
        streaming: true,
        push_notifications: false,
        state_transition_history: true,
    },
    skills: vec![AgentSkill {
        id: "review-pr".into(),
        name: "PR Review".into(),
        description: "Analyzes code changes and provides feedback".into(),
        tags: vec!["code".into(), "review".into()],
        examples: vec!["Review PR #42 for security issues".into()],
    }],
    auth: AuthConfig {
        schemes: vec![AuthScheme::Bearer],
    },
    ..Default::default()
};

// Create a task with a message
let message = Message {
    role: "user".into(),
    parts: vec![Part::text("Review the changes in PR #42")],
};
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `server` | Yes | A2A server support — accept incoming tasks and publish Agent Cards |
| `client` | Yes | A2A client with `reqwest` — discover agents and submit tasks remotely |

```toml
# Server only (no outbound HTTP)
[dependencies]
brainwires-a2a = { version = "0.1", default-features = false, features = ["server"] }

# Client only
[dependencies]
brainwires-a2a = { version = "0.1", default-features = false, features = ["client"] }
```

## Architecture

### Agent Card

Agent Cards are the discovery mechanism — they describe what an agent can do and how to authenticate.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Human-readable agent name |
| `description` | `String` | What the agent does |
| `url` | `String` | Endpoint URL for task submission |
| `capabilities` | `AgentCapabilities` | Streaming, push notifications, history |
| `skills` | `Vec<AgentSkill>` | Tagged skill descriptions with examples |
| `auth` | `AuthConfig` | Supported authentication schemes |
| `provider` | `Option<AgentProvider>` | Organization that hosts the agent |

### Task Lifecycle

Tasks move through well-defined states:

| State | Description |
|-------|-------------|
| `Submitted` | Task received, not yet started |
| `Working` | Agent is actively processing |
| `InputRequired` | Agent needs additional input from the caller |
| `Completed` | Task finished successfully, artifacts available |
| `Failed` | Task failed with error details |
| `Canceled` | Task was canceled by the caller |

### Authentication

| Scheme | Description |
|--------|-------------|
| `ApiKey` | Static API key in header |
| `Bearer` | Bearer token (e.g., from OAuth2 flow) |
| `OAuth2` | Full OAuth2 authorization code flow |
| `Jwt` | JSON Web Token authentication |

### Message Types

Messages contain typed parts for flexible content:

| Part Variant | Description |
|-------------|-------------|
| `Text` | Plain text content |
| `File` | File with name, MIME type, and bytes or URI |
| `Data` | Structured JSON data |

## Usage Examples

### Querying a Remote Agent

```rust
use brainwires_a2a::{AgentCard, Task, TaskSendParams, TaskQueryParams};

// Discover an agent's capabilities
let card: AgentCard = discover_agent("https://agents.example.com/.well-known/agent.json").await?;

// Submit a task
let params = TaskSendParams {
    message: Message {
        role: "user".into(),
        parts: vec![Part::text("Summarize this document")],
    },
    ..Default::default()
};
let task: Task = send_task(&card.url, params).await?;

// Poll for completion
let query = TaskQueryParams { id: task.id.clone() };
let updated: Task = get_task(&card.url, query).await?;
match updated.status.state {
    TaskState::Completed => {
        for artifact in &updated.artifacts {
            println!("Result: {:?}", artifact);
        }
    }
    TaskState::Failed => eprintln!("Task failed"),
    _ => println!("Still processing..."),
}
```

### Defining Multi-Skill Agents

```rust
let card = AgentCard {
    name: "polyglot-assistant".into(),
    description: "Multi-language code assistant".into(),
    url: "https://agents.example.com/polyglot".into(),
    skills: vec![
        AgentSkill {
            id: "translate".into(),
            name: "Code Translation".into(),
            description: "Translates code between programming languages".into(),
            tags: vec!["translation".into(), "code".into()],
            examples: vec!["Convert this Python to Rust".into()],
        },
        AgentSkill {
            id: "explain".into(),
            name: "Code Explanation".into(),
            description: "Explains code in plain language".into(),
            tags: vec!["explanation".into(), "documentation".into()],
            examples: vec!["Explain what this function does".into()],
        },
    ],
    ..Default::default()
};
```

### Handling Artifacts

```rust
// Artifacts are the outputs of completed tasks
for artifact in &task.artifacts {
    for part in &artifact.parts {
        match part {
            Part::Text { text } => println!("Text: {text}"),
            Part::File { name, mime_type, .. } => {
                println!("File: {name} ({mime_type})");
            }
            Part::Data { data } => {
                println!("Structured: {}", serde_json::to_string_pretty(data)?);
            }
        }
    }
}
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["a2a"] }
```

Or depend on `brainwires-a2a` directly for standalone A2A protocol support without the rest of the framework.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
