/**
 * @module routing
 *
 * Message routing: direct, broadcast, and content-based.
 * Equivalent to Rust's `Router`, `DirectRouter`, `BroadcastRouter`,
 * `ContentRouter`, `RoutingStrategy`.
 */

import type { MessageEnvelope } from "./envelope.ts";
import type { PeerTable, TransportAddress } from "./peer_table.ts";

/**
 * Routing strategy identifier.
 * Equivalent to Rust `RoutingStrategy`.
 */
export type RoutingStrategy =
  | { type: "direct" }
  | { type: "broadcast" }
  | { type: "contentBased" }
  | { type: "custom"; name: string };

/**
 * The core routing interface.
 * Equivalent to Rust `Router` trait.
 */
export interface Router {
  /** Determine the transport addresses to deliver this message to. */
  route(
    envelope: MessageEnvelope,
    peers: PeerTable,
  ): Promise<TransportAddress[]>;

  /** The routing strategy this router implements. */
  strategy(): RoutingStrategy;
}

/**
 * Point-to-point router.
 * Looks up the recipient UUID in the peer table and returns its addresses.
 * Equivalent to Rust `DirectRouter`.
 */
export class DirectRouter implements Router {
  // deno-lint-ignore require-await
  async route(
    envelope: MessageEnvelope,
    peers: PeerTable,
  ): Promise<TransportAddress[]> {
    if (envelope.recipient.type !== "direct") {
      throw new Error(
        `DirectRouter does not handle ${envelope.recipient.type} messages`,
      );
    }
    const addrs = peers.getAddresses(envelope.recipient.id);
    if (!addrs) {
      throw new Error(`No route to peer ${envelope.recipient.id}`);
    }
    return addrs;
  }

  strategy(): RoutingStrategy {
    return { type: "direct" };
  }
}

/**
 * Broadcast router.
 * Returns addresses of all known peers except the sender.
 * Equivalent to Rust `BroadcastRouter`.
 */
export class BroadcastRouter implements Router {
  // deno-lint-ignore require-await
  async route(
    envelope: MessageEnvelope,
    peers: PeerTable,
  ): Promise<TransportAddress[]> {
    const addrs: TransportAddress[] = [];
    for (const peerId of peers.allPeerIds()) {
      if (peerId === envelope.sender) continue;
      const peerAddrs = peers.getAddresses(peerId);
      if (peerAddrs) {
        addrs.push(...peerAddrs);
      }
    }
    return addrs;
  }

  strategy(): RoutingStrategy {
    return { type: "broadcast" };
  }
}

/**
 * Content-based (topic) router.
 * Routes to all peers subscribed to the message's topic.
 * Equivalent to Rust `ContentRouter`.
 */
export class ContentRouter implements Router {
  // deno-lint-ignore require-await
  async route(
    envelope: MessageEnvelope,
    peers: PeerTable,
  ): Promise<TransportAddress[]> {
    if (envelope.recipient.type !== "topic") {
      throw new Error(
        `ContentRouter does not handle ${envelope.recipient.type} messages`,
      );
    }
    const subscribers = peers.subscribers(envelope.recipient.topic);
    const addrs: TransportAddress[] = [];
    for (const subId of subscribers) {
      if (subId === envelope.sender) continue;
      const peerAddrs = peers.getAddresses(subId);
      if (peerAddrs) {
        addrs.push(...peerAddrs);
      }
    }
    return addrs;
  }

  strategy(): RoutingStrategy {
    return { type: "contentBased" };
  }
}
