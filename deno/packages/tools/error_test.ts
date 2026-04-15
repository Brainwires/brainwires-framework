import { assertEquals } from "@std/assert";
import {
  classifyError,
  delayForAttempt,
  isRetryable,
  type RetryStrategy,
} from "./error.ts";

Deno.test("classifyError - transient errors", () => {
  const cat = classifyError("bash", "Connection refused");
  assertEquals(cat.type, "Transient");
  assertEquals(isRetryable(cat), true);
});

Deno.test("classifyError - permission errors", () => {
  const cat = classifyError("write_file", "Permission denied");
  assertEquals(cat.type, "Permission");
  assertEquals(isRetryable(cat), false);
});

Deno.test("classifyError - resource errors", () => {
  const cat = classifyError("read_file", "No such file or directory");
  assertEquals(cat.type, "Resource");
  if (cat.type === "Resource") {
    assertEquals(cat.resourceType, "FileNotFound");
  }
});

Deno.test("classifyError - bash command not found", () => {
  const cat = classifyError("execute_command", "command not found: foobar");
  assertEquals(cat.type, "InputValidation");
});

Deno.test("classifyError - web ssl error", () => {
  const cat = classifyError("fetch_url", "SSL certificate problem");
  assertEquals(cat.type, "ExternalService");
});

Deno.test("classifyError - unknown tool", () => {
  const cat = classifyError("some_tool", "Something weird happened");
  assertEquals(cat.type, "Unknown");
});

Deno.test("RetryStrategy - exponential backoff delay", () => {
  const strategy: RetryStrategy = {
    type: "ExponentialBackoff",
    baseMs: 100,
    maxAttempts: 3,
  };
  assertEquals(delayForAttempt(strategy, 0), 100);
  assertEquals(delayForAttempt(strategy, 1), 200);
  assertEquals(delayForAttempt(strategy, 2), 400);
  assertEquals(delayForAttempt(strategy, 3), undefined);
});

Deno.test("RetryStrategy - no retry", () => {
  const strategy: RetryStrategy = { type: "NoRetry" };
  assertEquals(delayForAttempt(strategy, 0), undefined);
});

Deno.test("RetryStrategy - immediate", () => {
  const strategy: RetryStrategy = { type: "Immediate", maxAttempts: 2 };
  assertEquals(delayForAttempt(strategy, 0), 0);
  assertEquals(delayForAttempt(strategy, 1), 0);
  assertEquals(delayForAttempt(strategy, 2), undefined);
});

Deno.test("RetryStrategy - fixed delay", () => {
  const strategy: RetryStrategy = {
    type: "FixedDelay",
    delayMs: 500,
    maxAttempts: 2,
  };
  assertEquals(delayForAttempt(strategy, 0), 500);
  assertEquals(delayForAttempt(strategy, 1), 500);
  assertEquals(delayForAttempt(strategy, 2), undefined);
});
