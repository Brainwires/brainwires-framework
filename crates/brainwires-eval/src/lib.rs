//! # brainwires-eval
//!
//! Evaluation framework for Brainwires agents.
//!
//! ## What's included
//!
//! | Module | Key type | Purpose |
//! |--------|----------|---------|
//! | [`trial`] | [`TrialResult`], [`EvaluationStats`] | Per-trial results + Wilson-score 95 % CI |
//! | [`case`] | [`EvaluationCase`] | Trait for a single evaluatable scenario |
//! | [`suite`] | [`EvaluationSuite`], [`SuiteResult`] | N-trial Monte Carlo runner |
//! | [`recorder`] | [`ToolSequenceRecorder`], [`SequenceDiff`] | Record + diff tool call sequences |
//! | [`adversarial`] | [`AdversarialTestCase`] | Prompt injection, ambiguity, budget stress |
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use brainwires_eval::{
//!     EvaluationSuite, AlwaysPassCase, AdversarialTestCase, ToolSequenceRecorder,
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Run 30 trials of a smoke-test case
//!     let suite = EvaluationSuite::new(30);
//!     let case = Arc::new(AlwaysPassCase::new("smoke"));
//!     let results = suite.run_suite(&[case]).await;
//!
//!     let stats = &results.stats["smoke"];
//!     println!(
//!         "success={:.1}% CI=[{:.3}, {:.3}]",
//!         stats.success_rate * 100.0,
//!         stats.confidence_interval_95.lower,
//!         stats.confidence_interval_95.upper,
//!     );
//!
//!     // Record tool calls and diff against expected sequence
//!     let recorder = ToolSequenceRecorder::new();
//!     recorder.record("read_file", &serde_json::json!({"path": "main.rs"}));
//!     let diff = recorder.diff_against(&["read_file"]);
//!     assert!(diff.is_exact_match());
//!
//!     // Standard adversarial test cases
//!     let adversarial = brainwires_eval::adversarial::standard_adversarial_suite();
//!     println!("{} adversarial cases loaded", adversarial.len());
//! }
//! ```

pub mod trial;
pub mod case;
pub mod suite;
pub mod recorder;
pub mod adversarial;
pub mod regression;
pub mod stability_tests;

// ── Top-level re-exports ──────────────────────────────────────────────────────

// Trial types
pub use trial::{ConfidenceInterval95, EvaluationStats, TrialResult};

// Case trait + built-in helpers
pub use case::{AlwaysFailCase, AlwaysPassCase, EvaluationCase, StochasticCase};

// Suite types
pub use suite::{EvaluationSuite, SuiteConfig, SuiteResult};

// Recorder
pub use recorder::{SequenceDiff, ToolCallRecord, ToolSequenceRecorder};

// Adversarial
pub use adversarial::{AdversarialTestCase, AdversarialTestType};

// Regression suite
pub use regression::{CategoryBaseline, CategoryRegressionResult, RegressionConfig, RegressionResult, RegressionSuite};

// Stability tests
pub use stability_tests::{GoalPreservationCase, LoopDetectionSimCase, long_horizon_stability_suite};
