/**
 * @module agent_manager
 *
 * Agent lifecycle management interface and supporting types.
 * Equivalent to Rust's `AgentManager`, `SpawnConfig`, `AgentInfo`, `AgentResult`.
 */

/**
 * Configuration for spawning a new agent.
 * Equivalent to Rust `SpawnConfig`.
 */
export interface SpawnConfig {
  /** Description of the task for the agent to execute. */
  description: string;
  /** Optional working directory for file operations. */
  workingDirectory?: string;
  /** Optional maximum number of iterations (default: 100). */
  maxIterations?: number;
  /** Enable automatic validation checks before completion. */
  enableValidation?: boolean;
  /** Build type for validation (e.g. "npm", "cargo", "typescript"). */
  buildType?: string;
  /** Opaque blob for implementation-specific config. */
  extra?: Record<string, unknown>;
}

/**
 * Information about a running or completed agent.
 * Equivalent to Rust `AgentInfo`.
 */
export interface AgentInfo {
  /** Unique agent identifier. */
  agentId: string;
  /** Current agent status (e.g. "running", "completed", "failed"). */
  status: string;
  /** Description of the task the agent is working on. */
  taskDescription: string;
  /** Number of iterations completed so far. */
  iterations: number;
}

/**
 * Result from a completed agent.
 * Equivalent to Rust `AgentResult`.
 */
export interface AgentResult {
  /** Unique agent identifier. */
  agentId: string;
  /** Whether the agent completed successfully. */
  success: boolean;
  /** Human-readable summary of what was accomplished. */
  summary: string;
  /** Total number of iterations used. */
  iterations: number;
}

/**
 * Interface for agent lifecycle management.
 * Equivalent to Rust `AgentManager` trait.
 */
export interface AgentManager {
  /** Spawn a new agent and return its ID. */
  spawnAgent(config: SpawnConfig): Promise<string>;

  /** List all currently active agents. */
  listAgents(): Promise<AgentInfo[]>;

  /** Get the current status of a specific agent. */
  agentStatus(agentId: string): Promise<AgentInfo>;

  /** Stop a running agent. */
  stopAgent(agentId: string): Promise<void>;

  /**
   * Wait for an agent to complete and return its result.
   * If timeoutSecs is provided, throws if the agent hasn't completed in time.
   */
  awaitAgent(agentId: string, timeoutSecs?: number): Promise<AgentResult>;

  /** Return pool-level statistics as a JSON value. */
  poolStats(): Promise<Record<string, unknown>>;

  /** Return all currently held file locks as a JSON value. */
  fileLocks(): Promise<Record<string, unknown>>;
}
