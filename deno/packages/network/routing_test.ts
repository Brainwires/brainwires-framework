/**
 * Tests for routing: DirectRouter, BroadcastRouter, ContentRouter.
 * Equivalent to Rust routing tests.
 */

import {
  assertEquals,
  assert,
  assertRejects,
} from "@std/assert";
import { DirectRouter, BroadcastRouter, ContentRouter } from "./routing.ts";
import { PeerTable, type TransportAddress } from "./peer_table.ts";
import { createAgentIdentity } from "./identity.ts";
import {
  directEnvelope,
  broadcastEnvelope,
  topicEnvelope,
  textPayload,
} from "./envelope.ts";

function tcpAddr(address: string): TransportAddress {
  return { type: "tcp", address };
}

Deno.test("DirectRouter - routes to known peer", async () => {
  const router = new DirectRouter();
  const peers = new PeerTable();

  const target = createAgentIdentity("target");
  const addr = tcpAddr("127.0.0.1:9090");
  peers.upsert(target, [addr]);

  const env = directEnvelope(
    crypto.randomUUID(),
    target.id,
    textPayload("hello"),
  );
  const addrs = await router.route(env, peers);
  assertEquals(addrs.length, 1);
  assertEquals(addrs[0], addr);
});

Deno.test("DirectRouter - fails for unknown peer", async () => {
  const router = new DirectRouter();
  const peers = new PeerTable();

  const env = directEnvelope(
    crypto.randomUUID(),
    crypto.randomUUID(),
    textPayload("hello"),
  );

  await assertRejects(() => router.route(env, peers), Error, "No route");
});

Deno.test("DirectRouter - rejects broadcast", async () => {
  const router = new DirectRouter();
  const peers = new PeerTable();

  const env = broadcastEnvelope(crypto.randomUUID(), textPayload("hello"));
  await assertRejects(
    () => router.route(env, peers),
    Error,
    "does not handle",
  );
});

Deno.test("BroadcastRouter - reaches all except sender", async () => {
  const router = new BroadcastRouter();
  const peers = new PeerTable();

  const sender = createAgentIdentity("sender");
  const peerA = createAgentIdentity("a");
  const peerB = createAgentIdentity("b");

  peers.upsert(sender, [tcpAddr("127.0.0.1:1000")]);
  peers.upsert(peerA, [tcpAddr("127.0.0.1:2000")]);
  peers.upsert(peerB, [tcpAddr("127.0.0.1:3000")]);

  const env = broadcastEnvelope(sender.id, textPayload("ping"));
  const addrs = await router.route(env, peers);

  assertEquals(addrs.length, 2);
  // Sender address should not be included
  const addrStrings = addrs.map((a) =>
    a.type === "tcp" ? a.address : ""
  );
  assert(!addrStrings.includes("127.0.0.1:1000"));
});

Deno.test("BroadcastRouter - empty peers", async () => {
  const router = new BroadcastRouter();
  const peers = new PeerTable();

  const env = broadcastEnvelope(crypto.randomUUID(), textPayload("ping"));
  const addrs = await router.route(env, peers);
  assertEquals(addrs.length, 0);
});

Deno.test("ContentRouter - routes to subscribers", async () => {
  const router = new ContentRouter();
  const peers = new PeerTable();

  const sender = createAgentIdentity("sender");
  const subA = createAgentIdentity("sub-a");
  const subB = createAgentIdentity("sub-b");
  const nonSub = createAgentIdentity("non-sub");

  const addrA = tcpAddr("127.0.0.1:1000");
  const addrB = tcpAddr("127.0.0.1:2000");
  const addrNs = tcpAddr("127.0.0.1:3000");

  peers.upsert(sender, []);
  peers.upsert(subA, [addrA]);
  peers.upsert(subB, [addrB]);
  peers.upsert(nonSub, [addrNs]);

  peers.subscribe(subA.id, "events");
  peers.subscribe(subB.id, "events");

  const env = topicEnvelope(sender.id, "events", textPayload("update"));
  const addrs = await router.route(env, peers);

  assertEquals(addrs.length, 2);
  assert(addrs.some((a) => a.type === "tcp" && a.address === "127.0.0.1:1000"));
  assert(addrs.some((a) => a.type === "tcp" && a.address === "127.0.0.1:2000"));
});

Deno.test("ContentRouter - empty topic", async () => {
  const router = new ContentRouter();
  const peers = new PeerTable();

  const env = topicEnvelope(
    crypto.randomUUID(),
    "no-subscribers",
    textPayload("hello"),
  );
  const addrs = await router.route(env, peers);
  assertEquals(addrs.length, 0);
});

Deno.test("ContentRouter - rejects direct", async () => {
  const router = new ContentRouter();
  const peers = new PeerTable();

  const env = directEnvelope(
    crypto.randomUUID(),
    crypto.randomUUID(),
    textPayload("hello"),
  );
  await assertRejects(
    () => router.route(env, peers),
    Error,
    "does not handle",
  );
});
