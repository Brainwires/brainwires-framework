/**
 * @module discovery
 *
 * Peer discovery: how agents find each other on the network.
 * Equivalent to Rust's `Discovery`, `ManualDiscovery`, `DiscoveryProtocol`.
 */

import type { AgentIdentity } from "./identity.ts";

/**
 * Discovery protocol identifier.
 * Equivalent to Rust `DiscoveryProtocol`.
 */
export type DiscoveryProtocol =
  | { type: "manual" }
  | { type: "registry" }
  | { type: "mdns" }
  | { type: "gossip" }
  | { type: "custom"; name: string };

/**
 * The core discovery interface.
 * Equivalent to Rust `Discovery` trait.
 */
export interface Discovery {
  /** Register this agent's identity with the discovery service. */
  register(identity: AgentIdentity): Promise<void>;

  /** Deregister this agent from the discovery service. */
  deregister(id: string): Promise<void>;

  /** Discover all currently known/reachable peers. */
  discover(): Promise<AgentIdentity[]>;

  /** Look up a specific agent by UUID. */
  lookup(id: string): Promise<AgentIdentity | undefined>;

  /** The discovery protocol this implementation uses. */
  protocol(): DiscoveryProtocol;
}

/**
 * Manual peer discovery backed by an in-memory peer list.
 * Peers are added and removed explicitly.
 * Equivalent to Rust `ManualDiscovery`.
 */
export class ManualDiscovery implements Discovery {
  private peers: Map<string, AgentIdentity> = new Map();

  /** Create a manual discovery pre-populated with peers. */
  static withPeers(peers: AgentIdentity[]): ManualDiscovery {
    const discovery = new ManualDiscovery();
    for (const p of peers) {
      discovery.peers.set(p.id, p);
    }
    return discovery;
  }

  /** Explicitly add a peer. */
  addPeer(identity: AgentIdentity): void {
    this.peers.set(identity.id, identity);
  }

  /** Explicitly remove a peer. */
  removePeer(id: string): AgentIdentity | undefined {
    const identity = this.peers.get(id);
    this.peers.delete(id);
    return identity;
  }

  // deno-lint-ignore require-await
  async register(identity: AgentIdentity): Promise<void> {
    this.peers.set(identity.id, identity);
  }

  // deno-lint-ignore require-await
  async deregister(id: string): Promise<void> {
    this.peers.delete(id);
  }

  // deno-lint-ignore require-await
  async discover(): Promise<AgentIdentity[]> {
    return [...this.peers.values()];
  }

  // deno-lint-ignore require-await
  async lookup(id: string): Promise<AgentIdentity | undefined> {
    return this.peers.get(id);
  }

  protocol(): DiscoveryProtocol {
    return { type: "manual" };
  }
}
