# @brainwires/mcp-server

MCP-compliant tool server framework. Extracted from `@brainwires/network` in
v0.11.0 to mirror Rust's standalone `brainwires-mcp-server` crate.

Contents:

- **McpServer** — event loop + middleware pipeline
- **McpHandler** — handler trait for MCP method dispatch
- **McpToolRegistry** — tool registration
- **MiddlewareChain** — composable middleware (Auth, Logging, RateLimit,
  ToolFilter)
- **StdioServerTransport** — stdio-based MCP transport
