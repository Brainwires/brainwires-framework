import { assertEquals } from "@std/assert";
import { edgeTypeWeight } from "./mod.ts";

Deno.test("edgeTypeWeight values", () => {
  assertEquals(edgeTypeWeight("defines"), 1.0);
  assertEquals(edgeTypeWeight("contains"), 0.9);
  assertEquals(edgeTypeWeight("depends_on"), 0.8);
  assertEquals(edgeTypeWeight("co_occurs"), 0.3);
});
