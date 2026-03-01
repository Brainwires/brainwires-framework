# brainwires-extras

[![Crates.io](https://img.shields.io/crates/v/brainwires-extras.svg)](https://crates.io/crates/brainwires-extras)
[![Documentation](https://img.shields.io/docsrs/brainwires-extras)](https://docs.rs/brainwires-extras)
[![License](https://img.shields.io/crates/l/brainwires-extras.svg)](LICENSE)

Small utilities and example MCP servers for the Brainwires Agent Framework.

## Overview

`brainwires-extras` is a catch-all crate for utilities and example servers too small for their own dedicated crate. It ships as a library (currently minimal) plus standalone example binaries that demonstrate real-world patterns — config-driven MCP servers, process lifecycle management, and AI client integration.

**Design principles:**

- **Example-first** — self-contained, runnable examples with production-quality patterns
- **Config-driven** — behaviour controlled by JSON config files, not hardcoded logic
- **MCP-native** — examples build on `rmcp` and expose tools via Streamable HTTP transport
- **Unix-ready** — process management with escalating signals, detached spawning, and graceful shutdown

```text
  ┌──────────────────────────────────────────────────────────────┐
  │                    brainwires-extras                          │
  │                                                              │
  │  ┌──────────────────────────────────────────────────────┐   │
  │  │                 reload_daemon                         │   │
  │  │                                                      │   │
  │  │  config.json ──► DaemonConfig                        │   │
  │  │                     │                                │   │
  │  │                     ▼                                │   │
  │  │  ┌────────────────────────────────┐                  │   │
  │  │  │        ReloadServer            │                  │   │
  │  │  │   (MCP ServerHandler)          │                  │   │
  │  │  │                                │                  │   │
  │  │  │   reload_app tool              │                  │   │
  │  │  │     │                          │                  │   │
  │  │  │     ├─► validate binary name   │                  │   │
  │  │  │     ├─► transform args         │                  │   │
  │  │  │     ├─► kill (escalating)      │                  │   │
  │  │  │     │   SIGINT → SIGTERM →     │                  │   │
  │  │  │     │   SIGKILL                │                  │   │
  │  │  │     └─► spawn replacement      │                  │   │
  │  │  └────────────────────────────────┘                  │   │
  │  │                     │                                │   │
  │  │           Axum HTTP (/mcp)                           │   │
  │  │                     │                                │   │
  │  │  Claude Code / Cursor / AI Client ◄──────────────────│   │
  │  └──────────────────────────────────────────────────────┘   │
  │                                                              │
  │  ┌──────────────────────────────────────────────────────┐   │
  │  │  Future utilities and examples go here               │   │
  │  └──────────────────────────────────────────────────────┘   │
  └──────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-extras = "0.1"
```

Build and run the `reload_daemon` example:

```sh
# Build the example binary
cargo build -p brainwires-extras --example reload_daemon

# Run with a config file
cargo run -p brainwires-extras --example reload_daemon -- \
  --config crates/brainwires-extras/examples/reload_daemon/config.json

# Register with Claude Code as an MCP server
claude mcp add --transport http reload-daemon http://127.0.0.1:3100/mcp
```

## Examples

### reload_daemon

A minimal MCP server daemon that AI coding clients (Claude Code, Cursor, etc.) connect to over HTTP. It exposes one tool — `reload_app` — which kills the calling process and restarts it with transformed arguments. Restart strategies are config-driven.

**Use case:** Rapid development iteration when building/testing MCP servers. An app restart reconnects all MCP sessions, so changes take effect immediately.

```text
  AI Client (Claude Code)           reload_daemon
  ┌──────────────────┐              ┌────────────────────┐
  │                  │   reload_app │                    │
  │  calls tool ─────┼─────────────►  validate caller   │
  │                  │              │       │            │
  │                  │              │  kill process      │
  │  ✕ (killed)      │◄────────────┤  (SIGINT/TERM/KILL)│
  │                  │              │       │            │
  │                  │              │  spawn replacement  │
  │  ✓ (restarted)   │◄────────────┤  (detached)        │
  │                  │              │                    │
  │  reconnects MCP  │──────────────►                    │
  └──────────────────┘              └────────────────────┘
```

## Architecture

### Configuration

All behaviour is driven by a JSON config file loaded at startup.

**`DaemonConfig`:**

| Field | Type | Description |
|-------|------|-------------|
| `listen` | `String` | Address to bind (e.g. `"127.0.0.1:3100"`) |
| `clients` | `HashMap<String, ClientStrategy>` | Per-client restart strategies keyed by client type |

**`ClientStrategy`:**

| Field | Type | Description |
|-------|------|-------------|
| `process_name` | `String` | Expected binary name — defence against mis-targeting |
| `kill_signals` | `Vec<String>` | Signal names in escalation order |
| `kill_timeouts_ms` | `Vec<u64>` | Timeout per signal (`0` = fire-and-forget) |
| `restart_args_transform` | `Option<ArgsTransform>` | Optional argument transformation |

**`ArgsTransform`:**

| Field | Type | Description |
|-------|------|-------------|
| `preserve_flags` | `Vec<String>` | Flags to keep from original argv |
| `replace_trailing` | `Vec<String>` | Args to append at end |

**Supported signals:** `SIGINT`, `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGUSR1`, `SIGUSR2`

### MCP Server

`ReloadServer` implements `rmcp::ServerHandler` and exposes the `reload_app` tool via the `#[tool]` macro.

**`ReloadServer`:**

```rust
#[derive(Clone)]
pub struct ReloadServer {
    config: Arc<DaemonConfig>,
    tool_router: ToolRouter<Self>,
}
```

**`ReloadAppRequest`** (tool input schema):

| Field | Type | Description |
|-------|------|-------------|
| `client_type` | `String` | Key into the `clients` config map (e.g. `"claude-code"`) |
| `pid` | `i32` | PID of the process to kill |
| `original_args` | `Vec<String>` | Original argv of the process |
| `working_directory` | `String` | Working directory for the restarted process |

The tool performs these steps in order:

1. **Look up strategy** — find `ClientStrategy` by `client_type`
2. **Validate binary name** — compare `original_args[0]` against `process_name`
3. **Transform args** — apply `ArgsTransform` if configured (preserve flags + append trailing)
4. **Kill process** — send escalating signals with configurable timeouts
5. **Spawn replacement** — detached process that outlives the handler

### Process Management

Process kill and spawn functions in the `reload` module (Unix-only).

**`kill_process(pid, strategy)`** — escalating signal strategy:

- Iterates through `kill_signals` paired with `kill_timeouts_ms`
- For each signal: send → poll for exit → escalate on timeout
- A timeout of `0` means fire-and-forget (used for `SIGKILL`)
- Returns `Ok(())` if process exits at any stage or was already dead (`ESRCH`)

**`spawn_process(program, args, cwd)`** — starts a detached replacement:

- Spawns via `std::process::Command` with null stdio
- Detaches by calling `std::mem::forget(child)` so the process is adopted by init
- Returns immediately after spawn

**`transform_args(original_args, transform)`** — builds the new argv:

- Keeps flags from original args that appear in `preserve_flags`
- Appends `replace_trailing` args at the end

### HTTP Transport

The daemon runs as an Axum HTTP server with a single route:

- **Endpoint:** `/mcp` — Streamable HTTP MCP transport
- **Session management:** `LocalSessionManager` (per-connection sessions)
- **Graceful shutdown:** listens for `CTRL+C` via `tokio::signal`

## Usage Examples

### Example Config File

```json
{
  "listen": "127.0.0.1:3100",
  "clients": {
    "claude-code": {
      "process_name": "claude",
      "kill_signals": ["SIGINT", "SIGTERM", "SIGKILL"],
      "kill_timeouts_ms": [2000, 3000, 0],
      "restart_args_transform": {
        "preserve_flags": ["--allow-dangerously-skip-permissions"],
        "replace_trailing": ["--continue", "continue"]
      }
    }
  }
}
```

This config tells the daemon:
- Wait 2s after `SIGINT` before escalating to `SIGTERM`
- Wait 3s after `SIGTERM` before escalating to `SIGKILL`
- `SIGKILL` with `0` timeout — fire-and-forget
- Keep `--allow-dangerously-skip-permissions` from the original command
- Append `--continue continue` to restart with session continuation

### Run the Daemon

```sh
# Start with default config
cargo run -p brainwires-extras --example reload_daemon -- \
  --config crates/brainwires-extras/examples/reload_daemon/config.json

# Start with debug logging
RUST_LOG=debug cargo run -p brainwires-extras --example reload_daemon -- \
  --config crates/brainwires-extras/examples/reload_daemon/config.json
```

### Register with Claude Code

```sh
# Add as an HTTP-based MCP server
claude mcp add --transport http reload-daemon http://127.0.0.1:3100/mcp
```

Once registered, Claude Code can call the `reload_app` tool:

```json
{
  "name": "reload_app",
  "arguments": {
    "client_type": "claude-code",
    "pid": 12345,
    "original_args": ["claude", "--allow-dangerously-skip-permissions", "some-project"],
    "working_directory": "/home/user/dev/my-project"
  }
}
```

### Multiple Client Strategies

```json
{
  "listen": "127.0.0.1:3100",
  "clients": {
    "claude-code": {
      "process_name": "claude",
      "kill_signals": ["SIGINT", "SIGTERM", "SIGKILL"],
      "kill_timeouts_ms": [2000, 3000, 0],
      "restart_args_transform": {
        "preserve_flags": ["--allow-dangerously-skip-permissions"],
        "replace_trailing": ["--continue", "continue"]
      }
    },
    "cursor": {
      "process_name": "cursor",
      "kill_signals": ["SIGTERM", "SIGKILL"],
      "kill_timeouts_ms": [5000, 0],
      "restart_args_transform": null
    }
  }
}
```

### Gentle Restart (Single Signal)

```json
{
  "listen": "127.0.0.1:3100",
  "clients": {
    "my-app": {
      "process_name": "my-app",
      "kill_signals": ["SIGHUP"],
      "kill_timeouts_ms": [10000],
      "restart_args_transform": null
    }
  }
}
```

Uses a single `SIGHUP` with a 10-second timeout — suitable for apps that handle `SIGHUP` as a reload signal.

### Custom Arg Transformation

```json
{
  "restart_args_transform": {
    "preserve_flags": ["--verbose", "--config"],
    "replace_trailing": ["--restart-reason", "mcp-reload"]
  }
}
```

Given original args `["my-app", "--verbose", "--port", "8080", "--config"]`, the restarted process receives `["my-app", "--verbose", "--config", "--restart-reason", "mcp-reload"]` — non-preserved flags are dropped and trailing args are appended.

## Configuration

### Config File Format

The daemon reads a single JSON file passed via `--config`:

```sh
reload_daemon --config /path/to/config.json
```

| Top-Level Key | Required | Description |
|---------------|:--------:|-------------|
| `listen` | Yes | Bind address (`"host:port"`) |
| `clients` | Yes | Map of client type names to strategies |

### Signal Escalation

Signals and timeouts are paired positionally. The daemon sends each signal in order, waiting up to the corresponding timeout before escalating:

| Signal | Typical Use | Recommended Timeout |
|--------|-------------|:-------------------:|
| `SIGINT` | Graceful interrupt | 2000–5000 ms |
| `SIGTERM` | Terminate | 3000–5000 ms |
| `SIGKILL` | Force kill (cannot be caught) | 0 ms |
| `SIGHUP` | Hangup / reload | 5000–10000 ms |
| `SIGUSR1` | User-defined | application-specific |
| `SIGUSR2` | User-defined | application-specific |

### Platform Support

| Platform | Status | Notes |
|----------|:------:|-------|
| Linux | Supported | Full signal and process management |
| macOS | Supported | Full signal and process management |
| Windows | Not supported | Returns error — Unix signals not available |

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["extras"] }
```

Or use standalone — `brainwires-extras` has no dependency on any other Brainwires crate.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
