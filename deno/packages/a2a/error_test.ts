/**
 * Error factory methods and codes tests (v1.0).
 */

import {
  assertEquals,
  assertInstanceOf,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  A2aError,
  CONTENT_TYPE_NOT_SUPPORTED,
  EXTENDED_CARD_NOT_CONFIGURED,
  EXTENSION_SUPPORT_REQUIRED,
  INTERNAL_ERROR,
  INVALID_PARAMS,
  INVALID_REQUEST,
  JSON_PARSE_ERROR,
  METHOD_NOT_FOUND,
  PUSH_NOT_SUPPORTED,
  TASK_NOT_CANCELABLE,
  TASK_NOT_FOUND,
  UNSUPPORTED_OPERATION,
  VERSION_NOT_SUPPORTED,
} from "./error.ts";

Deno.test("A2aError constructor", () => {
  const err = new A2aError(-32000, "test error");
  assertInstanceOf(err, Error);
  assertInstanceOf(err, A2aError);
  assertEquals(err.code, -32000);
  assertEquals(err.message, "test error");
  assertEquals(err.data, undefined);
  assertEquals(err.name, "A2aError");
});

Deno.test("A2aError.withData chains", () => {
  const err = new A2aError(-32000, "test").withData({ detail: "extra" });
  assertEquals(err.code, -32000);
  assertEquals((err.data as Record<string, string>).detail, "extra");
});

Deno.test("A2aError.toJSON serializes correctly", () => {
  const err = new A2aError(-32000, "test", { x: 1 });
  const json = err.toJSON();
  assertEquals(json.code, -32000);
  assertEquals(json.message, "test");
  assertEquals((json.data as Record<string, number>).x, 1);
});

Deno.test("A2aError.toJSON omits data when undefined", () => {
  const err = new A2aError(-32000, "test");
  const json = err.toJSON();
  assertEquals("data" in json, false);
});

Deno.test("A2aError.fromJSON creates instance", () => {
  const err = A2aError.fromJSON({ code: -32001, message: "not found" });
  assertInstanceOf(err, A2aError);
  assertEquals(err.code, -32001);
  assertEquals(err.message, "not found");
});

Deno.test("A2aError.fromJSON with data", () => {
  const err = A2aError.fromJSON({
    code: -32600,
    message: "bad",
    data: [1, 2],
  });
  assertEquals(err.data, [1, 2]);
});

Deno.test("A2aError round-trips through JSON", () => {
  const original = new A2aError(TASK_NOT_FOUND, "Task not found: abc", {
    taskId: "abc",
  });
  const json = JSON.stringify(original.toJSON());
  const restored = A2aError.fromJSON(JSON.parse(json));
  assertEquals(restored.code, original.code);
  assertEquals(restored.message, original.message);
  assertEquals(
    (restored.data as Record<string, string>).taskId,
    "abc",
  );
});

// Factory methods

Deno.test("taskNotFound", () => {
  const err = A2aError.taskNotFound("task-123");
  assertEquals(err.code, TASK_NOT_FOUND);
  assertEquals(err.message, "Task not found: task-123");
});

Deno.test("taskNotCancelable", () => {
  const err = A2aError.taskNotCancelable("task-456");
  assertEquals(err.code, TASK_NOT_CANCELABLE);
  assertEquals(err.message, "Task cannot be canceled: task-456");
});

Deno.test("pushNotSupported", () => {
  const err = A2aError.pushNotSupported();
  assertEquals(err.code, PUSH_NOT_SUPPORTED);
});

Deno.test("unsupportedOperation", () => {
  const err = A2aError.unsupportedOperation("gRPC");
  assertEquals(err.code, UNSUPPORTED_OPERATION);
  assertEquals(err.message, "Unsupported operation: gRPC");
});

Deno.test("contentTypeNotSupported", () => {
  const err = A2aError.contentTypeNotSupported("image/bmp");
  assertEquals(err.code, CONTENT_TYPE_NOT_SUPPORTED);
  assertEquals(err.message, "Content type not supported: image/bmp");
});

Deno.test("invalidRequest", () => {
  const err = A2aError.invalidRequest("missing field");
  assertEquals(err.code, INVALID_REQUEST);
  assertEquals(err.message, "missing field");
});

Deno.test("internal", () => {
  const err = A2aError.internal("something broke");
  assertEquals(err.code, INTERNAL_ERROR);
});

Deno.test("methodNotFound", () => {
  const err = A2aError.methodNotFound("foo/bar");
  assertEquals(err.code, METHOD_NOT_FOUND);
  assertEquals(err.message, "Method not found: foo/bar");
});

Deno.test("invalidParams", () => {
  const err = A2aError.invalidParams("bad params");
  assertEquals(err.code, INVALID_PARAMS);
});

Deno.test("parseError", () => {
  const err = A2aError.parseError("unexpected token");
  assertEquals(err.code, JSON_PARSE_ERROR);
});

Deno.test("extendedCardNotConfigured", () => {
  const err = A2aError.extendedCardNotConfigured();
  assertEquals(err.code, EXTENDED_CARD_NOT_CONFIGURED);
});

Deno.test("extensionSupportRequired", () => {
  const err = A2aError.extensionSupportRequired();
  assertEquals(err.code, EXTENSION_SUPPORT_REQUIRED);
  assertEquals(err.message, "Extension support is required but not provided");
});

Deno.test("versionNotSupported", () => {
  const err = A2aError.versionNotSupported();
  assertEquals(err.code, VERSION_NOT_SUPPORTED);
  assertEquals(err.message, "Protocol version is not supported");
});

// Error codes are correct values

Deno.test("error code constants", () => {
  assertEquals(JSON_PARSE_ERROR, -32700);
  assertEquals(INVALID_REQUEST, -32600);
  assertEquals(METHOD_NOT_FOUND, -32601);
  assertEquals(INVALID_PARAMS, -32602);
  assertEquals(INTERNAL_ERROR, -32603);
  assertEquals(TASK_NOT_FOUND, -32001);
  assertEquals(TASK_NOT_CANCELABLE, -32002);
  assertEquals(PUSH_NOT_SUPPORTED, -32003);
  assertEquals(UNSUPPORTED_OPERATION, -32004);
  assertEquals(CONTENT_TYPE_NOT_SUPPORTED, -32005);
  assertEquals(EXTENDED_CARD_NOT_CONFIGURED, -32007);
  assertEquals(EXTENSION_SUPPORT_REQUIRED, -32008);
  assertEquals(VERSION_NOT_SUPPORTED, -32009);
});
