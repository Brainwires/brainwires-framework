# MCP Server Mode

The `brainwires` CLI can run as an **Model Context Protocol (MCP) server**, exposing its AI agents and tools to other applications like Claude Desktop, IDEs, and custom MCP clients.

## Overview

MCP server mode transforms the CLI into a background service that:
- Exposes all CLI tools as MCP tools
- Provides agent management capabilities
- Enables hierarchical task decomposition
- Supports parallel agent execution

## Quick Start

### Basic Usage

```bash
# Start as MCP server (listens on stdin/stdout)
brainwires chat --mcp-server

# With specific model
brainwires chat --mcp-server --model claude-3-5-sonnet-20241022

# With custom system prompt
brainwires chat --mcp-server --system "You are a specialized code reviewer"
```

### Using with Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "brainwires": {
      "command": "/path/to/brainwires",
      "args": ["chat", "--mcp-server"],
      "env": {
        "ANTHROPIC_API_KEY": "your-key-here"
      }
    }
  }
}
```

Restart Claude Desktop, and you'll have access to brainwires tools in your conversations.

## Available Tools

### Agent Management Tools

#### `agent_spawn`
Spawn an autonomous task agent to work on a subtask in the background.

**Parameters:**
- `task` (string, required): Description of the task to execute

**Example:**
```json
{
  "name": "agent_spawn",
  "arguments": {
    "task": "Analyze the authentication system and create a security report"
  }
}
```

**Returns:** Agent ID and confirmation message

#### `agent_list`
List all currently running task agents and their status.

**Parameters:** None

**Returns:** List of agents with IDs, status, and task descriptions

#### `agent_status`
Get detailed status information about a specific agent.

**Parameters:**
- `agent_id` (string, required): ID of the agent to query

**Returns:** Agent status, current task, iterations, and progress

#### `agent_stop`
Stop a running task agent.

**Parameters:**
- `agent_id` (string, required): ID of the agent to stop

**Returns:** Confirmation message

### Built-in Tools

All standard brainwires tools are also exposed:
- File operations (read, write, list directory)
- Code search (query_codebase)
- Git operations
- Bash execution
- And more...

## Architecture

### Hierarchical Task Management

Task agents can spawn sub-agents, creating a tree structure:

```
Main Agent (Task: "Refactor authentication system")
├── Sub-Agent 1 (Task: "Analyze current implementation")
├── Sub-Agent 2 (Task: "Design new architecture")
└── Sub-Agent 3 (Task: "Write migration plan")
    ├── Sub-Agent 3.1 (Task: "Create database migration")
    └── Sub-Agent 3.2 (Task: "Update API endpoints")
```

### Communication Hub

Agents communicate through a central hub:
- Status updates
- Help requests
- Task results
- Error reporting

### File Lock Management

Prevents conflicts when multiple agents access files:
- **Read locks** - Multiple agents can read simultaneously
- **Write locks** - Exclusive access for modifications
- Automatic lock release on completion

## Protocol Details

### JSON-RPC 2.0

The server implements JSON-RPC 2.0 over stdin/stdout:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "agent_spawn",
    "arguments": {
      "task": "Analyze codebase structure"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Spawned task agent 'agent-uuid' for task 'Analyze codebase structure'"
      }
    ],
    "isError": false
  }
}
```

### Supported Methods

- `initialize` - Handshake and capability negotiation
- `tools/list` - List all available tools
- `tools/call` - Execute a tool

### Capabilities

The server advertises these capabilities:

```json
{
  "tools": {
    "listChanged": false
  }
}
```

## Use Cases

### 1. Code Analysis Assistant

Spawn agents to analyze different aspects of a codebase:

```
User: "Analyze this React application"
→ agent_spawn("Analyze component structure")
→ agent_spawn("Check for security vulnerabilities")
→ agent_spawn("Review performance patterns")
→ agent_list() → "3 agents running"
```

### 2. Parallel Development Tasks

Break down large features into parallel subtasks:

```
agent_spawn("Implement user authentication")
  → Spawns sub-agents for:
    - Database schema
    - API endpoints
    - Frontend forms
    - Tests
```

### 3. Continuous Monitoring

Long-running agents for ongoing tasks:

```
agent_spawn("Monitor test suite and report failures")
agent_spawn("Watch for security vulnerabilities in dependencies")
```

### 4. IDE Integration

Connect your IDE to brainwires agents:
- Refactoring suggestions
- Test generation
- Documentation writing
- Code review

## Configuration

### Environment Variables

