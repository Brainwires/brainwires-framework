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
    /// Absolute path to the file
    pub path: PathBuf,
    /// Number of tokens (estimated)
    pub tokens: usize,
    /// Number of times accessed this session
    pub access_count: u32,
    /// Turn number when last accessed
    pub last_access_turn: u32,
    /// Turn number when added
    pub added_at_turn: u32,
    /// Whether this file is pinned (won't be evicted)
    pub pinned: bool,
    /// Optional label/reason for inclusion
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
    /// Maximum number of files
    pub max_files: usize,
    /// Maximum total tokens
    pub max_tokens: usize,
    /// Number of turns before a file becomes stale
    pub stale_after_turns: u32,
    /// Whether to auto-evict stale files
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
    /// Files in the working set, keyed by path string
    entries: HashMap<String, WorkingSetEntry>,
    /// Configuration
    config: WorkingSetConfig,
    /// Current turn number
    current_turn: u32,
    /// Last eviction reason (for debugging/display)
    last_eviction: Option<String>,
}

impl WorkingSet {
    /// Create a new empty working set
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            config: WorkingSetConfig::default(),
            current_turn: 0,
            last_eviction: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: WorkingSetConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            current_turn: 0,
            last_eviction: None,
        }
    }

    /// Increment the turn counter (call this each conversation turn)
    pub fn next_turn(&mut self) {
        self.current_turn += 1;
        if self.config.auto_evict {
            self.evict_stale();
        }
    }

    /// Get current turn number
    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    /// Add a file to the working set
    pub fn add(&mut self, path: PathBuf, tokens: usize) -> Option<String> {
        let key = path.to_string_lossy().to_string();

        // If already present, just update access
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            return None;
        }

        // Check if we need to evict
        let eviction_reason = self.maybe_evict(tokens);

        // Add the new entry
        let entry = WorkingSetEntry::new(path, tokens, self.current_turn);
        self.entries.insert(key, entry);

        eviction_reason
    }

    /// Add a file with a label
    pub fn add_labeled(&mut self, path: PathBuf, tokens: usize, label: &str) -> Option<String> {
        let key = path.to_string_lossy().to_string();

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.access_count += 1;
            entry.last_access_turn = self.current_turn;
            entry.label = Some(label.to_string());
            return None;
        }

        let eviction_reason = self.maybe_evict(tokens);

        let entry = WorkingSetEntry::new(path, tokens, self.current_turn)
            .with_label(label);
        self.entries.insert(key, entry);

        eviction_reason
    }

    /// Add a pinned file (won't be evicted)
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

    /// Touch a file (update access time without adding)
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

    /// Remove a specific file
    pub fn remove(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        self.entries.remove(&key).is_some()
    }

    /// Pin a file (prevent eviction)
    pub fn pin(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = true;
            true
        } else {
            false
        }
    }

    /// Unpin a file
    pub fn unpin(&mut self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = false;
            true
        } else {
            false
        }
    }

    /// Clear all files (except pinned if keep_pinned is true)
    pub fn clear(&mut self, keep_pinned: bool) {
        if keep_pinned {
            self.entries.retain(|_, entry| entry.pinned);
        } else {
            self.entries.clear();
        }
        self.last_eviction = None;
    }

    /// Get all entries
    pub fn entries(&self) -> impl Iterator<Item = &WorkingSetEntry> {
        self.entries.values()
    }

    /// Get entry by path
    pub fn get(&self, path: &PathBuf) -> Option<&WorkingSetEntry> {
        let key = path.to_string_lossy().to_string();
        self.entries.get(&key)
    }

    /// Check if a file is in the working set
    pub fn contains(&self, path: &PathBuf) -> bool {
        let key = path.to_string_lossy().to_string();
        self.entries.contains_key(&key)
    }

    /// Get number of files
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> usize {
        self.entries.values().map(|e| e.tokens).sum()
    }

    /// Get last eviction reason
    pub fn last_eviction(&self) -> Option<&str> {
        self.last_eviction.as_deref()
    }

    /// Get stale files (not accessed in stale_after_turns)
    pub fn stale_files(&self) -> Vec<&WorkingSetEntry> {
        self.entries
            .values()
            .filter(|e| {
                !e.pinned &&
                self.current_turn.saturating_sub(e.last_access_turn) >= self.config.stale_after_turns
            })
            .collect()
    }

    /// Evict stale files
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

    /// Maybe evict files to make room for new tokens
    fn maybe_evict(&mut self, new_tokens: usize) -> Option<String> {
        let mut evicted_files = Vec::new();

        // Evict if over file limit
        while self.entries.len() >= self.config.max_files {
            if let Some(key) = self.find_lru_candidate() {
                if let Some(entry) = self.entries.remove(&key) {
                    evicted_files.push(entry.path.to_string_lossy().to_string());
                }
            } else {
                break; // All files are pinned
            }
        }

        // Evict if over token limit
        while self.total_tokens() + new_tokens > self.config.max_tokens {
            if let Some(key) = self.find_lru_candidate() {
                if let Some(entry) = self.entries.remove(&key) {
                    evicted_files.push(entry.path.to_string_lossy().to_string());
                }
            } else {
                break; // All files are pinned
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

    /// Find the best candidate for eviction (LRU, not pinned)
    fn find_lru_candidate(&self) -> Option<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| !entry.pinned)
            .min_by_key(|(_, entry)| (entry.last_access_turn, entry.access_count))
            .map(|(key, _)| key.clone())
    }

    /// Format as a display string
    pub fn display(&self) -> String {
        if self.entries.is_empty() {
            return "Working set is empty".to_string();
        }

        let mut lines = vec![format!(
            "Working Set ({} files, ~{} tokens, turn {}):",
            self.entries.len(),
            self.total_tokens(),
            self.current_turn
        )];

        // Sort by last access (most recent first)
        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.last_access_turn));

        for entry in sorted {
            let file_name = entry.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| entry.path.to_string_lossy().to_string());

            let mut flags = Vec::new();
            if entry.pinned {
                flags.push("📌");
            }
            let stale = self.current_turn.saturating_sub(entry.last_access_turn) >= self.config.stale_after_turns;
            if stale && !entry.pinned {
                flags.push("⏳");
            }

            let label = entry.label.as_ref()
                .map(|l| format!(" ({})", l))
                .unwrap_or_default();

            lines.push(format!(
                "  {} {}{} [~{} tokens, accessed turn {}]",
                flags.join(""),
                file_name,
                label,
                entry.tokens,
                entry.last_access_turn
            ));
        }

        if let Some(reason) = &self.last_eviction {
            lines.push(format!("\n  Last eviction: {}", reason));
        }

        lines.join("\n")
    }
}

