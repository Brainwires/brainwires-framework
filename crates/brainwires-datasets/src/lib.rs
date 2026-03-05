#![warn(missing_docs)]
//! # Brainwires Datasets
//!
//! Training data pipelines for the Brainwires Agent Framework.
//!
//! Provides JSONL I/O, tokenization, deduplication, format conversion, and
//! dataset management for cloud and local model fine-tuning workflows.

/// Error types for dataset operations.
pub mod error;
/// Core training data types (messages, examples, preference pairs).
pub mod types;
/// Dataset trait and concrete dataset implementations.
pub mod dataset;
/// JSONL reader and writer for streaming I/O.
pub mod jsonl;
/// Tokenizer abstractions and implementations.
pub mod tokenizer;
/// Data quality validation, statistics, and deduplication.
pub mod quality;
/// Format converters for various fine-tuning providers.
pub mod format;
/// Train/eval splitting, curriculum ordering, and sampling utilities.
pub mod sampling;

// Re-export core types
pub use error::{DatasetError, DatasetResult};
pub use types::{
    DataFormat, PreferencePair, TrainingExample, TrainingMessage, TrainingRole,
};
pub use dataset::{Dataset, InstructDataset, PreferenceDataset};
pub use jsonl::{JsonlReader, JsonlWriter, read_jsonl, write_jsonl};
pub use quality::{
    DataValidator, ValidatorConfig, ValidationReport, ValidationIssue, IssueSeverity,
    DatasetStats, RoleCounts, compute_stats,
};
pub use format::{
    FormatConverter, OpenAiFormat, TogetherFormat, AlpacaFormat, ShareGptFormat, ChatMlFormat,
};
pub use sampling::{SplitConfig, SplitResult, train_eval_split, curriculum_order, sample_n};

// Feature-gated re-exports
#[cfg(feature = "hf-tokenizer")]
pub use tokenizer::HfTokenizer;

#[cfg(feature = "tiktoken")]
pub use tokenizer::TiktokenTokenizer;

#[cfg(feature = "dedup")]
pub use quality::{Deduplicator, exact_dedup};

pub use tokenizer::Tokenizer;
