#![deny(missing_docs)]
//! # Brainwires Reasoning
//!
//! Structured reasoning primitives for the Brainwires Agent Framework.
//!
//! ## Always Available
//! - **plan_parser** — Extract numbered task steps from LLM plan output
//! - **output_parser** — Parse structured data (JSON, regex) from raw LLM text
//!
//! These modules were previously part of `brainwires-core`. Re-exports from
//! core are kept for one release cycle to avoid breaking changes.

/// Plan parsing — extract numbered task steps from LLM plan output.
pub mod plan_parser;

/// Output parsing — extract structured data from raw LLM text.
pub mod output_parser;

// Re-exports for convenience
pub use output_parser::{JsonListParser, JsonOutputParser, OutputParser, RegexOutputParser};
pub use plan_parser::{ParsedStep, parse_plan_steps, steps_to_tasks};
