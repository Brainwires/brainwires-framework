/**
 * @module identity
 *
 * Agent identity, capability advertisement, and protocol identifiers.
 * Equivalent to Rust's `AgentIdentity`, `AgentCard`, `ProtocolId`.
 */

/** A protocol identifier (e.g. "mcp", "a2a", "ipc"). */
export type ProtocolId = string;

/**
 * Capability advertisement for an agent.
 * Equivalent to Rust `AgentCard`.
 */
export interface AgentCard {
  /** High-level capabilities (e.g. "code-review", "file-editing"). */
  capabilities: string[];
  /** Protocol identifiers this agent supports (e.g. "mcp", "a2a"). */
  supportedProtocols: ProtocolId[];
  /** Arbitrary key-value metadata (e.g. model name, version). */
  metadata: Record<string, unknown>;
  /** Network endpoint if directly reachable. */
  endpoint?: string;
  /** Maximum concurrent tasks. */
  maxConcurrentTasks?: number;
  /** Abstract compute capacity score. */
  computeCapacity?: number;
}

/** Create a default empty AgentCard. */
export function defaultAgentCard(): AgentCard {
  return {
    capabilities: [],
    supportedProtocols: [],
    metadata: {},
  };
}

/** Check whether an agent card supports a given protocol. */
export function supportsProtocol(card: AgentCard, protocol: string): boolean {
  return card.supportedProtocols.some(
    (p) => p.toLowerCase() === protocol.toLowerCase(),
  );
}

/** Check whether an agent card has a specific capability. */
export function hasCapability(card: AgentCard, capability: string): boolean {
  return card.capabilities.some(
    (c) => c.toLowerCase() === capability.toLowerCase(),
  );
}

/**
 * An agent's identity on the network.
 * Equivalent to Rust `AgentIdentity`.
 */
export interface AgentIdentity {
  /** Globally unique identifier for this agent. */
  id: string;
  /** Human-readable name (e.g. "code-review-agent"). */
  name: string;
  /** Capability advertisement. */
  agentCard: AgentCard;
}

/** Create a new AgentIdentity with a random UUID and empty card. */
export function createAgentIdentity(name: string): AgentIdentity {
  return {
    id: crypto.randomUUID(),
    name,
    agentCard: defaultAgentCard(),
  };
}

/** Create a new AgentIdentity with a specific UUID. */
export function createAgentIdentityWithId(
  id: string,
  name: string,
): AgentIdentity {
  return {
    id,
    name,
    agentCard: defaultAgentCard(),
  };
}
