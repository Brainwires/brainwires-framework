//! Attention mechanism — context focus and relevance scoring.
//!
//! Uses RAG integration to determine which parts of the codebase
//! are most relevant to the current task.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Relevance score for a code chunk or file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Path to the file or chunk.
    pub path: String,
    /// Relevance score (0.0 to 1.0).
    pub score: f64,
    /// Why this is relevant.
    pub reason: String,
}

/// Attention window — the subset of the codebase to focus on.
#[derive(Debug, Clone, Default)]
pub struct AttentionWindow {
    /// Files ranked by relevance.
    pub ranked_files: Vec<RelevanceScore>,
    /// Maximum number of files to include in context.
    pub max_files: usize,
    /// Maximum total tokens for the attention window.
    pub max_tokens: usize,
}

impl AttentionWindow {
    pub fn new(max_files: usize, max_tokens: usize) -> Self {
        Self {
            ranked_files: Vec::new(),
            max_files,
            max_tokens,
        }
    }

    /// Get the top-N most relevant files.
    pub fn top_files(&self, n: usize) -> Vec<&RelevanceScore> {
        self.ranked_files.iter().take(n.min(self.max_files)).collect()
    }

    /// Add a relevance score, maintaining sorted order (highest first).
    pub fn add(&mut self, score: RelevanceScore) {
        let pos = self.ranked_files
            .binary_search_by(|s| s.score.partial_cmp(&score.score).unwrap_or(std::cmp::Ordering::Equal).reverse())
            .unwrap_or_else(|p| p);
        self.ranked_files.insert(pos, score);
    }
}

/// Attention mechanism that focuses agent context on relevant code.
pub struct AttentionMechanism {
    /// Cache of previous attention computations.
    cache: HashMap<String, AttentionWindow>,
    /// Default attention window configuration.
    default_max_files: usize,
    default_max_tokens: usize,
}

impl AttentionMechanism {
    pub fn new(max_files: usize, max_tokens: usize) -> Self {
        Self {
            cache: HashMap::new(),
            default_max_files: max_files,
            default_max_tokens: max_tokens,
        }
    }

    /// Compute attention for a task description, returning relevant files.
    ///
    /// This is a framework-level method. Integration with RAG (brainwires-rag)
    /// is done at the application level by calling `query_codebase` and feeding
    /// results into `from_search_results`.
    pub fn from_search_results(
        &mut self,
        task_id: &str,
        results: Vec<(String, f64, String)>,
    ) -> &AttentionWindow {
        let mut window = AttentionWindow::new(self.default_max_files, self.default_max_tokens);

        for (path, score, reason) in results {
            window.add(RelevanceScore { path, score, reason });
        }

        self.cache.insert(task_id.to_string(), window);
        self.cache.get(task_id).unwrap()
    }

    /// Get a cached attention window for a task.
    pub fn get(&self, task_id: &str) -> Option<&AttentionWindow> {
        self.cache.get(task_id)
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
