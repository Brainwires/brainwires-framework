/**
 * Task lifecycle types: TaskState, TaskStatus, Task.
 *
 * `TaskState` uses kebab-case in JSON.
 */

import type { Artifact, Message } from "./types.ts";

/**
 * Possible lifecycle states of a Task.
 * Serialized as kebab-case strings.
 */
export type TaskState =
  | "unknown"
  | "submitted"
  | "working"
  | "completed"
  | "failed"
  | "canceled"
  | "rejected"
  | "input-required"
  | "auth-required";

/** Current status of a task. */
export interface TaskStatus {
  /** Current state. */
  state: TaskState;
  /** Optional message associated with the status. */
  message?: Message;
  /** ISO 8601 timestamp when the status was recorded. */
  timestamp?: string;
}

/** The core unit of action in A2A. */
export interface Task {
  /** Unique task identifier (UUID). */
  id: string;
  /** Context identifier for the conversation/session. */
  contextId?: string;
  /** Current task status. */
  status: TaskStatus;
  /** Output artifacts. */
  artifacts?: Artifact[];
  /** History of interactions. */
  history?: Message[];
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
  /** Discriminator field (always "task"). */
  kind: string;
}
