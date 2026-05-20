/**
 * @module @brainwires/mcp-server
 *
 * MCP-compliant tool server framework: `McpServer`, `McpHandler`,
 * `McpToolRegistry`, `MiddlewareChain` (Auth, Logging, RateLimit,
 * ToolFilter), and stdio transport.
 *
 * Extracted from `@brainwires/network` in v0.11.0 to mirror Rust's standalone
 * `brainwires-mcp-server` crate. Consumers building a pure MCP server no
 * longer need to pull in the A2A peer-table / discovery / routing surface
 * of `@brainwires/network`.
 */

export { AgentNetworkError, ErrorCode } from "./error.ts";

export {
  type McpToolDef,
  McpToolRegistry,
  type ToolHandler,
} from "./registry.ts";

export { type McpHandler } from "./handler.ts";

export { McpServer, type RequestContext } from "./server.ts";

// Middleware
export * from "./middleware/mod.ts";

// Transport
export * from "./transport/mod.ts";
