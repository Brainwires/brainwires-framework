//! Prompt Caching Utilities
//!
//! Provides utilities for prompt caching, particularly for Anthropic's API.
//! Anthropic supports caching system prompts and static context using
//! cache_control markers.
//!
//! Cache Benefits:
//! - 200-300ms reduction per request for cached portions
//! - Reduced API costs (cached tokens are cheaper)
//! - More consistent response times

use serde_json::{Value, json};

/// Marker for cacheable content in Anthropic API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    /// Ephemeral caching - cached for duration of session
    Ephemeral,
}

/// Represents content that can be cached
#[derive(Debug, Clone)]
pub struct CacheableContent {
    pub content_type: String,
    pub text: String,
    pub cache_type: CacheType,
}

impl CacheableContent {
    /// Create new cacheable text content
    pub fn text(content: &str) -> Self {
        Self {
            content_type: "text".to_string(),
            text: content.to_string(),
            cache_type: CacheType::Ephemeral,
        }
    }

    /// Convert to Anthropic API format with cache_control
    pub fn to_anthropic_format(&self) -> Value {
        json!({
            "type": self.content_type,
            "text": self.text,
            "cache_control": {
                "type": match self.cache_type {
                    CacheType::Ephemeral => "ephemeral",
                }
            }
        })
    }
}

/// Build system prompt with caching for Anthropic API
///
/// Returns the system parameter formatted for caching
pub fn build_cached_system_prompt(system_prompt: &str) -> Value {
    json!([{
        "type": "text",
        "text": system_prompt,
        "cache_control": { "type": "ephemeral" }
    }])
}

/// Build system prompt with multiple cached sections
///
/// Useful when you have static content (always cached) and dynamic content
pub fn build_multi_section_system(static_content: &str, dynamic_content: Option<&str>) -> Value {
    let mut sections = vec![json!({
        "type": "text",
        "text": static_content,
        "cache_control": { "type": "ephemeral" }
    })];

    if let Some(dynamic) = dynamic_content {
        // Dynamic content is not cached
        sections.push(json!({
            "type": "text",
            "text": dynamic
        }));
    }

    json!(sections)
}

/// Configuration for what to cache in a conversation
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache the system prompt
    pub cache_system_prompt: bool,
    /// Cache compaction summaries
    pub cache_compaction_summary: bool,
    /// Cache injected context
    pub cache_injected_context: bool,
    /// Minimum content length to cache (bytes)
    pub min_cache_length: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_system_prompt: true,
            cache_compaction_summary: true,
            cache_injected_context: true,
            min_cache_length: 1024, // Only cache content > 1KB
        }
    }
}

/// Identifies cacheable parts of a conversation context
pub struct CacheAnalyzer {
    config: CacheConfig,
}

impl CacheAnalyzer {
    pub fn new(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Analyze messages and identify cacheable portions
    ///
    /// Returns (cacheable_prefix_end_index, cache_points)
    pub fn analyze(&self, messages: &[Value]) -> CacheAnalysis {
        let mut cache_points = Vec::new();
        let mut stable_prefix_end = 0;

        for (i, msg) in messages.iter().enumerate() {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

            // System messages are generally stable
            if role == "system" {
                if content.len() >= self.config.min_cache_length && self.config.cache_system_prompt
                {
                    cache_points.push(CachePoint {
                        index: i,
                        reason: CacheReason::SystemPrompt,
                        estimated_savings_ms: 100,
                    });
                }
                stable_prefix_end = i + 1;
            }

            // Check for compaction summary
            if content.contains("[Compacted Context]") || content.contains("Summary of earlier") {
                if self.config.cache_compaction_summary {
                    cache_points.push(CachePoint {
                        index: i,
                        reason: CacheReason::CompactionSummary,
                        estimated_savings_ms: 150,
                    });
                }
                stable_prefix_end = i + 1;
            }

            // Check for injected context
            if content.contains("[Retrieved Context]") {
                if self.config.cache_injected_context {
                    cache_points.push(CachePoint {
                        index: i,
                        reason: CacheReason::InjectedContext,
                        estimated_savings_ms: 50,
                    });
                }
                stable_prefix_end = i + 1;
            }
        }

        CacheAnalysis {
            stable_prefix_end,
            cache_points,
        }
    }
}

/// Result of cache analysis
#[derive(Debug)]
pub struct CacheAnalysis {
    /// Index of last message in stable prefix (exclusive)
    pub stable_prefix_end: usize,
    /// Points where caching is beneficial
    pub cache_points: Vec<CachePoint>,
}

impl CacheAnalysis {
    /// Estimated total latency savings in milliseconds
    pub fn estimated_savings_ms(&self) -> u32 {
        self.cache_points
            .iter()
            .map(|p| p.estimated_savings_ms)
            .sum()
    }

    /// Whether any caching is recommended
    pub fn should_cache(&self) -> bool {
        !self.cache_points.is_empty()
    }
}

/// A point in the message list where caching is beneficial
#[derive(Debug)]
pub struct CachePoint {
    /// Index in message list
    pub index: usize,
    /// Why this is cacheable
    pub reason: CacheReason,
    /// Estimated latency savings
    pub estimated_savings_ms: u32,
}

/// Reason for caching a message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheReason {
    SystemPrompt,
    CompactionSummary,
    InjectedContext,
}

/// Hash content for cache key generation
pub fn content_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Generate a cache key for a conversation prefix
pub fn generate_cache_key(messages: &[Value], prefix_end: usize) -> String {
    let prefix_content: String = messages[..prefix_end]
        .iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    format!("cache_{:x}", content_hash(&prefix_content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cacheable_content() {
        let content = CacheableContent::text("System prompt content");
        let formatted = content.to_anthropic_format();

        assert_eq!(formatted["type"], "text");
        assert_eq!(formatted["text"], "System prompt content");
        assert_eq!(formatted["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_build_cached_system_prompt() {
        let result = build_cached_system_prompt("You are a helpful assistant");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_cache_analyzer() {
        let messages = vec![
            json!({"role": "system", "content": "A".repeat(2000)}),
            json!({"role": "user", "content": "Hello"}),
        ];

        let analyzer = CacheAnalyzer::new(CacheConfig::default());
        let analysis = analyzer.analyze(&messages);

        assert!(analysis.should_cache());
        assert_eq!(analysis.stable_prefix_end, 1);
    }

    #[test]
    fn test_cache_analyzer_compaction() {
        let messages = vec![
            json!({"role": "system", "content": "[Compacted Context] Summary of discussion..."}),
            json!({"role": "user", "content": "Continue"}),
        ];

        let analyzer = CacheAnalyzer::new(CacheConfig::default());
        let analysis = analyzer.analyze(&messages);

        assert!(
            analysis
                .cache_points
                .iter()
                .any(|p| p.reason == CacheReason::CompactionSummary)
        );
    }

    #[test]
    fn test_content_hash() {
        let hash1 = content_hash("test content");
        let hash2 = content_hash("test content");
        let hash3 = content_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_generate_cache_key() {
        let messages = vec![
            json!({"role": "system", "content": "prompt"}),
            json!({"role": "user", "content": "hello"}),
        ];

        let key = generate_cache_key(&messages, 1);
        assert!(key.starts_with("cache_"));
    }
}
