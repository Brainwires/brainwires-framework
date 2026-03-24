/**
 * JSON-RPC 2.0 envelope types and A2A method constants.
 *
 * These are separate from MCP JSON-RPC types since A2A has its own
 * method namespace and error semantics.
 */

import type { A2aError } from "./error.ts";

// ---------------------------------------------------------------------------
// Method constants (A2A v1.0 PascalCase names)
// ---------------------------------------------------------------------------

/** Send a message to an agent. */
export const METHOD_MESSAGE_SEND = "SendMessage";
/** Stream a message to an agent. */
export const METHOD_MESSAGE_STREAM = "SendStreamingMessage";
/** Get a task by ID. */
export const METHOD_TASKS_GET = "GetTask";
/** Cancel a task. */
export const METHOD_TASKS_CANCEL = "CancelTask";
/** Resubscribe to task updates. */
export const METHOD_TASKS_RESUBSCRIBE = "SubscribeToTask";
/** List tasks. */
export const METHOD_TASKS_LIST = "ListTasks";
/** Set push notification configuration. */
export const METHOD_PUSH_CONFIG_SET = "CreateTaskPushNotificationConfig";
/** Get push notification configuration. */
export const METHOD_PUSH_CONFIG_GET = "GetTaskPushNotificationConfig";
/** List push notification configurations. */
export const METHOD_PUSH_CONFIG_LIST = "ListTaskPushNotificationConfigs";
/** Delete push notification configuration. */
export const METHOD_PUSH_CONFIG_DELETE = "DeleteTaskPushNotificationConfig";
/** Get authenticated extended agent card. */
export const METHOD_EXTENDED_CARD = "GetExtendedAgentCard";

// ---------------------------------------------------------------------------
// Request ID
// ---------------------------------------------------------------------------

/** JSON-RPC request identifier (string or number). */
export type RequestId = string | number;

// ---------------------------------------------------------------------------
// JSON-RPC envelope types
// ---------------------------------------------------------------------------

/** JSON-RPC 2.0 request. */
export interface JsonRpcRequest {
  /** Protocol version (always "2.0"). */
  jsonrpc: "2.0";
  /** Method name. */
  method: string;
  /** Request parameters. */
  params?: unknown;
  /** Request identifier. */
  id: RequestId;
}

/** JSON-RPC 2.0 response. */
export interface JsonRpcResponse {
  /** Protocol version (always "2.0"). */
  jsonrpc: "2.0";
  /** Result on success. */
  result?: unknown;
  /** Error on failure. */
  error?: A2aError;
  /** Request identifier echoed back. */
  id: RequestId;
}

/** Create a success response. */
export function createJsonRpcSuccess(
  id: RequestId,
  result: unknown,
): JsonRpcResponse {
  return { jsonrpc: "2.0", result, id };
}

/** Create an error response. */
export function createJsonRpcError(
  id: RequestId,
  error: A2aError,
): JsonRpcResponse {
  return { jsonrpc: "2.0", error, id };
}
