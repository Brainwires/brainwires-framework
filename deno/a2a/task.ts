/**
 * Task lifecycle types: TaskState, TaskStatus, Task.
 *
 * `TaskState` uses SCREAMING_SNAKE_CASE per A2A v1.0.
 */

import type { Artifact, Message } from "./types.ts";

/**
 * Possible lifecycle states of a Task.
 * Serialized as SCREAMING_SNAKE_CASE strings.
 */
export type TaskState =
  | "TASK_STATE_UNSPECIFIED"
  | "TASK_STATE_SUBMITTED"
  | "TASK_STATE_WORKING"
  | "TASK_STATE_COMPLETED"
  | "TASK_STATE_FAILED"
  | "TASK_STATE_CANCELED"
  | "TASK_STATE_REJECTED"
  | "TASK_STATE_INPUT_REQUIRED"
  | "TASK_STATE_AUTH_REQUIRED";

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
}
