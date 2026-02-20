//! Working Set for File Context Management
//!
//! Tracks files that are currently "in context" for the AI agent.
//! Supports LRU-style eviction to prevent context bloat.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Maximum number of files in the working set by default
pub const DEFAULT_MAX_FILES: usize = 15;

/// Maximum total tokens in working set by default (rough estimate)
pub const DEFAULT_MAX_TOKENS: usize = 100_000;

/// A file entry in the working set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingSetEntry {
    pub path: PathBuf,
    pub tokens: usize,
    pub access_count: u32,
    pub last_access_turn: u32,
    pub added_at_turn: u32,
    pub pinned: bool,
    pub label: Option<String>,
}

impl WorkingSetEntry {
    pub fn new(path: PathBuf, tokens: usize, current_turn: u32) -> Self {
        Self {
            path,
            tokens,
            access_count: 1,
            last_access_turn: current_turn,
            added_at_turn: current_turn,
            pinned: false,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

/// Working set configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingSetConfig {
    pub max_files: usize,
    pub max_tokens: usize,
    pub stale_after_turns: u32,
    pub auto_evict: bool,
}

impl Default for WorkingSetConfig {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
            max_tokens: DEFAULT_MAX_TOKENS,
            stale_after_turns: 10,
            auto_evict: true,
        }
    }
}

/// Manages the set of files currently in the agent's context
#[derive(Debug, Clone, Default)]
pub struct WorkingSet {
    entries: HashMap<String, WorkingSetEntry>,
    config: WorkingSetConfig,
    current_turn: u32,
    last_eviction: Option<String>,
}

impl WorkingSet {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            config: WorkingSetConfig::default(),
            current_turn: 0,
            last_eviction: None,
        }
    }

    pub fn with_config(config: WorkingSetConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            current_turn: 0,
            last_eviction: None,
        }
    }

    pub fn next_turn(&mut self) {
        self.current_turn += 1;
        if self.config.auto_evict {
            self.evict_stale();
        }
    }

    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    pub fn add(&mut self, path: PathBuf, tokens: usize) -> Option<String> {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            return None;
        }
        let eviction_reason = self.maybe_evict(tokens);
        let entry = WorkingSetEntry::new(path, tokens, self.current_turn);
        self.entries.insert(key, entry);
        eviction_reason
    }

    pub fn add_labeled(&mut self, path: PathBuf, tokens: usize, label: &str) -> Option<String> {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            entry.label = Some(label.to_string());
            return None;
        }
        let eviction_reason = self.maybe_evict(tokens);
        let entry = WorkingSetEntry::new(path, tokens, self.current_turn).with_label(label);
        self.entries.insert(key, entry);
        eviction_reason
    }

    pub fn add_pinned(&mut self, path: PathBuf, tokens: usize, label: Option<&str>) {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = true;
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            if let Some(l) = label {
                entry.label = Some(l.to_string());
            }
            return;
        }
        let mut entry = WorkingSetEntry::new(path, tokens, self.current_turn).pinned();
        if let Some(l) = label {
            entry.label = Some(l.to_string());
        }
        self.entries.insert(key, entry);
    }

    pub fn touch(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        self.entries.remove(&key).is_some()
    }

    pub fn pin(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = true;
            true
        } else {
            false
        }
    }

    pub fn unpin(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = false;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self, keep_pinned: bool) {
        if keep_pinned {
            self.entries.retain(|_, entry| entry.pinned);
        } else {
            self.entries.clear();
        }
        self.last_eviction = None;
    }

    pub fn entries(&self) -> impl Iterator<Item = &WorkingSetEntry> {
        self.entries.values()
    }

    pub fn get(&self, path: &PathBuf) -> Option<&WorkingSetEntry> {
        let key = path.to_string_lossy().to_string();
        self.entries.get(&key)
    }

    pub fn contains(&self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        self.entries.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_tokens(&self) -> usize {
        self.entries.values().map(|e| e.tokens).sum()
    }

    pub fn last_eviction(&self) -> Option<&str> {
        self.last_eviction.as_deref()
    }

    pub fn file_paths(&self) -> Vec<&PathBuf> {
        self.entries.values().map(|e| &e.path).collect()
    }

    fn evict_stale(&mut self) {
        let stale_threshold = self.current_turn.saturating_sub(self.config.stale_after_turns);
        let before_count = self.entries.len();
        self.entries.retain(|_, entry| {
            entry.pinned || entry.last_access_turn > stale_threshold
        });
        let evicted = before_count - self.entries.len();
        if evicted > 0 {
            self.last_eviction = Some(format!("Evicted {} stale file(s)", evicted));
        }
    }

    fn maybe_evict(&mut self, new_tokens: usize) -> Option<String> {
        let mut evicted_files = Vec::new();
        while self.entries.len() >= self.config.max_files {
            if let Some(key) = self.find_lru_candidate() {
                if let Some(entry) = self.entries.remove(&key) {
                    evicted_files.push(entry.path.to_string_lossy().to_string());
                }
            } else {
                break;
            }
        }
        while self.total_tokens() + new_tokens > self.config.max_tokens {
            if let Some(key) = self.find_lru_candidate() {
                if let Some(entry) = self.entries.remove(&key) {
                    evicted_files.push(entry.path.to_string_lossy().to_string());
                }
            } else {
                break;
            }
        }
        if evicted_files.is_empty() {
            None
        } else {
            let reason = format!("Evicted: {}", evicted_files.join(", "));
            self.last_eviction = Some(reason.clone());
            Some(reason)
        }
    }

    fn find_lru_candidate(&self) -> Option<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| !entry.pinned)
            .min_by_key(|(_, entry)| (entry.last_access_turn, entry.access_count))
            .map(|(key, _)| key.clone())
    }
}

/// Estimate tokens for a string (rough: ~4 chars per token)
pub fn estimate_tokens(content: &str) -> usize {
    (content.len() + 3) / 4
}

/// Estimate tokens for a file by size
pub fn estimate_tokens_from_size(bytes: u64) -> usize {
    ((bytes as usize) + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_set_add_and_access() {
        let mut ws = WorkingSet::new();
        ws.add(PathBuf::from("/test/file1.rs"), 1000);
        assert_eq!(ws.len(), 1);
        assert!(ws.contains(&PathBuf::from("/test/file1.rs")));
    }

    #[test]
    fn test_working_set_lru_eviction() {
        let config = WorkingSetConfig {
            max_files: 3,
            max_tokens: 100_000,
            stale_after_turns: 10,
            auto_evict: false,
        };
        let mut ws = WorkingSet::with_config(config);
        ws.add(PathBuf::from("/test/file1.rs"), 100);
        ws.next_turn();
        ws.add(PathBuf::from("/test/file2.rs"), 100);
        ws.next_turn();
        ws.add(PathBuf::from("/test/file3.rs"), 100);
        ws.next_turn();
        ws.add(PathBuf::from("/test/file4.rs"), 100);
        assert_eq!(ws.len(), 3);
        assert!(!ws.contains(&PathBuf::from("/test/file1.rs")));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1);
    }
}
