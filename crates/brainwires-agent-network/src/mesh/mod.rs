//! Distributed agent mesh networking.
//!
//! **Note:** The networking stack has been reorganized into a 5-layer
//! protocol stack. Many types from this module have equivalents in the
//! new layer modules:
//!
//! | Old (mesh) | New (layer) |
//! |---|---|
//! | `MeshNode` | [`identity::AgentIdentity`](crate::identity::AgentIdentity) |
//! | `NodeCapabilities` | [`identity::AgentCard`](crate::identity::AgentCard) |
//! | `MeshError` | [`network::NetworkError`](crate::network::NetworkError) |
//! | `PeerDiscovery` | [`discovery::Discovery`](crate::discovery::Discovery) |
//! | `MessageRouter` | [`routing::Router`](crate::routing::Router) |
//! | `RoutingStrategy` | [`routing::RoutingStrategy`](crate::routing::RoutingStrategy) |
//!
//! Types that remain unique to the mesh module:
//! - [`FederationGateway`] / [`FederationPolicy`] — cross-mesh federation
//! - [`MeshTopology`] / [`TopologyType`] — mesh topology management

/// Peer discovery protocols for locating nodes in the mesh.
#[deprecated(
    since = "0.4.0",
    note = "Use `crate::discovery::Discovery` instead"
)]
pub mod discovery;
/// Error types for mesh operations.
#[deprecated(
    since = "0.4.0",
    note = "Use `crate::network::NetworkError` instead"
)]
pub mod error;
/// Federation gateways and policies for cross-mesh communication.
pub mod federation;
/// Mesh node definitions and capability tracking.
#[deprecated(
    since = "0.4.0",
    note = "Use `crate::identity::AgentIdentity` and `AgentCard` instead"
)]
pub mod node;
/// Message routing strategies and route tables.
#[deprecated(
    since = "0.4.0",
    note = "Use `crate::routing::Router` instead"
)]
pub mod routing;
/// Mesh topology management and layout types.
pub mod topology;

// ---- Re-exports ----

#[allow(deprecated)]
pub use discovery::{DiscoveryProtocol, PeerDiscovery};
#[allow(deprecated)]
pub use error::MeshError;
pub use federation::{FederationGateway, FederationPolicy};
#[allow(deprecated)]
pub use node::{MeshNode, NodeCapabilities, NodeState};
#[allow(deprecated)]
pub use routing::{MessageRouter, RouteEntry, RoutingStrategy};
pub use topology::{MeshTopology, TopologyType};
