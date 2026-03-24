/**
 * Streaming event types for A2A v1.0.
 *
 * `StreamResponse` is a wrapper with optional fields.
 * Exactly one field should be set per response.
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
  /** Index of the artifact within the task. */
  index?: number;
  /** If true, append to previously sent artifact with same ID. */
  append?: boolean;
  /** If true, this is the final chunk. */
  lastChunk?: boolean;
  /** Optional metadata. */
  metadata?: Record<string, unknown>;
}

/**
 * Wrapper response for streaming events.
 * Exactly one field should be set per response.
 */
export interface StreamResponse {
  /** A complete task snapshot. */
  task?: Task;
  /** A message from the agent. */
  message?: Message;
  /** A status update event. */
  statusUpdate?: TaskStatusUpdateEvent;
  /** An artifact update event. */
  artifactUpdate?: TaskArtifactUpdateEvent;
}

/**
 * Response for `message/send` -- wrapper with either a Task or a Message.
 * Exactly one field should be set.
 */
export interface SendMessageResponse {
  /** A complete task snapshot. */
  task?: Task;
  /** A message from the agent. */
  message?: Message;
}

/** Type guard: does the stream response contain a Task? */
export function isTaskResponse(r: StreamResponse): boolean {
  return r.task !== undefined;
}

/** Type guard: does the stream response contain a Message? */
export function isMessageResponse(r: StreamResponse): boolean {
  return r.message !== undefined;
}

/** Type guard: does the stream response contain a status update? */
export function isStatusUpdate(r: StreamResponse): boolean {
  return r.statusUpdate !== undefined;
}

/** Type guard: does the stream response contain an artifact update? */
export function isArtifactUpdate(r: StreamResponse): boolean {
  return r.artifactUpdate !== undefined;
}
