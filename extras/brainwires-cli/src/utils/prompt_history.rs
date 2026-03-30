//! Prompt History Management
//!
//! Stores user input prompts for navigation (up/down arrows) and search (Ctrl+R).
//! Similar to bash history functionality.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::PlatformPaths;

/// Maximum number of prompts to keep in history
const MAX_HISTORY_SIZE: usize = 1000;

/// Manages user prompt history
pub struct PromptHistory {
    /// List of user prompts (newest last)
    prompts: Vec<String>,
    /// Current position in history (for up/down navigation)
    current_index: Option<usize>,
    /// Path to history file
    history_file: PathBuf,
    /// Maximum history size
    max_size: usize,
}

impl PromptHistory {
    /// Create a new prompt history manager
    pub fn new() -> Result<Self> {
        let history_file = PlatformPaths::config_dir()?.join("prompt_history.txt");
        let mut history = Self {
            prompts: Vec::new(),
            current_index: None,
            history_file,
            max_size: MAX_HISTORY_SIZE,
        };
        history.load()?;
        Ok(history)
    }

    /// Load history from disk
    fn load(&mut self) -> Result<()> {
        if !self.history_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.history_file)
            .context("Failed to read prompt history file")?;

        self.prompts = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        // Trim to max size
        if self.prompts.len() > self.max_size {
            self.prompts.drain(0..self.prompts.len() - self.max_size);
        }

        Ok(())
    }

    /// Save history to disk
    fn save(&self) -> Result<()> {
        let content = self.prompts.join("\n");
        fs::write(&self.history_file, content)
            .context("Failed to write prompt history file")?;
        Ok(())
    }

    /// Add a prompt to history
    pub fn add(&mut self, prompt: String) -> Result<()> {
        // Don't add empty prompts
        if prompt.trim().is_empty() {
            return Ok(());
        }

        // Remove any existing occurrence of this prompt (move to end)
        self.prompts.retain(|p| p != &prompt);

        self.prompts.push(prompt);

        // Trim to max size
        if self.prompts.len() > self.max_size {
            self.prompts.remove(0);
        }

        // Reset navigation index
        self.current_index = None;

        self.save()
    }

    /// Get the previous prompt (up arrow)
    pub fn previous(&mut self) -> Option<String> {
        if self.prompts.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            None => Some(self.prompts.len() - 1),
            Some(0) => Some(0), // Stay at oldest
            Some(idx) => Some(idx - 1),
        };

        self.current_index = new_index;
        new_index.and_then(|idx| self.prompts.get(idx).cloned())
    }

    /// Get the next prompt (down arrow)
    pub fn next(&mut self) -> Option<String> {
        if self.prompts.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            None => None,
            Some(idx) if idx >= self.prompts.len() - 1 => None,
            Some(idx) => Some(idx + 1),
        };

        self.current_index = new_index;
        new_index.and_then(|idx| self.prompts.get(idx).cloned())
    }

    /// Reset navigation to most recent
    pub fn reset(&mut self) {
        self.current_index = None;
    }

    /// Check if currently navigating history (i.e., user has pressed Up at least once)
    pub fn is_navigating(&self) -> bool {
        self.current_index.is_some()
    }

    /// Search history for prompts containing the query
    pub fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return self.prompts.clone();
        }

        let query_lower = query.to_lowercase();
        self.prompts
            .iter()
            .rev() // Search from newest to oldest
            .filter(|prompt| prompt.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    /// Get all prompts (newest first)
    pub fn get_all(&self) -> Vec<String> {
        let mut prompts = self.prompts.clone();
        prompts.reverse();
        prompts
    }

    /// Clear all history
    pub fn clear(&mut self) -> Result<()> {
        self.prompts.clear();
        self.current_index = None;
        self.save()
    }

    /// Get the number of prompts in history
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}

impl Default for PromptHistory {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            prompts: Vec::new(),
            current_index: None,
            history_file: PathBuf::from("prompt_history.txt"),
            max_size: MAX_HISTORY_SIZE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Wrapper that holds the temp directory to keep it alive
    struct TestHistory {
        history: PromptHistory,
        _temp_dir: TempDir,
    }

    impl std::ops::Deref for TestHistory {
        type Target = PromptHistory;
        fn deref(&self) -> &Self::Target {
            &self.history
        }
    }

    impl std::ops::DerefMut for TestHistory {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.history
        }
    }

    /// Create a fresh test history that doesn't interfere with real history
    fn test_history() -> TestHistory {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_file = temp_dir.path().join("test_prompt_history.txt");
        TestHistory {
            history: PromptHistory {
                prompts: Vec::new(),
                current_index: None,
                history_file,
                max_size: MAX_HISTORY_SIZE,
            },
            _temp_dir: temp_dir,
        }
    }

    #[test]
    fn test_add_prompt() {
        let mut history = test_history();
        history.add("test prompt 1".to_string()).unwrap();
        history.add("test prompt 2".to_string()).unwrap();

        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_no_duplicates_moves_to_end() {
        let mut history = test_history();
        history.add("prompt 1".to_string()).unwrap();
        history.add("prompt 2".to_string()).unwrap();
        history.add("prompt 1".to_string()).unwrap(); // Should move to end

        assert_eq!(history.len(), 2);
        // Most recent should be "prompt 1"
        assert_eq!(history.previous(), Some("prompt 1".to_string()));
        assert_eq!(history.previous(), Some("prompt 2".to_string()));
    }

    #[test]
    fn test_navigation() {
        let mut history = test_history();
        history.add("prompt 1".to_string()).unwrap();
        history.add("prompt 2".to_string()).unwrap();
        history.add("prompt 3".to_string()).unwrap();

        // Up should give most recent
        assert_eq!(history.previous(), Some("prompt 3".to_string()));
        assert_eq!(history.previous(), Some("prompt 2".to_string()));
        assert_eq!(history.previous(), Some("prompt 1".to_string()));

        // Down should go forward
        assert_eq!(history.next(), Some("prompt 2".to_string()));
        assert_eq!(history.next(), Some("prompt 3".to_string()));
        assert_eq!(history.next(), None);
    }

    #[test]
    fn test_search() {
        let mut history = test_history();
        history.add("hello world".to_string()).unwrap();
        history.add("goodbye world".to_string()).unwrap();
        history.add("hello there".to_string()).unwrap();

        let results = history.search("hello");
        assert_eq!(results.len(), 2);
        assert!(results[0].contains("hello"));
    }

    #[test]
    fn test_empty_prompt_ignored() {
        let mut history = test_history();
        history.add("".to_string()).unwrap();
        history.add("  ".to_string()).unwrap();

        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_reset() {
        let mut history = test_history();
        history.add("prompt 1".to_string()).unwrap();
        history.add("prompt 2".to_string()).unwrap();

        history.previous();
        history.reset();

        assert_eq!(history.current_index, None);
    }
}
