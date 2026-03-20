/**
 * @module peer_table
 *
 * Peer table for routing decisions.
 * Equivalent to Rust's `PeerTable`.
 */

import type { AgentIdentity } from "./identity.ts";

/** A transport address for reaching a peer. Equivalent to Rust `TransportAddress`. */
export type TransportAddress =
  | { type: "unix"; path: string }
  | { type: "tcp"; address: string }
  | { type: "url"; url: string }
  | { type: "channel"; channel: string };

/** Display a transport address as a string. */
export function displayTransportAddress(addr: TransportAddress): string {
  switch (addr.type) {
    case "unix":
      return `unix://${addr.path}`;
    case "tcp":
      return `tcp://${addr.address}`;
    case "url":
      return addr.url;
    case "channel":
      return `channel://${addr.channel}`;
  }
}

/**
 * A table of known peers and their reachable transport addresses.
 * Central data structure for routing decisions.
 * Equivalent to Rust `PeerTable`.
 */
export class PeerTable {
  private peers: Map<string, AgentIdentity> = new Map();
  private addresses: Map<string, TransportAddress[]> = new Map();
  private subscriptions: Map<string, Set<string>> = new Map();

  /** Add or update a peer in the table. */
  upsert(identity: AgentIdentity, addresses: TransportAddress[]): void {
    this.peers.set(identity.id, identity);
    this.addresses.set(identity.id, addresses);
  }

  /** Remove a peer from the table. */
  remove(id: string): AgentIdentity | undefined {
    this.addresses.delete(id);
    // Remove from all topic subscriptions
    for (const subs of this.subscriptions.values()) {
      subs.delete(id);
    }
    const identity = this.peers.get(id);
    this.peers.delete(id);
    return identity;
  }

  /** Look up a peer's identity. */
  get(id: string): AgentIdentity | undefined {
    return this.peers.get(id);
  }

  /** Get all transport addresses for a peer. */
  getAddresses(id: string): TransportAddress[] | undefined {
    return this.addresses.get(id);
  }

  /** Get all known peers. */
  allPeers(): AgentIdentity[] {
    return [...this.peers.values()];
  }

  /** Get all known peer IDs. */
  allPeerIds(): string[] {
    return [...this.peers.keys()];
  }

  /** Number of known peers. */
  get length(): number {
    return this.peers.size;
  }

  /** Whether the table is empty. */
  get isEmpty(): boolean {
    return this.peers.size === 0;
  }

  /** Subscribe a peer to a topic. */
  subscribe(peerId: string, topic: string): void {
    let subs = this.subscriptions.get(topic);
    if (!subs) {
      subs = new Set();
      this.subscriptions.set(topic, subs);
    }
    subs.add(peerId);
  }

  /** Unsubscribe a peer from a topic. */
  unsubscribe(peerId: string, topic: string): void {
    const subs = this.subscriptions.get(topic);
    if (subs) {
      subs.delete(peerId);
      if (subs.size === 0) {
        this.subscriptions.delete(topic);
      }
    }
  }

  /** Get all peers subscribed to a topic. */
  subscribers(topic: string): string[] {
    const subs = this.subscriptions.get(topic);
    return subs ? [...subs] : [];
  }
}
