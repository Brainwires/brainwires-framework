//! # Brainwires Brain — Central Knowledge Crate
//!
//! The canonical home for all knowledge systems in the Brainwires Agent Framework:
//!
//! - **Knowledge Systems**: BKS (behavioral truths) and PKS (personal facts)
//! - **Entity Graph**: Entity types, entity store, relationship graph
//! - **Brain Client**: Persistent thought storage with semantic search
//! - **Thought Types**: Categories, sources, and metadata
//! - **Fact Extraction**: Automatic categorization and tag extraction
//!
//! ## Library Usage
//!
//! ```no_run
//! use brainwires_brain::BrainClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = BrainClient::new().await?;
//!     Ok(())
//! }
//! ```

pub mod brain_client;
pub mod entity;
pub mod fact_extractor;
pub mod knowledge;
pub mod relationship_graph;
pub mod thought;
pub mod types;

// Re-export main types
pub use brain_client::BrainClient;
pub use entity::{
    ContradictionEvent, ContradictionKind, Entity, EntityStore, EntityStoreStats, EntityType,
    ExtractionResult, Relationship,
};
pub use relationship_graph::{EdgeType, EntityContext, GraphEdge, GraphNode, RelationshipGraph};
pub use thought::{Thought, ThoughtCategory, ThoughtSource};
pub use types::{
    CaptureThoughtRequest, CaptureThoughtResponse, DeleteThoughtRequest, DeleteThoughtResponse,
    GetThoughtRequest, GetThoughtResponse, ListRecentRequest, ListRecentResponse,
    MemoryStatsRequest, MemoryStatsResponse, SearchKnowledgeRequest, SearchKnowledgeResponse,
    SearchMemoryRequest, SearchMemoryResponse,
};
