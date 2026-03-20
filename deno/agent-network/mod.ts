/**
 * @module @brainwires/agent-network
 *
 * Agent networking layer for the Brainwires Agent Framework.
 * Provides an MCP server framework, middleware pipeline, agent communication,
 * routing, discovery, and client connectivity.
 *
 * Equivalent to Rust's `brainwires-agent-network` crate.
 */

// =============================================================================
// MCP Server Framework
// =============================================================================

export { McpServer, RequestContext, type ClientInfo } from "./server.ts";
export type { McpHandler } from "./handler.ts";
export { McpToolRegistry, type McpToolDef, type ToolHandler } from "./registry.ts";

// Error types
export { AgentNetworkError, ErrorCode } from "./error.ts";

// Transport
export type { ServerTransport } from "./transport/mod.ts";
export { StdioServerTransport } from "./transport/mod.ts";

// Middleware
export {
  MiddlewareChain,
  type Middleware,
  type MiddlewareResult,
  middlewareContinue,
  middlewareReject,
} from "./middleware/mod.ts";
export { AuthMiddleware } from "./middleware/auth.ts";
export { LoggingMiddleware } from "./middleware/logging.ts";
export { RateLimitMiddleware } from "./middleware/rate_limit.ts";
export { ToolFilterMiddleware, type FilterMode } from "./middleware/tool_filter.ts";

// =============================================================================
// Identity Layer
// =============================================================================

export {
  type AgentCard,
  type AgentIdentity,
  type ProtocolId,
  createAgentIdentity,
  createAgentIdentityWithId,
  defaultAgentCard,
  hasCapability,
  supportsProtocol,
} from "./identity.ts";

// =============================================================================
// Network Core Types
// =============================================================================

export {
  type MessageEnvelope,
  type MessageTarget,
  type Payload,
  directEnvelope,
  broadcastEnvelope,
  topicEnvelope,
  replyEnvelope,
  withTtl,
  withCorrelation,
  textPayload,
  jsonPayload,
  binaryPayload,
} from "./envelope.ts";

// =============================================================================
// Routing Layer
// =============================================================================

export {
  type Router,
  type RoutingStrategy,
  DirectRouter,
  BroadcastRouter,
  ContentRouter,
} from "./routing.ts";

export {
  PeerTable,
  type TransportAddress,
  displayTransportAddress,
} from "./peer_table.ts";

// =============================================================================
// Discovery Layer
// =============================================================================

export {
  type Discovery,
  type DiscoveryProtocol,
  ManualDiscovery,
} from "./discovery.ts";

// =============================================================================
// Agent Management
// =============================================================================

export {
  type AgentManager,
  type SpawnConfig,
  type AgentInfo,
  type AgentResult,
} from "./agent_manager.ts";

export { AgentToolRegistry } from "./agent_tools.ts";

// =============================================================================
// Client
// =============================================================================

export {
  AgentNetworkClient,
  AgentNetworkClientError,
  type AgentConfig,
} from "./client.ts";
