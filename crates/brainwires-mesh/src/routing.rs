use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MeshError;

/// Strategy used to route messages between mesh nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Send directly to a specific node.
    DirectRoute,
    /// Use the shortest path through the mesh.
    ShortestPath,
    /// Distribute messages across nodes to balance load.
    LoadBalanced,
    /// Send to all nodes in the mesh.
    Broadcast,
    /// Send to a specific subset of nodes.
    Multicast(Vec<Uuid>),
}

/// A single entry in the routing table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    /// Final destination node.
    pub destination: Uuid,
    /// Next hop on the path to the destination.
    pub next_hop: Uuid,
    /// Routing cost / metric for this path.
    pub cost: f64,
    /// Time-to-live (max hops remaining).
    pub ttl: u32,
}

/// Trait for routing messages through the mesh.
#[async_trait]
pub trait MessageRouter: Send + Sync {
    /// Route a serialized message to the given destination using the specified strategy.
    async fn route_message(
        &self,
        destination: &Uuid,
        payload: &[u8],
        strategy: &RoutingStrategy,
    ) -> Result<(), MeshError>;

    /// Return the current routing table.
    fn get_route_table(&self) -> Vec<RouteEntry>;
}
