/**
 * Inter-agent communication hub and message types.
 *
 * Provides a broadcast-based messaging system for agent coordination,
 * including status updates, help requests, task results, and conflict
 * notifications. Uses an in-memory channel pattern (no tokio needed).
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Operation / Git operation enums
// ---------------------------------------------------------------------------

/** Types of operations that require coordination. */
export type OperationType =
  | "build"
  | "test"
  | "build_test"
  | "git_index"
  | "git_commit"
  | "git_push"
  | "git_pull"
  | "git_branch"
  | "file_write";

/** Git-specific operation types for finer-grained control. */
export type GitOperationType =
  | "read_only"
  | "staging"
  | "commit"
  | "remote_write"
  | "remote_merge"
  | "branch"
  | "destructive";

// ---------------------------------------------------------------------------
// Conflict types
// ---------------------------------------------------------------------------

/** Types of conflicts that can block operations. */
export type ConflictType =
  | { kind: "file_write_blocks_build"; path: string }
  | { kind: "build_blocks_file_write" }
  | { kind: "test_blocks_file_write" }
  | { kind: "git_blocks_file_write" }
  | { kind: "file_write_blocks_git"; path: string }
  | { kind: "build_blocks_git" };

/** Information about a conflict blocking an operation. */
export interface ConflictInfo {
  /** Type of conflict. */
  conflictType: ConflictType;
  /** Agent holding the conflicting resource. */
  holderAgent: string;
  /** Resource identifier (path or scope). */
  resource: string;
  /** How long the conflict has been active (seconds). */
  durationSecs: number;
  /** Current status of the blocking operation. */
  status: string;
}

// ---------------------------------------------------------------------------
// Agent message (discriminated union)
// ---------------------------------------------------------------------------

/** Types of messages agents can send to each other. */
export type AgentMessage =
  | { type: "task_request"; taskId: string; description: string; priority: number }
  | { type: "task_result"; taskId: string; success: boolean; result: string }
  | { type: "status_update"; agentId: string; status: string; details?: string }
  | { type: "help_request"; requestId: string; topic: string; details: string }
  | { type: "help_response"; requestId: string; response: string }
  | { type: "broadcast"; sender: string; message: string }
  | { type: "custom"; messageType: string; data: unknown }
  | { type: "agent_spawned"; agentId: string; taskId: string }
  | { type: "agent_progress"; agentId: string; progressPercent: number; message: string }
  | { type: "agent_completed"; agentId: string; taskId: string; summary: string }
  | { type: "lock_contention"; agentId: string; path: string; waitingFor: string }
  | { type: "approval_request"; requestId: string; agentId: string; operation: string; details: string }
  | { type: "approval_response"; requestId: string; approved: boolean; reason?: string }
  | { type: "operation_started"; agentId: string; operationType: OperationType; scope: string; estimatedDurationMs?: number; description: string }
  | { type: "operation_completed"; agentId: string; operationType: OperationType; scope: string; success: boolean; durationMs: number; summary: string }
  | { type: "lock_available"; operationType: OperationType; scope: string; releasedBy: string }
  | { type: "saga_started"; sagaId: string; agentId: string; description: string; totalSteps: number }
  | { type: "saga_step_completed"; sagaId: string; agentId: string; stepIndex: number; stepName: string; success: boolean }
  | { type: "saga_completed"; sagaId: string; agentId: string; success: boolean; compensated: boolean; summary: string }
  | { type: "saga_compensating"; sagaId: string; agentId: string; reason: string; stepsToCompensate: number }
  | { type: "task_announced"; taskId: string; announcer: string; description: string; bidDeadlineMs: number }
  | { type: "bid_submitted"; taskId: string; agentId: string; capabilityScore: number; currentLoad: number }
  | { type: "task_awarded"; taskId: string; winner: string; announcer: string }
  | { type: "task_accepted"; taskId: string; agentId: string }
  | { type: "task_declined"; taskId: string; agentId: string; reason: string }
  | { type: "version_conflict"; resourceId: string; agentId: string; expectedVersion: number; actualVersion: number }
  | { type: "conflict_resolution_applied"; resourceId: string; resolutionType: string; winningAgent?: string }
  | { type: "validation_failed"; agentId: string; operation: string; ruleName: string; message: string }
  | { type: "validation_warning"; agentId: string; operation: string; ruleName: string; message: string };

