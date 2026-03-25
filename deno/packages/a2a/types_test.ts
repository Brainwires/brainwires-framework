/**
 * Serialization round-trip tests for core A2A types (v1.0).
 */

import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import type {
  Artifact,
  Message,
  Part,
} from "./types.ts";
import { createAgentMessage, createUserMessage } from "./types.ts";

Deno.test("Part with text round-trips through JSON", () => {
  const part: Part = { text: "hello world" };
  const json = JSON.stringify(part);
  const parsed = JSON.parse(json) as Part;
  assertEquals(parsed.text, "hello world");
  assertEquals(parsed.metadata, undefined);
});

Deno.test("Part with text and metadata round-trips", () => {
  const part: Part = {
    text: "hi",
    metadata: { lang: "en" },
  };
  const parsed = JSON.parse(JSON.stringify(part)) as Part;
  assertEquals(parsed.metadata?.lang, "en");
});

Deno.test("Part with raw bytes round-trips", () => {
  const part: Part = {
    raw: "aGVsbG8=",
    mediaType: "text/plain",
    filename: "hello.txt",
  };
  const parsed = JSON.parse(JSON.stringify(part)) as Part;
  assertEquals(parsed.raw, "aGVsbG8=");
  assertEquals(parsed.mediaType, "text/plain");
  assertEquals(parsed.filename, "hello.txt");
});

Deno.test("Part with URL round-trips", () => {
  const part: Part = {
    url: "https://example.com/file.pdf",
    mediaType: "application/pdf",
  };
  const parsed = JSON.parse(JSON.stringify(part)) as Part;
  assertEquals(parsed.url, "https://example.com/file.pdf");
  assertEquals(parsed.mediaType, "application/pdf");
});

Deno.test("Part with data round-trips", () => {
  const part: Part = {
    data: { key: "value", nested: [1, 2, 3] },
  };
  const parsed = JSON.parse(JSON.stringify(part)) as Part;
  assertEquals((parsed.data as Record<string, unknown>).key, "value");
});

Deno.test("Part array serializes correctly", () => {
  const parts: Part[] = [
    { text: "hello" },
    { url: "https://example.com/f", mediaType: "text/plain" },
    { data: 42 },
  ];
  const json = JSON.stringify(parts);
  const parsed = JSON.parse(json) as Part[];
  assertEquals(parsed.length, 3);
  assertEquals(parsed[0].text, "hello");
  assertEquals(parsed[1].url, "https://example.com/f");
  assertEquals(parsed[2].data, 42);
});

Deno.test("Part with raw bytes and mediaType", () => {
  const part: Part = { raw: "AQID", mediaType: "application/octet-stream" };
  const json = JSON.stringify(part);
  const parsed = JSON.parse(json) as Part;
  assertEquals(parsed.raw, "AQID");
  assertEquals(parsed.mediaType, "application/octet-stream");
});

Deno.test("Part with url field", () => {
  const part: Part = { url: "s3://bucket/key" };
  const json = JSON.stringify(part);
  const parsed = JSON.parse(json) as Part;
  assertEquals(parsed.url, "s3://bucket/key");
});

Deno.test("Message round-trips with camelCase fields", () => {
  const msg: Message = {
    messageId: "msg-1",
    role: "ROLE_USER",
    parts: [{ text: "hi" }],
    contextId: "ctx-1",
    taskId: "task-1",
    referenceTaskIds: ["task-0"],
    metadata: { source: "test" },
    extensions: ["ext://custom"],
  };
  const json = JSON.stringify(msg);
  const parsed = JSON.parse(json) as Message;

  assertEquals(parsed.messageId, "msg-1");
  assertEquals(parsed.role, "ROLE_USER");
  assertEquals(parsed.contextId, "ctx-1");
  assertEquals(parsed.taskId, "task-1");
  assertEquals(parsed.referenceTaskIds, ["task-0"]);
});

Deno.test("Message optional fields omitted when undefined", () => {
  const msg: Message = {
    messageId: "msg-2",
    role: "ROLE_AGENT",
    parts: [],
  };
  const json = JSON.stringify(msg);
  const obj = JSON.parse(json);
  assertEquals(obj.contextId, undefined);
  assertEquals(obj.taskId, undefined);
  assertEquals(obj.referenceTaskIds, undefined);
  assertEquals(obj.metadata, undefined);
  assertEquals(obj.extensions, undefined);
});

Deno.test("createUserMessage creates valid user message", () => {
  const msg = createUserMessage("test");
  assertEquals(msg.role, "ROLE_USER");
  assertEquals(msg.parts.length, 1);
  assertEquals(msg.parts[0].text, "test");
  assertEquals(typeof msg.messageId, "string");
  assertEquals(msg.messageId.length > 0, true);
});

Deno.test("createAgentMessage creates valid agent message", () => {
  const msg = createAgentMessage("response");
  assertEquals(msg.role, "ROLE_AGENT");
  assertEquals(msg.parts.length, 1);
  assertEquals(msg.parts[0].text, "response");
});

Deno.test("Artifact round-trips", () => {
  const artifact: Artifact = {
    artifactId: "art-1",
    name: "output.txt",
    description: "Generated text file",
    parts: [{ text: "file contents" }],
    metadata: { format: "plain" },
    extensions: ["ext://a"],
  };
  const parsed = JSON.parse(JSON.stringify(artifact)) as Artifact;
  assertEquals(parsed.artifactId, "art-1");
  assertEquals(parsed.name, "output.txt");
  assertEquals(parsed.parts.length, 1);
});

Deno.test("Artifact optional fields omitted", () => {
  const artifact: Artifact = {
    artifactId: "art-2",
    parts: [{ data: null }],
  };
  const json = JSON.stringify(artifact);
  const obj = JSON.parse(json);
  assertEquals(obj.name, undefined);
  assertEquals(obj.description, undefined);
  assertEquals(obj.metadata, undefined);
  assertEquals(obj.extensions, undefined);
});
