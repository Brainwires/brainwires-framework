/**
 * JSON-RPC 2.0 envelope types and A2A method constants.
 *
 * These are separate from MCP JSON-RPC types since A2A has its own
 * method namespace and error semantics.
 */

import type { A2aError } from "./error.ts";

// ---------------------------------------------------------------------------
// Method constants
// ---------------------------------------------------------------------------

/** Send a message to an agent. */
export const METHOD_MESSAGE_SEND = "message/send";
/** Stream a message to an agent. */
export const METHOD_MESSAGE_STREAM = "message/stream";
/** Get a task by ID. */
export const METHOD_TASKS_GET = "tasks/get";
/** Cancel a task. */
export const METHOD_TASKS_CANCEL = "tasks/cancel";
/** Resubscribe to task updates. */
export const METHOD_TASKS_RESUBSCRIBE = "tasks/resubscribe";
/** List tasks. */
export const METHOD_TASKS_LIST = "tasks/list";
/** Set push notification configuration. */
export const METHOD_PUSH_CONFIG_SET = "tasks/pushNotificationConfig/set";
/** Get push notification configuration. */
export const METHOD_PUSH_CONFIG_GET = "tasks/pushNotificationConfig/get";
/** List push notification configurations. */
export const METHOD_PUSH_CONFIG_LIST = "tasks/pushNotificationConfig/list";
/** Delete push notification configuration. */
export const METHOD_PUSH_CONFIG_DELETE = "tasks/pushNotificationConfig/delete";
/** Get authenticated extended agent card. */
export const METHOD_EXTENDED_CARD = "agent/authenticatedExtendedCard";

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
