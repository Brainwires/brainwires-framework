use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MeshError;
use crate::node::MeshNode;

/// Supported mesh topology shapes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyType {
    /// Central coordinator with spoke nodes.
    Star,
    /// Circular ring where each node connects to the next.
    Ring,
    /// Every node connects to every other node.
    FullMesh,
    /// Tree-like structure with parent/child relationships.
    Hierarchical,
    /// User-defined topology with explicit adjacency.
    Custom(String),
}

/// Trait for managing the shape of the agent mesh.
#[async_trait]
pub trait MeshTopology: Send + Sync {
    /// Add a node to the topology.
    async fn add_node(&mut self, node: MeshNode) -> Result<(), MeshError>;

    /// Remove a node from the topology by its identifier.
    async fn remove_node(&mut self, node_id: &Uuid) -> Result<(), MeshError>;

    /// Return the identifiers of nodes adjacent to the given node.
    async fn get_neighbors(&self, node_id: &Uuid) -> Result<Vec<Uuid>, MeshError>;

    /// Return the type of this topology.
    fn topology_type(&self) -> TopologyType;
}