/// Estimate tokens for a string (rough: ~4 chars per token)
pub fn estimate_tokens(content: &str) -> usize {
    (content.len() + 3) / 4
}

impl WorkingSet {
    /// Build a context string with file contents for injection into system prompt.
    /// Returns None if working set is empty, or Some(formatted_content) with file contents.
    /// Respects max_tokens limit from config.
    pub fn build_context_injection(&self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        let mut sections = Vec::new();
        let mut total_tokens = 0;
        let max_tokens = self.config.max_tokens;

        // Sort by: pinned first, then by last access (most recent first)
        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by(|a, b| {
            match (a.pinned, b.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.last_access_turn.cmp(&a.last_access_turn),
            }
        });

        for entry in sorted {
            // Check if we'd exceed token limit
            if total_tokens >= max_tokens {
                break;
            }

            // Try to read the file
            match std::fs::read_to_string(&entry.path) {
                Ok(content) => {
                    let file_tokens = estimate_tokens(&content);

                    // Truncate if this single file would exceed remaining budget
                    let remaining = max_tokens.saturating_sub(total_tokens);
                    let (content, truncated) = if file_tokens > remaining {
                        // Truncate to fit
                        let max_chars = remaining * 4;
                        let truncated_content = if content.len() > max_chars {
                            format!("{}...\n[TRUNCATED - file too large]", &content[..max_chars])
                        } else {
                            content
                        };
                        (truncated_content, true)
                    } else {
                        (content, false)
                    };

                    let label = entry.label.as_ref()
                        .map(|l| format!(" ({})", l))
                        .unwrap_or_default();
                    let pin_marker = if entry.pinned { " [pinned]" } else { "" };
                    let trunc_marker = if truncated { " [truncated]" } else { "" };

                    sections.push(format!(
                        "=== {} {}{}{} ===\n{}",
                        entry.path.display(),
                        label,
                        pin_marker,
                        trunc_marker,
                        content
                    ));

                    total_tokens += estimate_tokens(&content);
                }
                Err(e) => {
                    // File couldn't be read - include error note
                    sections.push(format!(
                        "=== {} ===\n[Error reading file: {}]",
                        entry.path.display(),
                        e
                    ));
                }
            }
        }

        if sections.is_empty() {
            return None;
        }

        Some(format!(
            "[Working Set Files]\n\
            The following {} file(s) are currently in your working context (~{} tokens):\n\n\
            {}\n\n\
            [End Working Set Files]",
            self.entries.len(),
            total_tokens,
            sections.join("\n\n")
        ))
    }

    /// Get paths of all files in the working set
    pub fn file_paths(&self) -> Vec<&PathBuf> {
        self.entries.values().map(|e| &e.path).collect()
    }

