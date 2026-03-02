//! Automated Git workflow pipeline — issue to PR to merge.
//!
//! Provides the full pipeline: trigger → investigate → branch → fix → PR → merge.

pub mod forge;
pub mod trigger;
pub mod investigator;
pub mod branch_manager;
pub mod change_maker;
pub mod pr_manager;
pub mod merge_policy;
pub mod pipeline;

#[cfg(feature = "webhook")]
pub mod webhook;

pub use forge::{GitForge, Issue, PullRequest, CheckStatus, MergeMethod, CreatePrParams};
pub use trigger::{WorkflowTrigger, WorkflowEvent, ProgrammaticTrigger};
pub use investigator::{IssueInvestigator, InvestigationResult};
pub use branch_manager::BranchManager;
pub use change_maker::ChangeMaker;
pub use pr_manager::PullRequestManager;
pub use merge_policy::{MergePolicy, MergeDecision};
pub use pipeline::GitWorkflowPipeline;

#[cfg(feature = "webhook")]
pub use webhook::WebhookServer;
