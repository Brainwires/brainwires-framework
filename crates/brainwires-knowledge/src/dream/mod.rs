//! Autodream memory consolidation — offline summarisation and fact extraction.
//!
//! The dream system runs periodically (via cron or on-demand) to compress older
//! conversation messages into summaries and durable facts, reducing token
//! pressure while preserving important knowledge.

pub mod consolidator;
pub mod fact_extractor;
pub mod metrics;
pub mod policy;
pub mod summarizer;
pub mod task;
