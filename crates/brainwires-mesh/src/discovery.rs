use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::MeshError;
use crate::node::MeshNode;

/// Protocol used for discovering peers in the mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoveryProtocol {
    /// Multicast DNS for local-network discovery.
    Mdns,
    /// Gossip-based protocol for decentralized peer exchange.
    Gossip,
    /// Centralized registry service.
    Registry,
    /// Manually configured peer list.
    Manual,
}

/// Trait for peer discovery within the mesh.
#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    /// Discover available peers using the configured protocol.
    async fn discover_peers(&self) -> Result<Vec<MeshNode>, MeshError>;

    /// Register this node so it can be discovered by others.
    fn register_self(&mut self, node: MeshNode) -> Result<(), MeshError>;

    /// Remove this node from the discovery mechanism.
    fn deregister(&mut self) -> Result<(), MeshError>;
}
