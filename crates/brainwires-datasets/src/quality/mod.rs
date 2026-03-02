pub mod validator;
pub mod stats;

#[cfg(feature = "dedup")]
pub mod dedup;

pub use validator::{DataValidator, ValidatorConfig, ValidationReport, ValidationIssue, IssueSeverity};
pub use stats::{DatasetStats, RoleCounts, compute_stats};

#[cfg(feature = "dedup")]
pub use dedup::{Deduplicator, exact_dedup};
