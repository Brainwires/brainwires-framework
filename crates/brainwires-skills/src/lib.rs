#![deny(missing_docs)]
//! # brainwires-skills
//!
//! Agent skills system — SKILL.md parsing, registry, routing, and execution.
//!
//! Skills are markdown-based packages that extend agent capabilities using
//! progressive disclosure:
//! - At startup: only metadata (name, description) is loaded for fast matching
//! - On activation: full SKILL.md content is loaded on-demand
//!
//! ## SKILL.md Format
//!
//! ```markdown
//! ---
//! name: review-pr
//! description: Reviews pull requests for code quality and security issues.
//! allowed-tools:
//!   - Read
//!   - Grep
//! model: claude-sonnet-4
//! metadata:
//!   category: code-review
//!   execution: subagent
//! ---
//!
//! # PR Review Instructions
//! ...
//! ```

pub mod executor;
pub mod manifest;
pub mod metadata;
pub mod package;
pub mod parser;
pub mod registry;
#[cfg(feature = "registry")]
pub mod registry_client;
pub mod router;

pub use executor::{ScriptPrepared, SkillExecutor, SubagentPrepared};
pub use manifest::{SkillDependency, SkillManifest};
pub use metadata::{
    MatchSource, Skill, SkillExecutionMode, SkillMatch, SkillMetadata, SkillResult, SkillSource,
};
pub use package::SkillPackage;
pub use parser::{parse_skill_file, parse_skill_metadata, render_template};
pub use registry::SkillRegistry;
#[cfg(feature = "registry")]
pub use registry_client::RegistryClient;
pub use router::SkillRouter;
