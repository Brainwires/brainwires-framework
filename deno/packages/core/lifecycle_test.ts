import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  defaultEventFilter,
  eventAgentId,
  eventToolName,
  eventType,
  filterMatches,
  HookRegistry,
  type LifecycleEvent,
  type LifecycleHook,
} from "./mod.ts";

Deno.test("HookRegistry register", () => {
  const registry = new HookRegistry();
  assertEquals(registry.isEmpty(), true);
  const hook: LifecycleHook = {
    name: "test",
    async onEvent() {
      return { type: "continue" };
    },
  };
  registry.register(hook);
  assertEquals(registry.length, 1);
});

Deno.test("EventFilter matches all by default", () => {
  const filter = defaultEventFilter();
  const event: LifecycleEvent = {
    type: "agent_started",
    agent_id: "a1",
    task_description: "test",
  };
  assertEquals(filterMatches(filter, event), true);
});

Deno.test("EventFilter by type", () => {
  const filter = {
    ...defaultEventFilter(),
    event_types: new Set(["agent_started"]),
  };
  const started: LifecycleEvent = {
    type: "agent_started",
    agent_id: "a1",
    task_description: "test",
  };
  const completed: LifecycleEvent = {
    type: "agent_completed",
    agent_id: "a1",
    iterations: 5,
    summary: "done",
  };
  assertEquals(filterMatches(filter, started), true);
  assertEquals(filterMatches(filter, completed), false);
});

Deno.test("event type and agent_id extraction", () => {
  const event: LifecycleEvent = {
    type: "tool_before_execute",
    agent_id: "a1",
    tool_name: "read_file",
    args: {},
  };
  assertEquals(eventType(event), "tool_before_execute");
  assertEquals(eventAgentId(event), "a1");
  assertEquals(eventToolName(event), "read_file");
});
