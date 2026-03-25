/**
 * @module remote
 *
 * Remote bridge subsystem for connecting local agents to a cloud relay.
 * Provides WebSocket-based communication with priority command queuing,
 * heartbeat telemetry, and protocol negotiation.
 *
 * Equivalent to Rust's `brainwires-agent-network::remote` module.
 */

// Protocol types
export {
  PROTOCOL_VERSION,
  MIN_PROTOCOL_VERSION,
  SUPPORTED_VERSIONS,
  allSupportedCapabilities,
  PRIORITY_ORDER,
  defaultRetryPolicy,
  defaultProtocolHello,
  defaultProtocolAccept,
  NegotiatedProtocol,
} from "./protocol.ts";

export type {
  ProtocolCapability,
  CommandPriority,
  RetryPolicy,
  PrioritizedCommand,
  ProtocolHello,
  ProtocolAccept,
  RemoteMessage,
  RemoteMessage_Register,
  RemoteMessage_Heartbeat,
  RemoteMessage_CommandResult,
  RemoteMessage_AgentEvent,
  RemoteMessage_AgentStream,
  RemoteMessage_Pong,
  RemoteMessage_AttachmentReceived,
  BackendCommand,
  BackendCommand_Authenticated,
  BackendCommand_SendInput,
  BackendCommand_SlashCommand,
  BackendCommand_CancelOperation,
  BackendCommand_Subscribe,
  BackendCommand_Unsubscribe,
  BackendCommand_SpawnAgent,
  BackendCommand_RequestSync,
  BackendCommand_Ping,
  BackendCommand_Disconnect,
  BackendCommand_AuthenticationFailed,
  BackendCommand_AttachmentUpload,
  BackendCommand_AttachmentChunk,
  BackendCommand_AttachmentComplete,
  CompressionAlgorithm,
  RemoteAgentInfo,
  AgentEventType,
  StreamChunkType,
} from "./protocol.ts";

// Command queue
export { CommandQueue, QueueEntry, QueueError } from "./command_queue.ts";
export type { QueueStats } from "./command_queue.ts";

// Heartbeat & telemetry
export {
  HeartbeatCollector,
  ProtocolMetrics,
  assessConnectionQuality,
} from "./heartbeat.ts";
export type {
  HeartbeatData,
  AgentEvent,
  AgentInfoProvider,
  MetricsSnapshot,
  ConnectionQuality,
} from "./heartbeat.ts";

// Bridge
export { RemoteBridge, defaultBridgeConfig } from "./bridge.ts";
export type {
  BridgeConfig,
  BridgeState,
  ConnectionMode,
  CommandHandler,
  StateChangeHandler,
} from "./bridge.ts";

// Manager
export {
  RemoteBridgeManager,
  displayBridgeStatus,
} from "./manager.ts";
export type {
  RemoteBridgeConfig,
  BridgeConfigProvider,
  RemoteBridgeStatus,
} from "./manager.ts";
