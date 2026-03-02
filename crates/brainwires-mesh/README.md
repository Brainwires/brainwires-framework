# brainwires-mesh

[![Crates.io](https://img.shields.io/crates/v/brainwires-mesh.svg)](https://crates.io/crates/brainwires-mesh)
[![Documentation](https://img.shields.io/docsrs/brainwires-mesh)](https://docs.rs/brainwires-mesh)
[![License](https://img.shields.io/crates/l/brainwires-mesh.svg)](LICENSE)

Distributed agent mesh networking for the Brainwires Agent Framework.

## Overview

`brainwires-mesh` provides the building blocks for connecting agents into a coordinated mesh network. Nodes discover each other, organize into topologies, and route messages through configurable strategies — enabling multi-node agent coordination across processes, machines, or data centers.

**Design principles:**

- **Topology-aware** — supports star, ring, full-mesh, and hierarchical layouts with automatic neighbor management
- **Pluggable discovery** — mDNS, static seed lists, or custom discovery protocols
- **Strategy-based routing** — direct, broadcast, round-robin, and content-based routing via `RoutingStrategy`
- **Federation-ready** — `FederationGateway` bridges separate meshes with configurable trust policies
- **A2A integration** — optional interop with the A2A protocol for cross-framework agent communication

```text
  ┌──────────────────────────────────────────────────────────┐
  │                     brainwires-mesh                       │
  │                                                          │
  │  ┌────────────┐     ┌──────────────┐     ┌───────────┐  │
  │  │  MeshNode  │────▶│ MeshTopology │────▶│  Message   │  │
  │  │ Capabilities│    │ TopologyType │     │  Router    │  │
  │  │ NodeState  │     │ Neighbors    │     │ RouteEntry │  │
  │  └────────────┘     └──────────────┘     └───────────┘  │
  │         │                                      │         │
  │         ▼                                      ▼         │
  │  ┌────────────┐                       ┌──────────────┐   │
  │  │   Peer     │                       │ Federation   │   │
  │  │ Discovery  │                       │  Gateway     │   │
  │  │ Protocol   │                       │  Policy      │   │
  │  └────────────┘                       └──────────────┘   │
  └──────────────────────────────────────────────────────────┘

  Flow: Nodes → Topology → Router → Discovery + Federation
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-mesh = "0.1"
```

Create a mesh node and join a topology:

```rust
use brainwires_mesh::{
    MeshNode, NodeCapabilities, NodeState,
    MeshTopology, TopologyType,
    MessageRouter, RoutingStrategy,
    PeerDiscovery, DiscoveryProtocol,
};

// Create a node with declared capabilities
let node = MeshNode::new(
    "node-alpha",
    NodeCapabilities {
        max_concurrent_tasks: 10,
        supported_skills: vec!["code-review".into(), "testing".into()],
        ..Default::default()
    },
);

// Build a topology
let mut topology = MeshTopology::new(TopologyType::FullMesh);
topology.add_node(node);

// Create a router with a strategy
let router = MessageRouter::new(RoutingStrategy::RoundRobin);
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `a2a` | Yes | Enables A2A protocol integration via `brainwires-a2a` for cross-framework agent discovery and task delegation |

```toml
# Without A2A integration
[dependencies]
brainwires-mesh = { version = "0.1", default-features = false }
```

## Architecture

### MeshNode

A node represents a single participant in the mesh. Each node declares its capabilities and tracks its lifecycle state.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique node identifier |
| `capabilities` | `NodeCapabilities` | Concurrent task limit, supported skills, resource tags |
| `state` | `NodeState` | Current lifecycle state |

**Node states:**

| State | Description |
|-------|-------------|
| `Initializing` | Node is starting up |
| `Ready` | Node is available for tasks |
| `Busy` | Node is at capacity |
| `Draining` | Node is finishing current work before shutdown |
| `Offline` | Node is unreachable |

### MeshTopology

Topologies define how nodes are connected and which neighbors each node can communicate with directly.

| Topology | Description |
|----------|-------------|
| `Star` | Central hub with spoke nodes — low latency, single point of coordination |
| `Ring` | Each node connects to two neighbors — simple, predictable routing |
| `FullMesh` | Every node connects to every other — maximum redundancy, O(n^2) connections |
| `Hierarchical` | Tree structure with parent/child relationships — good for large deployments |

### MessageRouter

Routes messages between nodes using configurable strategies.

| Strategy | Description |
|----------|-------------|
| `Direct` | Route to a specific node by ID |
| `Broadcast` | Send to all nodes in the topology |
| `RoundRobin` | Distribute evenly across available nodes |
| `ContentBased` | Route based on message content and node capabilities |

### PeerDiscovery

Handles automatic discovery of new nodes joining the mesh.

| Protocol | Description |
|----------|-------------|
| `Static` | Fixed list of seed node addresses |
| `Mdns` | Multicast DNS for local network discovery |
| `Custom` | User-provided discovery implementation |

### FederationGateway

Bridges separate mesh networks with trust and routing policies.

| Field | Type | Description |
|-------|------|-------------|
| `gateway_id` | `String` | Identifier for this gateway |
| `policy` | `FederationPolicy` | Trust level, allowed skills, rate limits |
| `remote_meshes` | `Vec<String>` | Connected remote mesh endpoints |

## Usage Examples

### Building a Star Topology

```rust
use brainwires_mesh::{MeshNode, MeshTopology, TopologyType, NodeCapabilities};

let mut topology = MeshTopology::new(TopologyType::Star);

// Hub node
let hub = MeshNode::new("hub", NodeCapabilities {
    max_concurrent_tasks: 50,
    supported_skills: vec!["orchestration".into()],
    ..Default::default()
});
topology.add_node(hub);

// Worker nodes
for i in 0..5 {
    let worker = MeshNode::new(
        &format!("worker-{i}"),
        NodeCapabilities {
            max_concurrent_tasks: 10,
            supported_skills: vec!["code-gen".into(), "testing".into()],
            ..Default::default()
        },
    );
    topology.add_node(worker);
}
```

### Content-Based Routing

```rust
use brainwires_mesh::{MessageRouter, RoutingStrategy, RouteEntry};

let router = MessageRouter::new(RoutingStrategy::ContentBased);

// Messages tagged "security" route to the security-review node
router.add_route(RouteEntry {
    pattern: "security".into(),
    target_node: "security-reviewer".into(),
    priority: 10,
});

// Messages tagged "performance" route to the benchmark node
router.add_route(RouteEntry {
    pattern: "performance".into(),
    target_node: "benchmark-runner".into(),
    priority: 5,
});
```

### Federation Between Meshes

```rust
use brainwires_mesh::{FederationGateway, FederationPolicy};

let gateway = FederationGateway::new(
    "gateway-east",
    FederationPolicy {
        trust_level: "verified".into(),
        allowed_skills: vec!["code-review".into()],
        max_forwarded_tasks: 100,
        ..Default::default()
    },
);

// Connect to a remote mesh
gateway.connect("https://mesh-west.example.com").await?;
```

### Peer Discovery with Static Seeds

```rust
use brainwires_mesh::{PeerDiscovery, DiscoveryProtocol};

let discovery = PeerDiscovery::new(DiscoveryProtocol::Static {
    seeds: vec![
        "192.168.1.10:9090".into(),
        "192.168.1.11:9090".into(),
    ],
});

// Start discovery — new nodes are added to the topology automatically
discovery.start(&mut topology).await?;
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["mesh"] }
```

Or depend on `brainwires-mesh` directly for standalone mesh networking without the rest of the framework.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
