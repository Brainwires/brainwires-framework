# MCP Server Examples

This file contains example configurations for popular MCP servers that can be used with Brainwires CLI.

## Adding MCP Servers

Use the `brainwires mcp add` command to add servers:

```bash
brainwires mcp add <name> <command> [args...]
```

## Example Servers

### 1. Filesystem Server

Provides file system operations.

```bash
brainwires mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /path/to/directory
```

### 2. GitHub Server

Access GitHub repositories and issues.

```bash
# Set GITHUB_TOKEN environment variable first
export GITHUB_TOKEN=your_github_token

brainwires mcp add github npx -y @modelcontextprotocol/server-github
```

### 3. PostgreSQL Server

Query PostgreSQL databases.

```bash
# Set DATABASE_URL environment variable
export DATABASE_URL=postgresql://user:pass@localhost/dbname

brainwires mcp add postgres npx -y @modelcontextprotocol/server-postgres
```

### 4. Slack Server

Interact with Slack workspaces.

```bash
# Set SLACK_BOT_TOKEN and SLACK_TEAM_ID
export SLACK_BOT_TOKEN=xoxb-your-token
export SLACK_TEAM_ID=T123456

brainwires mcp add slack npx -y @modelcontextprotocol/server-slack
```

### 5. Google Drive Server

Access Google Drive files.

```bash
brainwires mcp add gdrive npx -y @modelcontextprotocol/server-gdrive
```

### 6. Memory Server

Simple key-value store for agent memory.

```bash
brainwires mcp add memory npx -y @modelcontextprotocol/server-memory
```

### 7. Brave Search Server

Web search via Brave Search API.

```bash
# Set BRAVE_API_KEY
export BRAVE_API_KEY=your_api_key

brainwires mcp add brave npx -y @modelcontextprotocol/server-brave-search
```

### 8. Git Server

Git repository operations.

```bash
brainwires mcp add git npx -y @modelcontextprotocol/server-git /path/to/repo
```

## Usage

After adding servers, connect to them:

```bash
# Connect to a server
brainwires mcp connect filesystem

# List available tools
brainwires mcp tools filesystem

# List available resources
brainwires mcp resources filesystem

# Use in chat
brainwires chat
> "Use the filesystem server to list files in the current directory"
```

## Configuration File

Servers are stored in `~/.brainwires/mcp-config.json`:

```json
{
  "servers": [
    {
      "name": "filesystem",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/directory"]
    }
  ]
}
```

## Custom MCP Servers

You can create your own MCP servers. Here's an example configuration:

```bash
# Python MCP server
brainwires mcp add custom python -m my_mcp_server

# Node.js MCP server
brainwires mcp add custom node /path/to/server.js

# Binary MCP server
brainwires mcp add custom /path/to/binary --arg1 --arg2
```

## Troubleshooting

### Server Won't Connect

1. Check the server command is correct:
   ```bash
   # Test manually
   npx -y @modelcontextprotocol/server-filesystem /tmp
   ```

2. Check environment variables are set

3. View server stderr (inherited by default)

### Tools Not Showing

1. Ensure server is connected:
   ```bash
   brainwires mcp list
   ```

2. Reconnect if needed:
   ```bash
   brainwires mcp disconnect server_name
   brainwires mcp connect server_name
   ```

### Permission Issues

MCP tools don't require approval by default. If you want stricter control, use the `--permission-mode` flag when running commands.

## Learn More

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Official MCP Servers](https://github.com/modelcontextprotocol/servers)
- [Brainwires Documentation](https://docs.brainwires.dev)