// ---------------------------------------------------------------------------
// Message envelope
// ---------------------------------------------------------------------------

/** Envelope containing message metadata. */
export interface MessageEnvelope {
  /** Sender agent ID. */
  from: string;
  /** Recipient agent ID. */
  to: string;
  /** The message payload. */
  message: AgentMessage;
  /** When the message was created (epoch ms). */
  timestamp: number;
}

function createEnvelope(
  from: string,
  to: string,
  message: AgentMessage,
): MessageEnvelope {
  return { from, to, message, timestamp: Date.now() };
}

// ---------------------------------------------------------------------------
// Agent channel (in-memory queue)
// ---------------------------------------------------------------------------

class AgentChannel {
  private queue: MessageEnvelope[] = [];
  private waiters: Array<(envelope: MessageEnvelope) => void> = [];

  send(envelope: MessageEnvelope): void {
    const waiter = this.waiters.shift();
    if (waiter) {
      waiter(envelope);
    } else {
      this.queue.push(envelope);
    }
  }

  /** Receive a message. Resolves immediately if one is queued, otherwise waits. */
  receive(): Promise<MessageEnvelope> {
    const queued = this.queue.shift();
    if (queued) return Promise.resolve(queued);
    return new Promise<MessageEnvelope>((resolve) => {
      this.waiters.push(resolve);
    });
  }

  /** Try to receive a message without blocking. */
  tryReceive(): MessageEnvelope | undefined {
    return this.queue.shift();
  }
}

// ---------------------------------------------------------------------------
// Communication hub
// ---------------------------------------------------------------------------

/** Communication hub for managing multiple agent channels. */
export class CommunicationHub {
  private channels = new Map<string, AgentChannel>();

  /** Register an agent with the hub. */
  registerAgent(agentId: string): void {
    if (this.channels.has(agentId)) {
      throw new Error(`Agent ${agentId} is already registered`);
    }
    this.channels.set(agentId, new AgentChannel());
  }

  /** Unregister an agent from the hub. */
  unregisterAgent(agentId: string): void {
    if (!this.channels.delete(agentId)) {
      throw new Error(`Agent ${agentId} is not registered`);
    }
  }

  /** Send a message from one agent to another. */
  sendMessage(from: string, to: string, message: AgentMessage): void {
    const channel = this.channels.get(to);
    if (!channel) {
      throw new Error(`Agent ${to} is not registered`);
    }
    channel.send(createEnvelope(from, to, message));
  }

  /** Broadcast a message to all agents. */
  broadcast(from: string, message: AgentMessage): void {
    for (const [agentId, channel] of this.channels) {
      channel.send(createEnvelope(from, agentId, message));
    }
  }

  /** Receive a message for a specific agent (waits until one is available). */
  receiveMessage(agentId: string): Promise<MessageEnvelope> | undefined {
    const channel = this.channels.get(agentId);
    return channel?.receive();
  }

  /** Try to receive a message without blocking. */
  tryReceiveMessage(agentId: string): MessageEnvelope | undefined {
    return this.channels.get(agentId)?.tryReceive();
  }

  /** Get the number of registered agents. */
  agentCount(): number {
    return this.channels.size;
  }

  /** Get list of registered agent IDs. */
  listAgents(): string[] {
    return [...this.channels.keys()];
  }

  /** Check if an agent is registered. */
  isRegistered(agentId: string): boolean {
    return this.channels.has(agentId);
  }
}
