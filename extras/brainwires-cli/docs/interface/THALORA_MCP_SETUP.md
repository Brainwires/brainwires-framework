# Thalora Browser MCP Server Setup

## Overview

Thalora is a headless browser MCP server that provides web scraping and search capabilities to brainwires-cli. It's already configured and ready to use!

## What's Configured

### MCP Server: `browser`
- **Binary**: `/home/nightness/dev/brainwires-studio/rust/thalora-web-browser/target/release/thalora`
- **Mode**: `minimal` (basic web scraping tools)
- **Config**: `~/.brainwires/mcp-config.json`

### Available Tools (Minimal Mode)

#### 1. `scrape`
Unified web scraping tool with multiple extraction methods:

**Parameters**:
- `url` (required) - URL to scrape
- `wait_for_js` (optional) - Wait for JavaScript execution (default: false)
- `extract_basic` (optional) - Extract links, images, metadata (default: true)
- `extract_readable` (optional) - Extract clean readable content (default: false)
- `extract_structured` (optional) - Extract tables, lists, code blocks (default: false)
- `selectors` (optional) - Custom CSS selectors: `{"title": "h1", "price": ".price"}`

**Example usage**:
```json
{
  "url": "https://example.com",
  "extract_readable": true,
  "format": "markdown"
}
```

#### 2. `web_search`
Search the web using various search engines:

**Parameters**:
- `query` (required) - Search query
- `num_results` (optional) - Number of results (default: 10, max: 20)
- `search_engine` (optional) - Engine to use: duckduckgo, bing, google, startpage (default: duckduckgo)
- `region` (optional) - Search region/country code (e.g., 'us', 'uk')

**Example usage**:
```json
{
  "query": "rust async programming",
  "num_results": 5,
  "search_engine": "duckduckgo"
}
```

## Usage

### List Configured Servers
```bash
brainwires mcp list
```

### Connect to Browser (for testing)
```bash
brainwires mcp connect browser
```

### In Chat Sessions
When using brainwires chat, the AI can automatically use these tools:
- Tools are prefixed as: `mcp_browser_scrape`, `mcp_browser_web_search`
- The MCP server starts automatically when needed
- Connection is maintained for the duration of the chat session

## Full Mode (Advanced)

To enable all browser features (sessions, CDP, memory), change the config mode:

Edit `~/.brainwires/mcp-config.json`:
```json
{
  "servers": [
    {
      "name": "browser",
      "command": "/home/nightness/dev/brainwires-studio/rust/thalora-web-browser/target/release/thalora",
      "args": ["server", "--mcp-mode", "full"],
      "env": {
        "THALORA_SILENT": "false",
        "THALORA_ENABLE_AI_MEMORY": "1"
      }
    }
  ]
}
```

Full mode adds:
- Browser session management
- Chrome DevTools Protocol (CDP)
- AI memory/persistent storage
- Multi-window workflows
- Advanced form automation

## Troubleshooting

### Connection Issues
If connection fails, check:
1. Binary exists: `ls -lh ~/dev/brainwires-studio/rust/thalora-web-browser/target/release/thalora`
2. Rebuild if needed: `cd ~/dev/brainwires-studio/rust/thalora-web-browser && cargo build --release`

### Rebuilding Thalora
```bash
cd ~/dev/brainwires-studio/rust/thalora-web-browser
cargo build --release
```

### Viewing MCP Server Logs
The server logs to stderr. To see detailed output:
```bash
# Remove THALORA_SILENT from env in config
# Or run manually:
~/dev/brainwires-studio/rust/thalora-web-browser/target/release/thalora server --mcp-mode minimal
```

## Architecture

### How It Works
1. brainwires-cli reads `~/.brainwires/mcp-config.json`
2. When AI needs browser tools, it spawns the thalora process
3. Communication happens via JSON-RPC over STDIO
4. Tools are automatically registered with `mcp_browser_` prefix
5. Results are returned to the AI for processing

### Integration Flow
```
User Chat Request
    ↓
AI decides to use web tool
    ↓
brainwires-cli spawns thalora
    ↓
thalora executes scraping/search
    ↓
Results returned via JSON-RPC
    ↓
AI processes and responds to user
```

## Performance

- **Startup**: ~50ms (minimal mode)
- **Scrape**: 100-500ms per page (depends on content)
- **Search**: 200-1000ms (depends on engine and results)
- **Binary Size**: 27MB (statically linked)

## Security

- SSRF protection enabled
- No external credentials required for basic mode
- Runs in isolated process
- Automatic cleanup on disconnect
