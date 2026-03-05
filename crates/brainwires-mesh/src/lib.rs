#![warn(missing_docs)]
//! # brainwires-mesh
//!
//! Distributed agent mesh networking for the Brainwires Agent Framework.
//!
//! This crate provides the building blocks for connecting agents across
//! multiple nodes into a coordinated mesh network. It defines topology
//! management, message routing, peer discovery, node lifecycle tracking,
//! and federation policies.

/// Peer discovery protocols for locating nodes in the mesh.
pub mod discovery;
/// Error types for mesh operations.
pub mod error;
/// Federation gateways and policies for cross-mesh communication.
pub mod federation;
/// Mesh node definitions and capability tracking.
pub mod node;
/// Message routing strategies and route tables.
pub mod routing;
/// Mesh topology management and layout types.
pub mod topology;

// ---- Re-exports ----

pub use discovery::{DiscoveryProtocol, PeerDiscovery};
pub use error::MeshError;
pub use federation::{FederationGateway, FederationPolicy};
pub use node::{MeshNode, NodeCapabilities, NodeState};
pub use routing::{MessageRouter, RouteEntry, RoutingStrategy};
pub use topology::{MeshTopology, TopologyType};
