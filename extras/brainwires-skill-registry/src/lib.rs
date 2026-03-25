//! # brainwires-skill-registry
//!
//! Skill Marketplace registry server.
//!
//! Provides an HTTP API for publishing, searching, and downloading
//! distributable skill packages backed by SQLite storage with FTS5
//! full-text search.

pub mod api;
pub mod search;
pub mod storage;
