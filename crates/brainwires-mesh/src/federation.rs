use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MeshError;
use crate::node::MeshNode;

/// Policy governing which peers may join a federated mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationPolicy {
    /// Any peer may join.
    Open,
    /// Only explicitly listed peers may join.
    AllowList(Vec<Uuid>),
    /// All peers except those listed may join.
    DenyList(Vec<Uuid>),
    /// Peers are admitted based on required capabilities.
    CapabilityBased(Vec<String>),
}

/// Trait for managing federation between mesh clusters.
#[async_trait]
pub trait FederationGateway: Send + Sync {
    /// Evaluate and optionally accept a peer into the federation.
    async fn accept_peer(&mut self, peer: &MeshNode) -> Result<bool, MeshError>;

    /// Return the current federation policy.
    fn policy(&self) -> &FederationPolicy;

    /// List the identifiers of all currently federated peers.
    fn list_federated_peers(&self) -> Vec<Uuid>;
}
