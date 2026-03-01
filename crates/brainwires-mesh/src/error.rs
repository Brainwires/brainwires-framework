use thiserror::Error;

/// Errors that can occur within the mesh networking layer.
#[derive(Debug, Clone, Error)]
pub enum MeshError {
    /// The requested node was not found in the mesh.
    #[error("node not found: {0}")]
    NodeNotFound(String),

    /// A message could not be routed to its destination.
    #[error("routing failed: {0}")]
    RoutingFailed(String),

    /// Peer discovery failed.
    #[error("discovery failed: {0}")]
    DiscoveryFailed(String),

    /// A federation request was denied by policy.
    #[error("federation denied: {0}")]
    FederationDenied(String),

    /// An error occurred while modifying the mesh topology.
    #[error("topology error: {0}")]
    TopologyError(String),

    /// A transport-level error occurred.
    #[error("transport error: {0}")]
    Transport(String),

    /// An internal or unexpected error.
    #[error("internal error: {0}")]
    Internal(String),
}
