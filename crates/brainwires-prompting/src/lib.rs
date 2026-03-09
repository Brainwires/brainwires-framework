#![deny(missing_docs)]
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

// Knowledge systems re-exported from brainwires-brain (optional)
#[cfg(feature = "knowledge")]
pub use brainwires_brain::knowledge;

pub mod clustering;
pub mod generator;
pub mod learning;
pub mod library;
pub mod seal;
#[cfg(feature = "native")]
pub mod storage;
pub mod techniques;
pub mod temperature;

// Re-export main types
pub use clustering::{TaskCluster, TaskClusterManager, cosine_similarity};
pub use generator::{GeneratedPrompt, PromptGenerator};
pub use learning::{ClusterSummary, PromptingLearningCoordinator, TechniqueStats};
pub use library::TechniqueLibrary;
pub use seal::SealProcessingResult;
#[cfg(feature = "native")]
pub use storage::{ClusterStorage, StorageStats};
pub use techniques::{
    ComplexityLevel, PromptingTechnique, TaskCharacteristic, TechniqueCategory, TechniqueMetadata,
};
pub use temperature::{TemperatureOptimizer, TemperaturePerformance};
