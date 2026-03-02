use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A persistent thought stored in the Open Brain.
///
/// Thoughts are the primary unit of knowledge capture — explicit records
/// of decisions, insights, people, action items, and more that persist
/// with Canonical authority (no TTL, never auto-evicted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub id: String,
    pub content: String,
    pub category: ThoughtCategory,
    pub tags: Vec<String>,
    pub source: ThoughtSource,
    pub importance: f32,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted: bool,
}

impl Thought {
    /// Create a new thought with the given content and defaults.
    pub fn new(content: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            category: ThoughtCategory::General,
            tags: Vec::new(),
            source: ThoughtSource::ManualCapture,
            importance: 0.5,
            created_at: now,
            updated_at: now,
            deleted: false,
        }
    }

    /// Builder: set category.
    pub fn with_category(mut self, category: ThoughtCategory) -> Self {
        self.category = category;
        self
    }

    /// Builder: set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: set source.
    pub fn with_source(mut self, source: ThoughtSource) -> Self {
        self.source = source;
        self
    }

    /// Builder: set importance (clamped to 0.0–1.0).
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }
}

/// Category of a thought, used for filtering and organisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThoughtCategory {
    Decision,
    Person,
    Insight,
    MeetingNote,
    Idea,
    ActionItem,
    Reference,
    General,
}

impl ThoughtCategory {
    /// All variants for iteration.
    pub const ALL: &[ThoughtCategory] = &[
        Self::Decision,
        Self::Person,
        Self::Insight,
        Self::MeetingNote,
        Self::Idea,
        Self::ActionItem,
        Self::Reference,
        Self::General,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Person => "person",
            Self::Insight => "insight",
            Self::MeetingNote => "meeting_note",
            Self::Idea => "idea",
            Self::ActionItem => "action_item",
            Self::Reference => "reference",
            Self::General => "general",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "decision" => Self::Decision,
            "person" => Self::Person,
            "insight" => Self::Insight,
            "meeting_note" | "meetingnote" => Self::MeetingNote,
            "idea" => Self::Idea,
            "action_item" | "actionitem" | "todo" => Self::ActionItem,
            "reference" | "ref" => Self::Reference,
            _ => Self::General,
        }
    }
}

impl fmt::Display for ThoughtCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a thought was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThoughtSource {
    ManualCapture,
    ConversationExtract,
    Import,
}

impl ThoughtSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManualCapture => "manual",
            Self::ConversationExtract => "conversation",
            Self::Import => "import",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "manual" | "manual_capture" => Self::ManualCapture,
            "conversation" | "conversation_extract" => Self::ConversationExtract,
            "import" => Self::Import,
            _ => Self::ManualCapture,
        }
    }
}

impl fmt::Display for ThoughtSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thought_creation() {
        let thought = Thought::new("Test thought".into())
            .with_category(ThoughtCategory::Decision)
            .with_tags(vec!["rust".into(), "architecture".into()])
            .with_importance(0.8);

        assert_eq!(thought.category, ThoughtCategory::Decision);
        assert_eq!(thought.tags.len(), 2);
        assert!((thought.importance - 0.8).abs() < f32::EPSILON);
        assert!(!thought.deleted);
    }

    #[test]
    fn test_category_roundtrip() {
        for cat in ThoughtCategory::ALL {
            let s = cat.as_str();
            let parsed = ThoughtCategory::from_str(s);
            assert_eq!(*cat, parsed);
        }
    }

    #[test]
    fn test_importance_clamped() {
        let t = Thought::new("x".into()).with_importance(1.5);
        assert!((t.importance - 1.0).abs() < f32::EPSILON);

        let t = Thought::new("x".into()).with_importance(-0.5);
        assert!((t.importance - 0.0).abs() < f32::EPSILON);
    }
}
