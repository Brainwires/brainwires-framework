import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { DEFAULT_PERMISSION_MODE, parsePermissionMode } from "./mod.ts";

Deno.test("parsePermissionMode", () => {
  assertEquals(parsePermissionMode("read-only"), "read-only");
  assertEquals(parsePermissionMode("auto"), "auto");
  assertEquals(parsePermissionMode("full"), "full");
  assertEquals(parsePermissionMode("invalid"), undefined);
});

Deno.test("default permission mode is auto", () => {
  assertEquals(DEFAULT_PERMISSION_MODE, "auto");
});
