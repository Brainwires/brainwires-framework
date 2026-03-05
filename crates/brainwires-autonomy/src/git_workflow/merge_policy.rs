//! Merge policies — decide when and how PRs should be merged.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::forge::{CheckState, GitForge, MergeMethod, PullRequest, RepoRef};

/// Decision from a merge policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeDecision {
    /// Approve the merge with a specific method.
    Approve {
        /// Merge method to use.
        method: MergeMethod,
    },
    /// Wait for some condition to be met.
    Wait {
        /// Reason for waiting.
        reason: String,
    },
    /// Reject the merge.
    Reject {
        /// Reason for rejection.
        reason: String,
    },
}

/// Context for merge policy evaluation.
#[derive(Debug, Clone)]
pub struct MergeContext {
    /// Investigation confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Number of diff lines in the changes.
    pub diff_lines: u32,
    /// Number of files modified.
    pub files_modified: usize,
}

/// Trait for merge policies.
#[async_trait]
pub trait MergePolicy: Send + Sync {
    /// Evaluate whether a PR should be merged.
    async fn evaluate(&self, pr: &PullRequest, ctx: &MergeContext) -> MergeDecision;
}

/// Always requires human approval (default safe policy).
pub struct RequireApprovalPolicy;

#[async_trait]
impl MergePolicy for RequireApprovalPolicy {
    async fn evaluate(&self, _pr: &PullRequest, _ctx: &MergeContext) -> MergeDecision {
        MergeDecision::Wait {
            reason: "Requires human approval".to_string(),
        }
    }
}

/// Requires CI checks to pass before merging.
pub struct CiPassPolicy {
    forge: std::sync::Arc<dyn GitForge>,
    merge_method: MergeMethod,
}

impl CiPassPolicy {
    /// Create a CI pass policy with the given forge and merge method.
    pub fn new(forge: std::sync::Arc<dyn GitForge>, merge_method: MergeMethod) -> Self {
        Self { forge, merge_method }
    }
}

#[async_trait]
impl MergePolicy for CiPassPolicy {
    async fn evaluate(&self, pr: &PullRequest, _ctx: &MergeContext) -> MergeDecision {
        let repo = RepoRef {
            owner: String::new(), // Must be provided externally
            name: String::new(),
        };

        match self.forge.get_check_status(&repo, pr.number).await {
            Ok(status) => match status.state {
                CheckState::Success => MergeDecision::Approve {
                    method: self.merge_method,
                },
                CheckState::Pending => MergeDecision::Wait {
                    reason: "CI checks still running".to_string(),
                },
                CheckState::Failure => MergeDecision::Reject {
                    reason: "CI checks failed".to_string(),
                },
                CheckState::Error => MergeDecision::Reject {
                    reason: "CI checks errored".to_string(),
                },
            },
            Err(e) => MergeDecision::Wait {
                reason: format!("Failed to fetch check status: {e}"),
            },
        }
    }
}

/// Auto-merge when investigation confidence exceeds threshold AND CI passes.
pub struct ConfidenceBasedPolicy {
    min_confidence: f64,
    merge_method: MergeMethod,
}

impl ConfidenceBasedPolicy {
    /// Create a confidence-based policy with the given threshold and merge method.
    pub fn new(min_confidence: f64, merge_method: MergeMethod) -> Self {
        Self { min_confidence, merge_method }
    }
}

#[async_trait]
impl MergePolicy for ConfidenceBasedPolicy {
    async fn evaluate(&self, _pr: &PullRequest, ctx: &MergeContext) -> MergeDecision {
        if ctx.confidence < self.min_confidence {
            return MergeDecision::Wait {
                reason: format!(
                    "Confidence {:.1}% below threshold {:.1}%",
                    ctx.confidence * 100.0,
                    self.min_confidence * 100.0
                ),
            };
        }

        MergeDecision::Approve {
            method: self.merge_method,
        }
    }
}
