/**
 * @module @brainwires/network
 *
 * Agent-to-agent networking layer for the Brainwires Agent Framework.
 * Provides identity, routing, discovery, peer table, agent management, remote
 * bridge, and client connectivity.
 *
 * In v0.11.0 the MCP server framework (McpServer, McpToolRegistry, middleware,
 * transport) moved to `@brainwires/mcp-server` to mirror Rust's standalone
 * `brainwires-mcp-server` crate. The transitional re-export below keeps the
 * old import paths working; remove it in 0.12.0.
 */

// MCP server framework (moved to @brainwires/mcp-server in v0.11.0).
// Includes AgentNetworkError + ErrorCode + transport + middleware.
export * from "@brainwires/mcp-server";

// =============================================================================
// Identity Layer
// =============================================================================

export {
  type AgentCard,
  type AgentIdentity,
  createAgentIdentity,
  createAgentIdentityWithId,
  defaultAgentCard,
  hasCapability,
  type ProtocolId,
  supportsProtocol,
} from "./identity.ts";

// =============================================================================
// Network Core Types
// =============================================================================

export {
  binaryPayload,
  broadcastEnvelope,
  directEnvelope,
  jsonPayload,
  type MessageEnvelope,
  type MessageTarget,
  type Payload,
  replyEnvelope,
  textPayload,
  topicEnvelope,
  withCorrelation,
  withTtl,
} from "./envelope.ts";

// =============================================================================
// Routing Layer
// =============================================================================

export {
  BroadcastRouter,
  ContentRouter,
  DirectRouter,
  type Router,
  type RoutingStrategy,
} from "./routing.ts";

export {
  displayTransportAddress,
  PeerTable,
  type TransportAddress,
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
  type AgentInfo,
  type AgentManager,
  type AgentResult,
  type SpawnConfig,
} from "./agent_manager.ts";

export { AgentToolRegistry } from "./agent_tools.ts";

// =============================================================================
// Client
// =============================================================================

export {
  type AgentConfig,
  AgentNetworkClient,
  AgentNetworkClientError,
} from "./client.ts";

// =============================================================================
// Remote Bridge
// =============================================================================

export {
  allSupportedCapabilities,
  assessConnectionQuality,
  // Command queue
  CommandQueue,
  defaultBridgeConfig,
  defaultProtocolAccept,
  defaultProtocolHello,
  defaultRetryPolicy,
  displayBridgeStatus,
  // Heartbeat & telemetry
  HeartbeatCollector,
  MIN_PROTOCOL_VERSION,
  NegotiatedProtocol,
  PRIORITY_ORDER,
  // Protocol
  PROTOCOL_VERSION,
  ProtocolMetrics,
  QueueEntry,
  QueueError,
  // Bridge
  RemoteBridge,
  // Manager
  RemoteBridgeManager,
  SUPPORTED_VERSIONS,
} from "./remote/mod.ts";

export type {
  AgentEvent,
  AgentEventType,
  AgentInfoProvider,
  BackendCommand,
  // Bridge types
  BridgeConfig,
  BridgeConfigProvider,
  BridgeState,
  CommandHandler,
  CommandPriority,
  CompressionAlgorithm,
  ConnectionMode,
  ConnectionQuality,
  // Heartbeat types
  HeartbeatData,
  MetricsSnapshot,
  PrioritizedCommand,
  ProtocolAccept,
  // Protocol types
  ProtocolCapability,
  ProtocolHello,
  QueueStats,
  RemoteAgentInfo,
  // Manager types
  RemoteBridgeConfig,
  RemoteBridgeStatus,
  RemoteMessage,
  RetryPolicy,
  StateChangeHandler,
  StreamChunkType,
} from "./remote/mod.ts";
