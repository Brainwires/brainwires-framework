use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── capture_thought ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaptureThoughtRequest {
    /// The thought text to capture
    pub content: String,
    /// Category: decision, person, insight, meeting_note, idea, action_item, reference, general.
    /// Auto-detected if omitted.
    #[serde(default)]
    pub category: Option<String>,
    /// User-provided tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Importance score 0.0–1.0 (default: 0.5)
    #[serde(default)]
    pub importance: Option<f32>,
    /// Source identifier (default: "manual")
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureThoughtResponse {
    pub id: String,
    pub category: String,
    pub tags: Vec<String>,
    pub importance: f32,
    pub facts_extracted: usize,
}

// ── search_memory ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMemoryRequest {
    /// Natural language search query
    pub query: String,
    /// Max results (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum similarity score (default: 0.6)
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    /// Filter by ThoughtCategory
    #[serde(default)]
    pub category: Option<String>,
    /// Which stores to search: "thoughts", "facts". Default: all.
    #[serde(default)]
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryResponse {
    pub results: Vec<MemorySearchResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub content: String,
    pub score: f32,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

// ── list_recent ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRecentRequest {
    /// Max results (default: 20)
    #[serde(default = "default_list_limit")]
    pub limit: usize,
    /// Filter by category
    #[serde(default)]
    pub category: Option<String>,
    /// ISO 8601 timestamp (default: 7 days ago)
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRecentResponse {
    pub thoughts: Vec<ThoughtSummary>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSummary {
    pub id: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub importance: f32,
    pub created_at: i64,
}

// ── get_thought ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetThoughtRequest {
    /// Thought UUID
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetThoughtResponse {
    pub id: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub source: String,
    pub importance: f32,
    pub created_at: i64,
    pub updated_at: i64,
}

// ── search_knowledge ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchKnowledgeRequest {
    /// Context to match against
    pub query: String,
    /// "personal" (PKS), "behavioral" (BKS), or "all" (default)
    #[serde(default)]
    pub source: Option<String>,
    /// PKS/BKS category filter
    #[serde(default)]
    pub category: Option<String>,
    /// Minimum confidence (default: 0.5)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
    /// Max results (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKnowledgeResponse {
    pub results: Vec<KnowledgeResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeResult {
    pub source: String,
    pub category: String,
    pub key: String,
    pub value: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

// ── memory_stats ─────────────────────────────────────────────────────────

// No request params needed — but we still define an empty struct for the macro.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemoryStatsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatsResponse {
    pub thoughts: ThoughtStats,
    pub pks: PksStats,
    pub bks: BksStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStats {
    pub total: usize,
    pub by_category: std::collections::HashMap<String, usize>,
    pub recent_24h: usize,
    pub recent_7d: usize,
    pub recent_30d: usize,
    pub top_tags: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PksStats {
    pub total_facts: u32,
    pub by_category: std::collections::HashMap<String, u32>,
    pub avg_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BksStats {
    pub total_truths: u32,
    pub by_category: std::collections::HashMap<String, u32>,
}

// ── delete_thought ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteThoughtRequest {
    /// Thought UUID to delete
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteThoughtResponse {
    pub deleted: bool,
    pub id: String,
}

// ── defaults ─────────────────────────────────────────────────────────────

fn default_limit() -> usize {
    10
}

fn default_list_limit() -> usize {
    20
}

fn default_min_score() -> f32 {
    0.6
}

fn default_min_confidence() -> f32 {
    0.5
}
