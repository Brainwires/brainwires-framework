/**
 * Tests for PeerTable.
 * Equivalent to Rust peer_table::tests.
 */

import {
  assertEquals,
  assert,
  assertFalse,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { PeerTable, type TransportAddress } from "./peer_table.ts";
import { createAgentIdentity } from "./identity.ts";

function makePeer(name: string): {
  identity: ReturnType<typeof createAgentIdentity>;
  addrs: TransportAddress[];
} {
  return {
    identity: createAgentIdentity(name),
    addrs: [{ type: "tcp", address: "127.0.0.1:9090" }],
  };
}

Deno.test("peer table - upsert and get", () => {
  const table = new PeerTable();
  const { identity, addrs } = makePeer("agent-a");

  table.upsert(identity, addrs);

  assertEquals(table.length, 1);
  assertFalse(table.isEmpty);
  const found = table.get(identity.id);
  assert(found !== undefined);
  assertEquals(found!.name, "agent-a");
  assertEquals(table.getAddresses(identity.id)!.length, 1);
});

Deno.test("peer table - remove peer", () => {
  const table = new PeerTable();
  const { identity, addrs } = makePeer("agent-b");

  table.upsert(identity, addrs);
  const removed = table.remove(identity.id);
  assert(removed !== undefined);
  assertEquals(table.length, 0);
  assertEquals(table.get(identity.id), undefined);
});

Deno.test("peer table - topic subscriptions", () => {
  const table = new PeerTable();
  const { identity: idA, addrs: addrsA } = makePeer("a");
  const { identity: idB, addrs: addrsB } = makePeer("b");

  table.upsert(idA, addrsA);
  table.upsert(idB, addrsB);

  table.subscribe(idA.id, "status");
  table.subscribe(idB.id, "status");
  table.subscribe(idA.id, "errors");

  assertEquals(table.subscribers("status").length, 2);
  assertEquals(table.subscribers("errors").length, 1);
  assertEquals(table.subscribers("unknown").length, 0);

  table.unsubscribe(idA.id, "status");
  assertEquals(table.subscribers("status").length, 1);
});

Deno.test("peer table - remove peer cleans subscriptions", () => {
  const table = new PeerTable();
  const { identity, addrs } = makePeer("agent-c");

  table.upsert(identity, addrs);
  table.subscribe(identity.id, "events");
  assertEquals(table.subscribers("events").length, 1);

  table.remove(identity.id);
  assertEquals(table.subscribers("events").length, 0);
});
