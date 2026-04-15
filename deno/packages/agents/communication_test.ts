import { assertEquals, assertThrows } from "@std/assert";
import { CommunicationHub } from "./communication.ts";

Deno.test("register and unregister agent", () => {
  const hub = new CommunicationHub();

  hub.registerAgent("agent-1");
  assertEquals(hub.agentCount(), 1);
  assertEquals(hub.isRegistered("agent-1"), true);

  // Duplicate registration should throw
  assertThrows(() => hub.registerAgent("agent-1"));

  hub.unregisterAgent("agent-1");
  assertEquals(hub.agentCount(), 0);
  assertEquals(hub.isRegistered("agent-1"), false);
});

// deno-lint-ignore require-await
Deno.test("send and receive message", async () => {
  const hub = new CommunicationHub();
  hub.registerAgent("agent-1");
  hub.registerAgent("agent-2");

  hub.sendMessage("agent-1", "agent-2", {
    type: "task_request",
    taskId: "task-1",
    description: "Do something",
    priority: 5,
  });

  const envelope = hub.tryReceiveMessage("agent-2");
  assertEquals(envelope?.from, "agent-1");
  assertEquals(envelope?.to, "agent-2");
  assertEquals(envelope?.message.type, "task_request");
});

Deno.test("broadcast", () => {
  const hub = new CommunicationHub();
  hub.registerAgent("agent-1");
  hub.registerAgent("agent-2");
  hub.registerAgent("agent-3");

  hub.broadcast("orchestrator", {
    type: "broadcast",
    sender: "orchestrator",
    message: "Hello all!",
  });

  // All agents should receive the message
  assertEquals(hub.tryReceiveMessage("agent-1") !== undefined, true);
  assertEquals(hub.tryReceiveMessage("agent-2") !== undefined, true);
  assertEquals(hub.tryReceiveMessage("agent-3") !== undefined, true);
});

Deno.test("send to unregistered agent throws", () => {
  const hub = new CommunicationHub();
  hub.registerAgent("agent-1");

  assertThrows(() =>
    hub.sendMessage("agent-1", "nonexistent", {
      type: "broadcast",
      sender: "agent-1",
      message: "hi",
    })
  );
});

Deno.test("list agents", () => {
  const hub = new CommunicationHub();
  hub.registerAgent("a");
  hub.registerAgent("b");

  const agents = hub.listAgents();
  assertEquals(agents.length, 2);
  assertEquals(agents.includes("a"), true);
  assertEquals(agents.includes("b"), true);
});
