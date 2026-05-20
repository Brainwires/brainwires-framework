import { assert } from "@std/assert/assert";
import { assertEquals } from "@std/assert/equals";
import { type FileContent, FileContextManager } from "./file_context.ts";

Deno.test("formatContent: full", () => {
  const c: FileContent = { kind: "full", content: "hello world" };
  assertEquals(FileContextManager.formatContent(c), "hello world");
});

Deno.test("formatContent: already_in_context", () => {
  const c: FileContent = {
    kind: "already_in_context",
    path: "/path/to/file.ts",
  };
  const s = FileContextManager.formatContent(c);
  assert(s.includes("already shown above"));
  assert(s.includes("/path/to/file.ts"));
});

Deno.test("formatContent: chunked", () => {
  const c: FileContent = {
    kind: "chunked",
    path: "/path/to/file.ts",
    total_size: 50000,
    chunks: [
      {
        content: "fn main() {}",
        line_start: 1,
        line_end: 1,
        relevance_score: 0.95,
      },
      {
        content: "fn helper() {}",
        line_start: 10,
        line_end: 10,
        relevance_score: 0.85,
      },
    ],
    has_more: true,
  };
  const s = FileContextManager.formatContent(c);
  assert(s.includes("/path/to/file.ts"));
  assert(s.includes("50000 chars"));
  assert(s.includes("2 relevant sections"));
  assert(s.includes("fn main()"));
  assert(s.includes("fn helper()"));
  assert(s.includes("more content available"));
});

Deno.test("computeHash deterministic + SHA-256 hex length", async () => {
  const h1 = await FileContextManager.computeHash("hello world");
  const h2 = await FileContextManager.computeHash("hello world");
  const h3 = await FileContextManager.computeHash("different content");
  assertEquals(h1, h2);
  assert(h1 !== h3);
  assertEquals(h1.length, 64);
});

Deno.test("context tracking add/check/clear", () => {
  const m = new FileContextManager();
  assert(!m.isInContext("/some/file.ts"));
  assertEquals(m.contextFileCount(), 0);

  m.markInContext("/some/file.ts");
  assert(m.isInContext("/some/file.ts"));
  assertEquals(m.contextFileCount(), 1);

  m.clearContext();
  assert(!m.isInContext("/some/file.ts"));
  assertEquals(m.contextFileCount(), 0);
});

Deno.test("buildFileChunks: small content produces one chunk starting at line 1", () => {
  const m = new FileContextManager();
  const chunks = m.buildFileChunks("line 1\nline 2\nline 3\nline 4\nline 5");
  assert(chunks.length > 0);
  assertEquals(chunks[0].line_start, 1);
});

Deno.test("findRelevantChunks: ranks matching content higher", () => {
  const m = new FileContextManager();
  const chunks = [
    {
      content: "This is about authentication and login",
      line_start: 1,
      line_end: 1,
      relevance_score: 1.0,
    },
    {
      content: "This is about database queries",
      line_start: 2,
      line_end: 2,
      relevance_score: 1.0,
    },
    {
      content: "This handles user login flow",
      line_start: 3,
      line_end: 3,
      relevance_score: 1.0,
    },
  ];
  const relevant = m.findRelevantChunks(chunks, "login authentication");
  assert(relevant.length > 0);
  assert(
    relevant[0].content.includes("login") ||
      relevant[0].content.includes("authentication"),
  );
});

Deno.test("getFileContent: full content for small file", async () => {
  const tmp = await Deno.makeTempFile({ suffix: ".txt" });
  try {
    await Deno.writeTextFile(tmp, "hello from disk");
    const m = new FileContextManager();
    const result = await m.getFileContent(tmp);
    assertEquals(result.kind, "full");
    if (result.kind === "full") assertEquals(result.content, "hello from disk");
    // Second fetch returns already_in_context
    const second = await m.getFileContent(tmp);
    assertEquals(second.kind, "already_in_context");
  } finally {
    await Deno.remove(tmp);
  }
});
