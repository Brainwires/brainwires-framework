//! Adaptive Prompting Techniques
//!
//! This crate implements the adaptive prompting system from
//! "Adaptive Selection of Prompting Techniques" (arXiv:2510.18162),
//! with BKS/PKS/SEAL integration for intelligent technique selection.
//!
//! Key components:
//! - **Techniques**: 15 prompting techniques from the paper
//! - **Clustering**: K-means task clustering by semantic similarity
//! - **Library**: Technique metadata with BKS integration
//! - **Generator**: Dynamic prompt generation with multi-source selection
//! - **Learning**: Technique effectiveness tracking and BKS promotion
//! - **Temperature**: Adaptive temperature optimization per cluster
//! - **Storage**: SQLite persistence for clusters and performance data

// Knowledge systems re-exported from brainwires-knowledge knowledge module (optional)
#[cfg(feature = "knowledge")]
pub use crate::knowledge::bks_pks;

/// Pure types (always available)
pub mod seal;
pub mod techniques;

/// Clustering (requires prompting feature for linfa/ndarray)
#[cfg(feature = "prompting")]
pub mod clustering;

/// generator depends on both knowledge (BKS/PKS) and prompting (clustering)
#[cfg(all(feature = "knowledge", feature = "prompting"))]
pub mod generator;
/// These modules depend on knowledge types (BKS/PKS caches)
#[cfg(feature = "knowledge")]
pub mod learning;
#[cfg(feature = "knowledge")]
pub mod library;
/// temperature depends on both knowledge (BKS types) and prompting (clustering)
#[cfg(all(feature = "knowledge", feature = "prompting"))]
pub mod temperature;

/// SQLite persistence
#[cfg(feature = "prompting-storage")]
pub mod storage;

// Re-export main types
pub use seal::SealProcessingResult;
pub use techniques::{
    ComplexityLevel, PromptingTechnique, TaskCharacteristic, TechniqueCategory, TechniqueMetadata,
};

#[cfg(feature = "prompting")]
pub use clustering::{TaskCluster, TaskClusterManager, cosine_similarity};
#[cfg(all(feature = "knowledge", feature = "prompting"))]
pub use generator::{GeneratedPrompt, PromptGenerator};
#[cfg(feature = "knowledge")]
pub use learning::{ClusterSummary, PromptingLearningCoordinator, TechniqueStats};
#[cfg(feature = "knowledge")]
pub use library::TechniqueLibrary;
#[cfg(feature = "prompting-storage")]
pub use storage::{ClusterStorage, StorageStats};
#[cfg(all(feature = "knowledge", feature = "prompting"))]
pub use temperature::{TemperatureOptimizer, TemperaturePerformance};
