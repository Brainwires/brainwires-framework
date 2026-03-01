use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The current state of a mesh node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is starting up and not yet ready.
    Initializing,
    /// Node is active and accepting work.
    Active,
    /// Node is draining in-flight tasks before shutdown.
    Draining,
    /// Node has lost connectivity.
    Disconnected,
    /// Node has encountered an unrecoverable failure.
    Failed,
}

/// Capabilities advertised by a mesh node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Maximum number of tasks the node can run concurrently.
    pub max_concurrent_tasks: usize,
    /// Protocol identifiers the node supports (e.g. "a2a", "mcp").
    pub supported_protocols: Vec<String>,
    /// Tool names available on this node.
    pub available_tools: Vec<String>,
    /// Abstract compute capacity score (higher is more powerful).
    pub compute_capacity: f64,
}

/// A single node within the agent mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshNode {
    /// Unique identifier for this node.
    pub id: Uuid,
    /// Network address (e.g. "host:port" or URI).
    pub address: String,
    /// Current lifecycle state.
    pub state: NodeState,
    /// Advertised capabilities.
    pub capabilities: NodeCapabilities,
    /// Last time this node was seen (ISO-8601 timestamp).
    pub last_seen: String,
    /// Arbitrary metadata attached to the node.
    pub metadata: HashMap<String, String>,
}
