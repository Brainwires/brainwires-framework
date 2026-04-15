import { assertEquals } from "@std/assert";
import { estimateTokens, WorkingSet } from "./mod.ts";

Deno.test("WorkingSet add and access", () => {
  const ws = new WorkingSet();
  ws.add("/test/file1.rs", 1000);
  assertEquals(ws.length, 1);
  assertEquals(ws.contains("/test/file1.rs"), true);
});

Deno.test("WorkingSet LRU eviction", () => {
  const ws = new WorkingSet({
    max_files: 3,
    max_tokens: 100_000,
    stale_after_turns: 10,
    auto_evict: false,
  });
  ws.add("/test/file1.rs", 100);
  ws.nextTurn();
  ws.add("/test/file2.rs", 100);
  ws.nextTurn();
  ws.add("/test/file3.rs", 100);
  ws.nextTurn();
  ws.add("/test/file4.rs", 100);
  assertEquals(ws.length, 3);
  assertEquals(ws.contains("/test/file1.rs"), false);
});

Deno.test("estimateTokens", () => {
  assertEquals(estimateTokens(""), 0);
  assertEquals(estimateTokens("test"), 1);
});

Deno.test("WorkingSet pinned files resist eviction", () => {
  const ws = new WorkingSet({
    max_files: 2,
    max_tokens: 100_000,
    stale_after_turns: 10,
    auto_evict: false,
  });
  ws.addPinned("/pinned.rs", 100);
  ws.add("/normal1.rs", 100);
  ws.add("/normal2.rs", 100);
  // Should evict normal1 not pinned
  assertEquals(ws.contains("/pinned.rs"), true);
  assertEquals(ws.length, 2);
});
