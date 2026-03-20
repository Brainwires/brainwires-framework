/**
 * Streaming event types for A2A.
 *
 * `StreamEvent` and `SendMessageResponse` are untagged unions.
 * Discrimination is done via the `kind` field on Task/Message,
 * or the presence of `taskId`+`status` vs `taskId`+`artifact`.
 */

import type { Task, TaskStatus } from "./task.ts";
import type { Artifact, Message } from "./types.ts";

/** Event notifying a change in task status. */
export interface TaskStatusUpdateEvent {
  /** Task identifier. */
  taskId: string;
  /** Context identifier. */
  contextId: string;
  /** New task status. */
  status: TaskStatus;
  /** Optional metadata. */
  metadata?: Record<string, unknown>;
}

/** Event notifying an artifact update. */
export interface TaskArtifactUpdateEvent {
  /** Task identifier. */
  taskId: string;
  /** Context identifier. */
  contextId: string;
  /** The artifact. */
  artifact: Artifact;
  /** If true, append to previously sent artifact with same ID. */
  append?: boolean;
  /** If true, this is the final chunk. */
  lastChunk?: boolean;
  /** Optional metadata. */
  metadata?: Record<string, unknown>;
}

/**
 * Union of all possible stream events (untagged).
 *
 * Discrimination strategy:
 * - Has `kind === "task"` -> Task
 * - Has `kind === "message"` -> Message
 * - Has `status` + `taskId` but no `kind` -> TaskStatusUpdateEvent
 * - Has `artifact` + `taskId` but no `kind` -> TaskArtifactUpdateEvent
 */
export type StreamEvent =
  | Task
  | Message
  | TaskStatusUpdateEvent
  | TaskArtifactUpdateEvent;

/**
 * Response for `message/send` -- either a Task or a Message (untagged).
 * Use the `kind` field to discriminate.
 */
export type SendMessageResponse = Task | Message;

/** Type guard: is the stream event a Task? */
export function isTask(event: StreamEvent): event is Task {
  return "kind" in event && (event as Task).kind === "task";
}

/** Type guard: is the stream event a Message? */
export function isMessage(event: StreamEvent): event is Message {
  return "kind" in event && (event as Message).kind === "message";
}

/** Type guard: is the stream event a TaskStatusUpdateEvent? */
export function isTaskStatusUpdate(
  event: StreamEvent,
): event is TaskStatusUpdateEvent {
  return "status" in event && "taskId" in event && !("kind" in event);
}

/** Type guard: is the stream event a TaskArtifactUpdateEvent? */
export function isTaskArtifactUpdate(
  event: StreamEvent,
): event is TaskArtifactUpdateEvent {
  return "artifact" in event && "taskId" in event && !("kind" in event);
}
