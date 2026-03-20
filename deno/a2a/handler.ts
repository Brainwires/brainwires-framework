/**
 * A2aHandler interface -- the trait that agent implementations must satisfy.
 *
 * Implement this interface to define agent behavior. Server infrastructure
 * will route requests from JSON-RPC and REST to these methods.
 */

import type { AgentCard } from "./agent_card.ts";
import type { A2aError } from "./error.ts";
import type {
  CancelTaskRequest,
  DeleteTaskPushNotificationConfigRequest,
  GetExtendedAgentCardRequest,
  GetTaskPushNotificationConfigRequest,
  GetTaskRequest,
  ListTaskPushNotificationConfigsRequest,
  ListTaskPushNotificationConfigsResponse,
  ListTasksRequest,
  ListTasksResponse,
  SendMessageRequest,
  SubscribeToTaskRequest,
} from "./params.ts";
import type { TaskPushNotificationConfig } from "./push_notification.ts";
import type { SendMessageResponse, StreamEvent } from "./streaming.ts";
import type { Task } from "./task.ts";

/**
 * Core handler interface for A2A agents.
 *
 * Implement this interface to define agent behavior. Required methods
 * must be implemented; optional methods have default implementations
 * that return "not supported" errors.
 */
export interface A2aHandler {
  /** Return the agent card for discovery. */
  agentCard(): AgentCard;

  /** Handle a `message/send` request. */
  onSendMessage(req: SendMessageRequest): Promise<SendMessageResponse>;

  /** Handle a `message/stream` request (server-streaming). */
  onSendStreamingMessage(
    req: SendMessageRequest,
  ): Promise<AsyncIterable<StreamEvent>>;

  /** Handle a `tasks/get` request. */
  onGetTask(req: GetTaskRequest): Promise<Task>;

  /** Handle a `tasks/list` request. */
  onListTasks(req: ListTasksRequest): Promise<ListTasksResponse>;

  /** Handle a `tasks/cancel` request. */
  onCancelTask(req: CancelTaskRequest): Promise<Task>;

  /** Handle a `tasks/resubscribe` request (server-streaming). */
  onSubscribeToTask(
    req: SubscribeToTaskRequest,
  ): Promise<AsyncIterable<StreamEvent>>;

  /** Create a push notification config. Optional -- default returns unsupported. */
  onCreatePushConfig?(
    config: TaskPushNotificationConfig,
  ): Promise<TaskPushNotificationConfig>;

  /** Get a push notification config. Optional -- default returns unsupported. */
  onGetPushConfig?(
    req: GetTaskPushNotificationConfigRequest,
  ): Promise<TaskPushNotificationConfig>;

  /** List push notification configs. Optional -- default returns unsupported. */
  onListPushConfigs?(
    req: ListTaskPushNotificationConfigsRequest,
  ): Promise<ListTaskPushNotificationConfigsResponse>;

  /** Delete a push notification config. Optional -- default returns unsupported. */
  onDeletePushConfig?(
    req: DeleteTaskPushNotificationConfigRequest,
  ): Promise<void>;

  /** Get the authenticated extended agent card. Optional -- default returns not configured. */
  onGetExtendedAgentCard?(
    req: GetExtendedAgentCardRequest,
  ): Promise<AgentCard>;
}
