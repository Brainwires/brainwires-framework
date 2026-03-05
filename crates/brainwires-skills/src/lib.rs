#![warn(missing_docs)]
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
pub mod metadata;
pub mod parser;
pub mod registry;
pub mod router;

pub use executor::{ScriptPrepared, SkillExecutor, SubagentPrepared};
pub use metadata::{
    MatchSource, Skill, SkillExecutionMode, SkillMatch, SkillMetadata, SkillResult, SkillSource,
};
pub use parser::{parse_skill_file, parse_skill_metadata, render_template};
pub use registry::SkillRegistry;
pub use router::SkillRouter;
