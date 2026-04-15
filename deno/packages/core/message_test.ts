import { assertEquals } from "@std/assert";
import {
  createUsage,
  Message,
  serializeMessagesToStatelessHistory,
  type ToolUseBlock,
} from "./mod.ts";

Deno.test("Message.user creates user message", () => {
  const msg = Message.user("Hello");
  assertEquals(msg.role, "user");
  assertEquals(msg.text(), "Hello");
});

Deno.test("Message.assistant creates assistant message", () => {
  const msg = Message.assistant("Response");
  assertEquals(msg.role, "assistant");
  assertEquals(msg.text(), "Response");
});

Deno.test("Message.toolResult creates tool result message", () => {
  const msg = Message.toolResult("tool-1", "Result");
  assertEquals(msg.role, "tool");
  assertEquals(msg.text(), undefined);
});

Deno.test("createUsage calculates total tokens", () => {
  const usage = createUsage(100, 50);
  assertEquals(usage.total_tokens, 150);
});

Deno.test("Role serializes as lowercase string", () => {
  const msg = Message.user("test");
  assertEquals(JSON.parse(JSON.stringify(msg.toJSON())).role, "user");
});

Deno.test("serializeMessagesToStatelessHistory - simple text", () => {
  const messages = [Message.user("Hello"), Message.assistant("Hi there")];
  const history = serializeMessagesToStatelessHistory(messages);
  assertEquals(history.length, 2);
  assertEquals(history[0].role, "user");
  assertEquals(history[1].role, "assistant");
});

Deno.test("serializeMessagesToStatelessHistory - skips system", () => {
  const messages = [Message.system("You are helpful"), Message.user("Hello")];
  const history = serializeMessagesToStatelessHistory(messages);
  assertEquals(history.length, 1);
  assertEquals(history[0].role, "user");
});

Deno.test("serializeMessagesToStatelessHistory - tool round trip", () => {
  const messages = [
    Message.user("Read the file"),
    new Message({
      role: "assistant",
      content: [
        { type: "text", text: "I'll check." },
        {
          type: "tool_use",
          id: "call-1",
          name: "read_file",
          input: { path: "main.rs" },
        } as ToolUseBlock,
      ],
    }),
    Message.toolResult("call-1", "fn main() {}"),
    Message.assistant("The file contains a main function."),
  ];
  const history = serializeMessagesToStatelessHistory(messages);
  assertEquals(history.length, 5);
  assertEquals(history[0].role, "user");
  assertEquals(history[1].role, "assistant");
  assertEquals(history[2].role, "function_call");
  assertEquals(history[3].role, "tool");
  assertEquals(history[4].role, "assistant");
});

Deno.test("Message.textOrSummary with blocks", () => {
  const msg = new Message({
    role: "assistant",
    content: [
      { type: "text", text: "Let me check." },
      { type: "image", source: { type: "base64", media_type: "image/png", data: "..." } },
    ],
  });
  assertEquals(msg.textOrSummary(), "Let me check.\n[Image]");
});

Deno.test("Message.toJSON omits undefined fields", () => {
  const msg = Message.user("test");
  const json = msg.toJSON();
  assertEquals("name" in json, false);
  assertEquals("metadata" in json, false);
});
