#![deny(missing_docs)]
//! # Brainwires Reasoning
//!
//! Structured reasoning primitives for the Brainwires Agent Framework.
//!
//! Re-exports the plan and output parsing utilities from `brainwires-core`
//! as a single, discoverable reasoning crate for Layer 3 consumers.
//!
//! ## Available
//! - **plan_parser** — Extract numbered task steps from LLM plan output
//! - **output_parser** — Parse structured data (JSON, regex) from raw LLM text

// Re-export from core — canonical source of truth lives in brainwires-core
pub use brainwires_core::output_parser;
pub use brainwires_core::plan_parser;

// Flat re-exports for convenience
pub use brainwires_core::output_parser::{
    JsonListParser, JsonOutputParser, OutputParser, RegexOutputParser,
};
pub use brainwires_core::plan_parser::{ParsedStep, parse_plan_steps, steps_to_tasks};