- `ANTHROPIC_API_KEY` - Anthropic API key for Claude models
- `BRAINWIRES_SESSION` - Alternative: Brainwires Studio session
- `RUST_LOG` - Logging level (e.g., `debug`, `info`)

### Model Selection

```bash
# Use specific Claude model
brainwires chat --mcp-server --model claude-3-5-sonnet-20241022

# Uses default model from config if not specified
brainwires chat --mcp-server
```

### System Prompt Customization

Customize agent behavior with system prompts:

```bash
brainwires chat --mcp-server --system "You are an expert in Rust programming. Focus on memory safety and performance."
```

## Performance

### Resource Usage

- **Memory**: ~50MB base + ~10MB per active agent
- **CPU**: Varies by agent activity
- **Network**: API calls to AI provider

### Limits

- **Max iterations per agent**: 15 (configurable)
- **Concurrent agents**: Unlimited (limited by system resources)
- **File locks**: No limit

### Optimization Tips

1. **Reuse agents** - Stop completed agents to free resources
2. **Specific tasks** - Clear task descriptions improve agent efficiency
3. **Monitor progress** - Use `agent_status` to track agent work

## Security Considerations

### Tool Execution

- Agents run with same permissions as the CLI process
- **Auto mode**: Tools execute without confirmation
- **Bash execution**: Commands run in the working directory
- **File access**: Agents can read/write files

### Best Practices

1. **Run in isolated environments** for untrusted tasks
2. **Review agent tasks** before spawning
3. **Monitor agent activity** with `agent_list`
4. **Use read-only operations** when possible
5. **Set clear task boundaries** to prevent scope creep

### API Keys

Store API keys securely:
- Use environment variables
- Never commit keys to repositories
- Rotate keys regularly
- Use minimal permissions

## Troubleshooting

### Server Not Starting

```bash
# Check for configuration issues
brainwires chat --mcp-server 2>&1 | grep -i error

# Verify API key
echo $ANTHROPIC_API_KEY

# Test with verbose logging
RUST_LOG=debug brainwires chat --mcp-server
```

### Agent Not Spawning

**Problem:** Agent spawn fails with error

**Solutions:**
1. Check task description is provided
2. Verify AI provider is configured
3. Check available system resources
4. Review logs for specific error

### JSON-RPC Errors

**Problem:** Invalid response format

**Check:**
1. Ensure proper JSON-RPC 2.0 format
2. Verify method names are correct
3. Check parameter types match schema
4. Review server logs

### Performance Issues

**Problem:** Server becomes slow or unresponsive

**Solutions:**
1. Stop idle agents with `agent_stop`
2. Reduce concurrent agent count
3. Check system resources (memory, CPU)
4. Review agent iteration counts

## Examples

### Example 1: Code Review

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "agent_spawn",
    "arguments": {
      "task": "Review src/auth/ for security issues and provide detailed report"
    }
  }
}
```

### Example 2: Documentation Generation

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "agent_spawn",
    "arguments": {
      "task": "Generate API documentation for all public functions in src/api/"
    }
  }
}
```

### Example 3: Test Suite

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "agent_spawn",
    "arguments": {
      "task": "Create comprehensive unit tests for UserManager class"
    }
  }
}
```

## Advanced Usage

### Custom MCP Client

Build your own MCP client in Python:

```python
import json
import subprocess

def call_mcp_tool(tool_name, arguments):
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    }

    proc = subprocess.Popen(
        ["brainwires", "chat", "--mcp-server"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True
    )

    proc.stdin.write(json.dumps(request) + "\n")
    proc.stdin.flush()

    response = proc.stdout.readline()
    return json.loads(response)

# Spawn an agent
result = call_mcp_tool("agent_spawn", {
    "task": "Analyze codebase complexity"
})
print(result)
```

### Integration with CI/CD

```yaml
# .github/workflows/agent-review.yml
name: AI Code Review

on: [pull_request]

jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install brainwires
        run: cargo install --path .

      - name: Run AI review
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"agent_spawn","arguments":{"task":"Review PR changes for issues"}}}' | \
            brainwires chat --mcp-server > review.json

      - name: Post review
        run: cat review.json
```

## Related Documentation

- [Testing Guide](MCP_SERVER_TESTS.md)
- [Tool Development](../CONTRIBUTING.md)
- [Architecture Overview](../README.md)
- [MCP Specification](https://modelcontextprotocol.io)

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/yourusername/brainwires-cli/issues
- MCP Discord: https://discord.gg/modelcontextprotocol
