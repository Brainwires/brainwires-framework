/**
 * Serialization round-trip tests for core A2A types.
 */

import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import type {
  Artifact,
  DataPart,
  FileContent,
  FileContentBytes,
  FileContentUri,
  FilePart,
  Message,
  Part,
  TextPart,
} from "./types.ts";
import { createAgentMessage, createUserMessage } from "./types.ts";

Deno.test("TextPart round-trips through JSON", () => {
  const part: TextPart = { kind: "text", text: "hello world" };
  const json = JSON.stringify(part);
  const parsed = JSON.parse(json) as TextPart;
  assertEquals(parsed.kind, "text");
  assertEquals(parsed.text, "hello world");
  assertEquals(parsed.metadata, undefined);
});

Deno.test("TextPart with metadata round-trips", () => {
  const part: TextPart = {
    kind: "text",
    text: "hi",
    metadata: { lang: "en" },
  };
  const parsed = JSON.parse(JSON.stringify(part)) as TextPart;
  assertEquals(parsed.metadata?.lang, "en");
});

Deno.test("FilePart with bytes round-trips", () => {
  const file: FileContentBytes = {
    bytes: "aGVsbG8=",
    mimeType: "text/plain",
    name: "hello.txt",
  };
  const part: FilePart = { kind: "file", file };
  const parsed = JSON.parse(JSON.stringify(part)) as FilePart;
  assertEquals(parsed.kind, "file");
  assertEquals((parsed.file as FileContentBytes).bytes, "aGVsbG8=");
  assertEquals(parsed.file.mimeType, "text/plain");
  assertEquals(parsed.file.name, "hello.txt");
});

Deno.test("FilePart with URI round-trips", () => {
  const file: FileContentUri = {
    uri: "https://example.com/file.pdf",
    mimeType: "application/pdf",
  };
  const part: FilePart = { kind: "file", file };
  const parsed = JSON.parse(JSON.stringify(part)) as FilePart;
  assertEquals(parsed.kind, "file");
  assertEquals((parsed.file as FileContentUri).uri, "https://example.com/file.pdf");
});

Deno.test("DataPart round-trips", () => {
  const part: DataPart = {
    kind: "data",
    data: { key: "value", nested: [1, 2, 3] },
  };
  const parsed = JSON.parse(JSON.stringify(part)) as DataPart;
  assertEquals(parsed.kind, "data");
  assertEquals((parsed.data as Record<string, unknown>).key, "value");
});

Deno.test("Part discriminated union serializes correctly", () => {
  const parts: Part[] = [
    { kind: "text", text: "hello" },
    { kind: "file", file: { uri: "https://example.com/f" } },
    { kind: "data", data: 42 },
  ];
  const json = JSON.stringify(parts);
  const parsed = JSON.parse(json) as Part[];
  assertEquals(parsed.length, 3);
  assertEquals(parsed[0].kind, "text");
  assertEquals(parsed[1].kind, "file");
  assertEquals(parsed[2].kind, "data");
});

Deno.test("FileContent untagged union: bytes variant", () => {
  const fc: FileContent = { bytes: "AQID", mimeType: "application/octet-stream" };
  const json = JSON.stringify(fc);
  const parsed = JSON.parse(json) as FileContent;
  assertEquals("bytes" in parsed, true);
  assertEquals((parsed as FileContentBytes).bytes, "AQID");
});

Deno.test("FileContent untagged union: uri variant", () => {
  const fc: FileContent = { uri: "s3://bucket/key" };
  const json = JSON.stringify(fc);
  const parsed = JSON.parse(json) as FileContent;
  assertEquals("uri" in parsed, true);
  assertEquals((parsed as FileContentUri).uri, "s3://bucket/key");
});

Deno.test("Message round-trips with camelCase fields", () => {
  const msg: Message = {
    messageId: "msg-1",
    role: "user",
    parts: [{ kind: "text", text: "hi" }],
    contextId: "ctx-1",
    taskId: "task-1",
    referenceTaskIds: ["task-0"],
    metadata: { source: "test" },
    extensions: ["ext://custom"],
    kind: "message",
  };
  const json = JSON.stringify(msg);
  const parsed = JSON.parse(json) as Message;

  assertEquals(parsed.messageId, "msg-1");
  assertEquals(parsed.role, "user");
  assertEquals(parsed.contextId, "ctx-1");
  assertEquals(parsed.taskId, "task-1");
  assertEquals(parsed.referenceTaskIds, ["task-0"]);
  assertEquals(parsed.kind, "message");
});

Deno.test("Message optional fields omitted when undefined", () => {
  const msg: Message = {
    messageId: "msg-2",
    role: "agent",
    parts: [],
    kind: "message",
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
  assertEquals(msg.role, "user");
  assertEquals(msg.parts.length, 1);
  assertEquals(msg.parts[0].kind, "text");
  assertEquals((msg.parts[0] as TextPart).text, "test");
  assertEquals(msg.kind, "message");
  assertEquals(typeof msg.messageId, "string");
  assertEquals(msg.messageId.length > 0, true);
});

Deno.test("createAgentMessage creates valid agent message", () => {
  const msg = createAgentMessage("response");
  assertEquals(msg.role, "agent");
  assertEquals(msg.parts.length, 1);
  assertEquals((msg.parts[0] as TextPart).text, "response");
  assertEquals(msg.kind, "message");
});

Deno.test("Artifact round-trips", () => {
  const artifact: Artifact = {
    artifactId: "art-1",
    name: "output.txt",
    description: "Generated text file",
    parts: [{ kind: "text", text: "file contents" }],
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
    parts: [{ kind: "data", data: null }],
  };
  const json = JSON.stringify(artifact);
  const obj = JSON.parse(json);
  assertEquals(obj.name, undefined);
  assertEquals(obj.description, undefined);
  assertEquals(obj.metadata, undefined);
  assertEquals(obj.extensions, undefined);
});
