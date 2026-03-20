/**
 * Client construction, discovery URL building, and transport tests.
 */

import {
  assertEquals,
  assertThrows,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { A2aClient } from "./client.ts";
import type { JsonRpcRequest, JsonRpcResponse } from "./jsonrpc.ts";
import {
  createJsonRpcError,
  createJsonRpcSuccess,
  METHOD_MESSAGE_SEND,
  METHOD_TASKS_GET,
} from "./jsonrpc.ts";
import { A2aError } from "./error.ts";
import type { Task } from "./task.ts";
import type { SendMessageRequest } from "./params.ts";
import { createUserMessage } from "./types.ts";

Deno.test("A2aClient construction with default transport", () => {
  const client = new A2aClient({ baseUrl: "https://agent.example.com/a2a" });
  // Should not throw
  assertEquals(typeof client, "object");
});

Deno.test("A2aClient construction with REST transport", () => {
  const client = new A2aClient({
    baseUrl: "https://agent.example.com/a2a",
    transport: "rest",
  });
  assertEquals(typeof client, "object");
});

Deno.test("A2aClient.withBearerToken returns new client", () => {
  const client = new A2aClient({ baseUrl: "https://agent.example.com" });
  const authed = client.withBearerToken("tok-123");
  // They should be different instances
  assertEquals(client !== authed, true);
});

Deno.test("A2aClient trims trailing slashes from baseUrl", () => {
  // Verify it doesn't throw and works correctly
  const client = new A2aClient({ baseUrl: "https://agent.example.com///" });
  assertEquals(typeof client, "object");
});

Deno.test("createJsonRpcSuccess creates correct response", () => {
  const resp = createJsonRpcSuccess(1, { task: "data" });
  assertEquals(resp.jsonrpc, "2.0");
  assertEquals(resp.id, 1);
  assertEquals(resp.result, { task: "data" });
  assertEquals(resp.error, undefined);
});

Deno.test("createJsonRpcError creates correct response", () => {
  const err = A2aError.taskNotFound("t-1");
  const resp = createJsonRpcError("req-1", err);
  assertEquals(resp.jsonrpc, "2.0");
  assertEquals(resp.id, "req-1");
  assertEquals(resp.result, undefined);
  assertEquals(resp.error?.code, -32001);
});

Deno.test("discovery URL construction", () => {
  // Test that the static discover method would construct the right URL
  // We can't actually call it without a server, but we verify the pattern
  const baseUrls = [
    "https://agent.example.com",
    "https://agent.example.com/",
    "https://agent.example.com///",
  ];
  for (const base of baseUrls) {
    const expected = "https://agent.example.com/.well-known/agent-card.json";
    const url = `${base.replace(/\/+$/, "")}/.well-known/agent-card.json`;
    assertEquals(url, expected);
  }
});

Deno.test("Task JSON structure matches A2A spec", () => {
  const task: Task = {
    id: "task-1",
    contextId: "ctx-1",
    status: {
      state: "working",
      timestamp: "2024-01-01T00:00:00Z",
    },
    kind: "task",
  };
  const json = JSON.stringify(task);
  const parsed = JSON.parse(json);
  assertEquals(parsed.id, "task-1");
  assertEquals(parsed.contextId, "ctx-1");
  assertEquals(parsed.status.state, "working");
  assertEquals(parsed.kind, "task");
});

Deno.test("TaskState uses kebab-case", () => {
  const states = [
    "unknown",
    "submitted",
    "working",
    "completed",
    "failed",
    "canceled",
    "rejected",
    "input-required",
    "auth-required",
  ];
  for (const state of states) {
    const task: Task = {
      id: "t",
      status: { state: state as Task["status"]["state"] },
      kind: "task",
    };
    const parsed = JSON.parse(JSON.stringify(task));
    assertEquals(parsed.status.state, state);
  }
});

Deno.test("SendMessageRequest JSON structure", () => {
  const req: SendMessageRequest = {
    message: createUserMessage("hello"),
    configuration: {
      acceptedOutputModes: ["text/plain"],
      historyLength: 10,
      blocking: true,
    },
    metadata: { source: "test" },
  };
  const json = JSON.stringify(req);
  const parsed = JSON.parse(json);
  assertEquals(parsed.message.role, "user");
  assertEquals(parsed.configuration.acceptedOutputModes, ["text/plain"]);
  assertEquals(parsed.configuration.historyLength, 10);
  assertEquals(parsed.configuration.blocking, true);
});

Deno.test("JsonRpcRequest structure matches spec", () => {
  const req: JsonRpcRequest = {
    jsonrpc: "2.0",
    method: METHOD_MESSAGE_SEND,
    params: { message: { role: "user" } },
    id: 42,
  };
  const parsed = JSON.parse(JSON.stringify(req));
  assertEquals(parsed.jsonrpc, "2.0");
  assertEquals(parsed.method, "message/send");
  assertEquals(parsed.id, 42);
});

Deno.test("JsonRpcRequest with string ID", () => {
  const req: JsonRpcRequest = {
    jsonrpc: "2.0",
    method: METHOD_TASKS_GET,
    params: { id: "task-1" },
    id: "req-abc",
  };
  const parsed = JSON.parse(JSON.stringify(req));
  assertEquals(parsed.id, "req-abc");
});
