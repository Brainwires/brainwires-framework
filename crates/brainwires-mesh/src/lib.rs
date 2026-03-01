//! # brainwires-mesh
//!
//! Distributed agent mesh networking for the Brainwires Agent Framework.
//!
//! This crate provides the building blocks for connecting agents across
//! multiple nodes into a coordinated mesh network. It defines topology
//! management, message routing, peer discovery, node lifecycle tracking,
//! and federation policies.

pub mod discovery;
pub mod error;
pub mod federation;
pub mod node;
pub mod routing;
pub mod topology;

// ---- Re-exports ----

pub use discovery::{DiscoveryProtocol, PeerDiscovery};
pub use error::MeshError;
pub use federation::{FederationGateway, FederationPolicy};
pub use node::{MeshNode, NodeCapabilities, NodeState};
pub use routing::{MessageRouter, RouteEntry, RoutingStrategy};
pub use topology::{MeshTopology, TopologyType};
