/**
 * Typed request/response parameter types for all A2A methods.
 */

import type { AgentCard } from "./agent_card.ts";
import type { TaskPushNotificationConfig } from "./push_notification.ts";
import type { Task, TaskState } from "./task.ts";
import type { Message } from "./types.ts";

/** Configuration for a send-message request. */
export interface SendMessageConfiguration {
  /** Accepted output media types. */
  acceptedOutputModes?: string[];
  /** Push notification configuration. */
  taskPushNotificationConfig?: TaskPushNotificationConfig;
  /** Max number of history messages to return. */
  historyLength?: number;
  /** If true, return immediately without waiting for terminal state. */
  returnImmediately?: boolean;
}

/** Request parameters for `message/send` and `message/stream`. */
export interface SendMessageRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** The message to send. */
  message: Message;
  /** Request configuration. */
  configuration?: SendMessageConfiguration;
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
}

/** Request parameters for `tasks/get`. */
export interface GetTaskRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Task identifier. */
  id: string;
  /** Max number of history messages to return. */
  historyLength?: number;
}

/** Request parameters for `tasks/list`. */
export interface ListTasksRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Filter by context ID. */
  contextId?: string;
  /** Filter by task state. */
  status?: TaskState;
  /** Maximum number of tasks to return. */
  pageSize?: number;
  /** Pagination token. */
  pageToken?: string;
  /** Max history messages per task. */
  historyLength?: number;
  /** Filter tasks with status updated after this ISO 8601 timestamp. */
  statusTimestampAfter?: string;
  /** Whether to include artifacts. */
  includeArtifacts?: boolean;
}

/** Response for `tasks/list`. */
export interface ListTasksResponse {
  /** Matching tasks. */
  tasks: Task[];
  /** Pagination token for next page. */
  nextPageToken: string;
  /** Page size used. */
  pageSize: number;
  /** Total number of matching tasks. */
  totalSize: number;
}

/** Request parameters for `tasks/cancel`. */
export interface CancelTaskRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Task identifier. */
  id: string;
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
}

/** Request parameters for `tasks/resubscribe`. */
export interface SubscribeToTaskRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Task identifier. */
  id: string;
}

/** Request for `tasks/pushNotificationConfig/get`. */
export interface GetTaskPushNotificationConfigRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Parent task identifier. */
  taskId: string;
  /** Configuration identifier. */
  configId: string;
}

/** Request for `tasks/pushNotificationConfig/delete`. */
export interface DeleteTaskPushNotificationConfigRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Parent task identifier. */
  taskId: string;
  /** Configuration identifier. */
  configId: string;
}

/** Request for `tasks/pushNotificationConfig/list`. */
export interface ListTaskPushNotificationConfigsRequest {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Parent task identifier. */
  taskId: string;
  /** Maximum configs to return. */
  pageSize?: number;
  /** Pagination token. */
  pageToken?: string;
}

/** Response for listing push notification configs. */
export interface ListTaskPushNotificationConfigsResponse {
  /** The configs. */
  configs: TaskPushNotificationConfig[];
  /** Pagination token for next page. */
  nextPageToken?: string;
}

/** Request for `agent/authenticatedExtendedCard`. */
export interface GetExtendedAgentCardRequest {
  /** Optional tenant identifier. */
  tenant?: string;
}

/** Response for the extended agent card. */
export type GetExtendedAgentCardResponse = AgentCard;
