import { assertEquals } from "jsr:@std/assert@^1.0.0";
import { analyzeQuery } from "./smart_router.ts";

Deno.test("analyzeQuery - git query", () => {
  const categories = analyzeQuery("Show me the git diff");
  assertEquals(categories.includes("Git"), true);
});

Deno.test("analyzeQuery - file query", () => {
  const categories = analyzeQuery("Read the config file");
  assertEquals(categories.includes("FileOps"), true);
});

Deno.test("analyzeQuery - search query", () => {
  const categories = analyzeQuery("Find all functions named handle");
  assertEquals(categories.includes("Search"), true);
});

Deno.test("analyzeQuery - web search query", () => {
  const categories = analyzeQuery(
    "Search the web for Rust best practices",
  );
  assertEquals(categories.includes("WebSearch"), true);
});

Deno.test("analyzeQuery - bash query", () => {
  const categories = analyzeQuery("Run cargo build");
  assertEquals(categories.includes("Bash"), true);
});

Deno.test("analyzeQuery - default categories on unrelated text", () => {
  const categories = analyzeQuery("Hello, how are you?");
  assertEquals(categories.length > 0, true);
  assertEquals(categories.includes("FileOps"), true);
  assertEquals(categories.includes("Search"), true);
  assertEquals(categories.includes("Bash"), true);
});

Deno.test("analyzeQuery - FileOps always included", () => {
  const categories = analyzeQuery("Show me the git status");
  assertEquals(categories.includes("FileOps"), true);
  assertEquals(categories.includes("Git"), true);
});

Deno.test("analyzeQuery - multiple categories", () => {
  const categories = analyzeQuery("Search for files and run the tests");
  assertEquals(categories.includes("Search"), true);
  assertEquals(categories.includes("Bash"), true);
});
