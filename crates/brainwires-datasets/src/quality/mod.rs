/// Dataset validation rules and reporting.
pub mod validator;
/// Dataset statistics computation.
pub mod stats;

/// MinHash-based and exact deduplication.
#[cfg(feature = "dedup")]
pub mod dedup;

pub use validator::{DataValidator, ValidatorConfig, ValidationReport, ValidationIssue, IssueSeverity};
pub use stats::{DatasetStats, RoleCounts, compute_stats};

#[cfg(feature = "dedup")]
pub use dedup::{Deduplicator, exact_dedup};
