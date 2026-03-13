#![deny(missing_docs)]
//! # Brainwires Cognition — Unified Intelligence Layer
//!
//! This crate consolidates three previously separate crates into a single
//! coherent intelligence layer for the Brainwires Agent Framework:
//!
//! ## Knowledge (from brainwires-brain)
//! - **BrainClient** — Persistent thought storage with semantic search
//! - **Entity/Relationship Graph** — Entity types, co-occurrence, impact analysis
//! - **BKS** — Behavioral Knowledge System (shared truths with confidence scoring)
//! - **PKS** — Personal Knowledge System (user-scoped facts)
//! - **Fact Extraction** — Automatic categorization and tag extraction
//!
//! ## Prompting (from brainwires-prompting)
//! - **Techniques** — 15 prompting techniques from the adaptive selection paper
//! - **Clustering** — K-means task clustering by semantic similarity
//! - **Generator** — Dynamic prompt generation with BKS/PKS/SEAL integration
//! - **Learning** — Technique effectiveness tracking and BKS promotion
//! - **Temperature** — Adaptive temperature optimization per cluster
//!
//! ## RAG (from brainwires-rag)
//! - **RagClient** — Core semantic code search with hybrid BM25+vector search
//! - **Embedding** — FastEmbed (all-MiniLM-L6-v2) local embedding generation
//! - **Indexer** — File walking, AST-based chunking for 12 languages
//! - **Git Search** — Semantic search over commit history
//! - **Documents** — PDF, markdown, and plaintext document processing
//!
//! ## Spectral
//! - **SpectralReranker** — MSS-inspired log-det maximization for diverse retrieval
//! - **GraphOps** — Laplacian, Fiedler vector, spectral clustering, sparsification
//! - **Kernel** — Relevance-weighted kernel matrix construction
//! - **Linalg** — Cholesky decomposition and log-determinant computation
//!
//! ## Code Analysis
//! - **RepoMap** — AST-based symbol extraction (definitions, references)
//! - **Relations** — Call graph generation, definition/reference lookup
//! - **Storage** — LanceDB persistence for code relations

// Re-export core types
pub use brainwires_core;

// ── Knowledge (from brainwires-brain) ──────────────────────────────────────

/// Knowledge graph, entities, thoughts, BKS/PKS, brain client.
#[cfg(feature = "knowledge")]
pub mod knowledge;

// ── Prompting (from brainwires-prompting) ──────────────────────────────────

/// Adaptive prompting techniques, clustering, temperature optimization.
pub mod prompting;

// ── RAG (from brainwires-rag) ──────────────────────────────────────────────

/// RAG error types.
#[cfg(feature = "rag")]
pub mod rag;

// ── Spectral math ──────────────────────────────────────────────────────────

/// Spectral graph methods: diverse retrieval, clustering, centrality, sparsification.
#[cfg(feature = "spectral")]
pub mod spectral;

// ── Code analysis ──────────────────────────────────────────────────────────

/// Code relationship extraction (definitions, references, call graphs).
#[cfg(feature = "code-analysis")]
pub mod code_analysis;

// ── Re-exports (knowledge) ─────────────────────────────────────────────────

#[cfg(feature = "knowledge")]
pub use knowledge::brain_client::BrainClient;
#[cfg(feature = "knowledge")]
pub use knowledge::entity::{
    ContradictionEvent, ContradictionKind, Entity, EntityStore, EntityStoreStats, EntityType,
    ExtractionResult, Relationship,
};
#[cfg(feature = "knowledge")]
pub use knowledge::relationship_graph::{
    EdgeType, EntityContext, GraphEdge, GraphNode, RelationshipGraph,
};
#[cfg(feature = "knowledge")]
pub use knowledge::thought::{Thought, ThoughtCategory, ThoughtSource};
#[cfg(feature = "knowledge")]
pub use knowledge::types::{
    CaptureThoughtRequest, CaptureThoughtResponse, DeleteThoughtRequest, DeleteThoughtResponse,
    GetThoughtRequest, GetThoughtResponse, ListRecentRequest, ListRecentResponse,
    MemoryStatsRequest, MemoryStatsResponse, SearchKnowledgeRequest, SearchKnowledgeResponse,
    SearchMemoryRequest, SearchMemoryResponse,
};

// ── Re-exports (prompting) ─────────────────────────────────────────────────

#[cfg(feature = "prompting")]
pub use prompting::clustering::{TaskCluster, TaskClusterManager, cosine_similarity};
#[cfg(feature = "knowledge")]
pub use prompting::generator::{GeneratedPrompt, PromptGenerator};
#[cfg(feature = "knowledge")]
pub use prompting::learning::{ClusterSummary, PromptingLearningCoordinator, TechniqueStats};
#[cfg(feature = "knowledge")]
pub use prompting::library::TechniqueLibrary;
pub use prompting::seal::SealProcessingResult;
#[cfg(feature = "prompting-storage")]
pub use prompting::storage::{ClusterStorage, StorageStats};
pub use prompting::techniques::{
    ComplexityLevel, PromptingTechnique, TaskCharacteristic, TechniqueCategory, TechniqueMetadata,
};
#[cfg(feature = "knowledge")]
pub use prompting::temperature::{TemperatureOptimizer, TemperaturePerformance};

// ── Re-exports (RAG) ──────────────────────────────────────────────────────

#[cfg(feature = "rag")]
pub use rag::client::RagClient;
#[cfg(feature = "rag")]
pub use rag::config::Config;
#[cfg(feature = "rag")]
pub use rag::error::RagError;
#[cfg(feature = "rag")]
pub use rag::types::{
    AdvancedSearchRequest, ClearRequest, ClearResponse, FindDefinitionRequest,
    FindReferencesRequest, GetCallGraphRequest, GitSearchResult, IndexRequest, IndexResponse,
    IndexingMode, LanguageStats, QueryRequest, QueryResponse, SearchGitHistoryRequest,
    SearchGitHistoryResponse, StatisticsRequest, StatisticsResponse,
};
#[cfg(all(feature = "rag", feature = "code-analysis"))]
pub use rag::types::{FindDefinitionResponse, FindReferencesResponse, GetCallGraphResponse};

// ── Re-exports (spectral) ─────────────────────────────────────────────────

#[cfg(feature = "spectral")]
pub use spectral::SpectralReranker;

// ── Re-exports (code analysis) ────────────────────────────────────────────

#[cfg(feature = "code-analysis")]
pub use code_analysis::types::{
    CallEdge, CallGraphNode, Definition, Reference, ReferenceKind, SymbolId, SymbolKind, Visibility,
};

/// Prelude for convenient imports.
pub mod prelude {
    #[cfg(feature = "knowledge")]
    pub use super::knowledge::brain_client::BrainClient;
    #[cfg(feature = "knowledge")]
    pub use super::knowledge::entity::{Entity, EntityStore, EntityType};
    #[cfg(feature = "knowledge")]
    pub use super::knowledge::thought::{Thought, ThoughtCategory};

    #[cfg(feature = "knowledge")]
    pub use super::prompting::generator::PromptGenerator;
    pub use super::prompting::techniques::{PromptingTechnique, TechniqueCategory};

    #[cfg(feature = "rag")]
    pub use super::rag::client::RagClient;
    #[cfg(feature = "rag")]
    pub use super::rag::types::{IndexRequest, QueryRequest};

    #[cfg(feature = "spectral")]
    pub use super::spectral::SpectralReranker;
}
