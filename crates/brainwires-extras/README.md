# brainwires-extras

Small utilities and example MCP servers for the Brainwires Agent Framework.

This crate is a catch-all for things too small for their own crate.

## Examples

### `reload_daemon`

A minimal MCP server daemon that AI coding clients (Claude Code, Cursor, etc.)
connect to over HTTP. It exposes one tool — `reload_app` — which kills the
calling process and restarts it with transformed arguments. Restart strategies
are config-driven.

```sh
# Build
cargo build -p brainwires-extras --example reload_daemon

# Run
cargo run -p brainwires-extras --example reload_daemon -- \
  --config crates/brainwires-extras/examples/reload_daemon/config.json

# Register with Claude Code
claude mcp add --transport http reload-daemon http://127.0.0.1:3100/mcp
```