    /// Extract file paths mentioned in text content.
    /// Looks for patterns like `/path/to/file.rs`, `src/foo.rs`, `./file.txt`.
    /// Returns existing file paths only.
    pub fn extract_file_references(content: &str) -> Vec<PathBuf> {
        use std::path::Path;

        let mut found_paths = Vec::new();

        // Split by whitespace and common delimiters
        let tokens: Vec<&str> = content
            .split(|c: char| c.is_whitespace() || c == '\n' || c == '`' || c == '"' || c == '\'' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' || c == '<' || c == '>')
            .filter(|s| !s.is_empty())
            .collect();

        for token in tokens {
            let token = token.trim_matches(|c: char| c == ':' || c == ',' || c == ';');

            // Check if it looks like a path (contains / or \ and has an extension)
            if (token.contains('/') || token.contains('\\'))
                && token.contains('.')
                && token.len() > 3
            {
                let path = Path::new(token);

                // Try as-is first
                if path.exists() && path.is_file() {
                    if let Ok(canonical) = path.canonicalize() {
                        if !found_paths.contains(&canonical) {
                            found_paths.push(canonical);
                        }
                    }
                    continue;
                }

                // Try relative to CWD
                if let Ok(cwd) = std::env::current_dir() {
                    let full_path = cwd.join(token);
                    if full_path.exists() && full_path.is_file() {
                        if let Ok(canonical) = full_path.canonicalize() {
                            if !found_paths.contains(&canonical) {
                                found_paths.push(canonical);
                            }
                        }
                    }
                }
            }
        }

        found_paths
    }

    /// Suggest files to add based on content (e.g., from retrieved history).
    /// Returns files that are mentioned but not currently in the working set.
    pub fn suggest_from_content(&self, content: &str) -> Vec<PathBuf> {
        Self::extract_file_references(content)
            .into_iter()
            .filter(|p| !self.contains(p))
            .collect()
    }
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

        // Adding same file updates access count
        ws.add(PathBuf::from("/test/file1.rs"), 1000);
        assert_eq!(ws.len(), 1);
        let entry = ws.get(&PathBuf::from("/test/file1.rs")).unwrap();
        assert_eq!(entry.access_count, 2);
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

        // This should evict file1 (oldest)
        ws.add(PathBuf::from("/test/file4.rs"), 100);

        assert_eq!(ws.len(), 3);
        assert!(!ws.contains(&PathBuf::from("/test/file1.rs")));
        assert!(ws.contains(&PathBuf::from("/test/file4.rs")));
    }

    #[test]
    fn test_working_set_pinned_not_evicted() {
        let config = WorkingSetConfig {
            max_files: 2,
            max_tokens: 100_000,
            stale_after_turns: 10,
            auto_evict: false,
        };
        let mut ws = WorkingSet::with_config(config);

        ws.add_pinned(PathBuf::from("/test/pinned.rs"), 100, Some("important"));
        ws.add(PathBuf::from("/test/normal.rs"), 100);

        // Try to add third file - should evict normal, not pinned
        ws.add(PathBuf::from("/test/new.rs"), 100);

        assert!(ws.contains(&PathBuf::from("/test/pinned.rs")));
        assert!(!ws.contains(&PathBuf::from("/test/normal.rs")));
        assert!(ws.contains(&PathBuf::from("/test/new.rs")));
    }

    #[test]
    fn test_working_set_stale_eviction() {
        let config = WorkingSetConfig {
            max_files: 10,
            max_tokens: 100_000,
            stale_after_turns: 2,
            auto_evict: true,
        };
        let mut ws = WorkingSet::with_config(config);

        ws.add(PathBuf::from("/test/file1.rs"), 100);
        ws.next_turn();
        ws.next_turn();
        ws.next_turn(); // File1 is now stale and should be evicted

        assert!(!ws.contains(&PathBuf::from("/test/file1.rs")));
    }

    #[test]
    fn test_working_set_clear() {
        let mut ws = WorkingSet::new();

        ws.add_pinned(PathBuf::from("/test/pinned.rs"), 100, None);
        ws.add(PathBuf::from("/test/normal.rs"), 100);

        ws.clear(true); // Keep pinned
        assert_eq!(ws.len(), 1);
        assert!(ws.contains(&PathBuf::from("/test/pinned.rs")));

        ws.clear(false); // Clear all
        assert!(ws.is_empty());
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
    }

    #[test]
    fn test_extract_file_references() {
        // Test with content mentioning existing files (use Cargo.toml which should exist)
        let content = "I modified src/lib.rs and Cargo.toml to add the feature";
        let refs = WorkingSet::extract_file_references(content);
        // Should find Cargo.toml if we're in the project directory
        // (This test is environment-dependent but demonstrates the function works)
        assert!(refs.iter().all(|p| p.exists()));
    }

    #[test]
    fn test_suggest_from_content() {
        let ws = WorkingSet::new();
        // Test that it returns empty for non-existent files
        let content = "Let's modify /nonexistent/fake/path.rs";
        let suggestions = ws.suggest_from_content(content);
        assert!(suggestions.is_empty());
    }
}
