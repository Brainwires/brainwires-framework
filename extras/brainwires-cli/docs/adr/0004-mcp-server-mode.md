# ADR 0004: MCP Server Mode for Agent Management

## Status

Accepted

## Context

AI assistants like Claude Desktop need to spawn and manage autonomous coding agents. Rather than building this capability into each client, we can expose brainwires-cli as an MCP (Model Context Protocol) server.

Requirements:
- Allow external clients to spawn agents
- Provide agent status monitoring
- Support agent lifecycle management (stop, await)
- Maintain file lock state visibility

## Options Considered

### 1. Custom RPC Protocol

**Pros:**
- Full control over protocol design
- Optimized for our specific needs

**Cons:**
- No ecosystem compatibility
- Every client needs custom integration
- Documentation/maintenance burden

### 2. REST API

**Pros:**
- Well-understood pattern
- Many client libraries available

**Cons:**
- Requires HTTP server setup
- Stateless by default (awkward for agents)
- Long-polling needed for streaming

### 3. MCP Server Mode (Chosen)

**Pros:**
- Standard protocol for AI tool integration
- Claude Desktop native support
- JSON-RPC over stdio (simple, reliable)
- Bidirectional communication

**Cons:**
- Tied to MCP protocol evolution
- Less flexible than custom protocol

## Decision

Implement **MCP server mode** via the `--mcp-server` flag:

```bash
cargo run -- chat --mcp-server
```

This exposes agent management as MCP tools that any MCP client can use.

## Available MCP Tools

| Tool | Description |
|------|-------------|
| `agent_spawn` | Spawn a new task agent |
| `agent_list` | List all active agents |
| `agent_status` | Get status of specific agent |
| `agent_stop` | Stop a running agent |
| `agent_await` | Wait for agent completion |
| `agent_pool_stats` | Get agent pool statistics |
| `agent_file_locks` | List current file locks |

## Tool Schema: `agent_spawn`

```json
{
  "name": "agent_spawn",
  "arguments": {
    "description": "Task description",
    "working_directory": "/path/to/project",
    "max_iterations": 20,
    "enable_validation": true,
    "build_type": "typescript",
    "enable_mdap": true,
    "mdap_preset": "high_reliability"
  }
}
```

## Architecture

```
┌─────────────────┐         ┌─────────────────┐
│  Claude Desktop │◄──MCP──►│ brainwires-cli  │
│  (MCP Client)   │         │ (MCP Server)    │
└─────────────────┘         └────────┬────────┘
                                     │
                            ┌────────┴────────┐
                            │   Agent Pool    │
                            │                 │
                            │ ┌─────────────┐ │
                            │ │ TaskAgent 1 │ │
                            │ └─────────────┘ │
                            │ ┌─────────────┐ │
                            │ │ TaskAgent 2 │ │
                            │ └─────────────┘ │
                            └─────────────────┘
```

## Usage Flow

1. Client (Claude Desktop) connects to brainwires-cli via MCP
2. Client calls `agent_spawn` with task description
3. brainwires-cli creates TaskAgent and returns agent ID
4. Client can poll `agent_status` or call `agent_await`
5. Agent executes autonomously using tools
6. On completion, result returned to client

## Consequences

### Positive
- Claude Desktop can orchestrate complex coding tasks
- Standard protocol means other MCP clients work too
- Clear separation between orchestration and execution
- Agent state visible via MCP tools

### Negative
- Dependent on MCP protocol stability
- Stdio communication limits some use cases
- Requires MCP client for interaction

### Mitigations
- Use `rmcp` crate for protocol compliance
- Test with multiple MCP clients
- Provide fallback interactive mode

## References

- `src/mcp_server/` - MCP server implementation
- `src/mcp_server/agent_tools.rs` - Agent management tools
- [MCP Specification](https://spec.modelcontextprotocol.io/)
