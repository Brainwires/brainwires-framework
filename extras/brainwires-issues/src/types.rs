//! Core data types for the issue tracking system.

use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── IssueStatus ──────────────────────────────────────────────────────────

/// Workflow status of an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    /// Not yet scheduled.
    #[default]
    Backlog,
    /// Scheduled, not started.
    Todo,
    /// Actively being worked on.
    InProgress,
    /// In code/design review.
    InReview,
    /// Completed successfully.
    Done,
    /// Cancelled (won't fix).
    Cancelled,
}

impl IssueStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueStatus::Backlog => "backlog",
            IssueStatus::Todo => "todo",
            IssueStatus::InProgress => "in_progress",
            IssueStatus::InReview => "in_review",
            IssueStatus::Done => "done",
            IssueStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "backlog" => IssueStatus::Backlog,
            "todo" => IssueStatus::Todo,
            "in_progress" => IssueStatus::InProgress,
            "in_review" => IssueStatus::InReview,
            "done" => IssueStatus::Done,
            "cancelled" => IssueStatus::Cancelled,
            _ => IssueStatus::Backlog,
        }
    }

    /// Returns true if this status represents a closed issue.
    pub fn is_closed(&self) -> bool {
        matches!(self, IssueStatus::Done | IssueStatus::Cancelled)
    }
}

// ── IssuePriority ────────────────────────────────────────────────────────

/// Priority level of an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuePriority {
    /// No priority assigned.
    #[default]
    NoPriority,
    /// Low priority.
    Low,
    /// Medium priority.
    Medium,
    /// High priority.
    High,
    /// Urgent — needs immediate attention.
    Urgent,
}

impl IssuePriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssuePriority::NoPriority => "no_priority",
            IssuePriority::Low => "low",
            IssuePriority::Medium => "medium",
            IssuePriority::High => "high",
            IssuePriority::Urgent => "urgent",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "no_priority" => IssuePriority::NoPriority,
            "low" => IssuePriority::Low,
            "medium" => IssuePriority::Medium,
            "high" => IssuePriority::High,
            "urgent" => IssuePriority::Urgent,
            _ => IssuePriority::NoPriority,
        }
    }
}

// ── Issue ────────────────────────────────────────────────────────────────

/// A tracked issue or bug report.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Issue {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Auto-incrementing display number (e.g. #42).
    pub number: u64,
    /// Short title of the issue.
    pub title: String,
    /// Full description in Markdown.
    pub description: String,
    /// Current workflow status.
    pub status: IssueStatus,
    /// Priority level.
    pub priority: IssuePriority,
    /// Comma-separated label tags stored as a JSON array string internally.
    pub labels: Vec<String>,
    /// Person or agent assigned to this issue.
    pub assignee: Option<String>,
    /// Project or milestone this issue belongs to.
    pub project: Option<String>,
    /// Parent issue ID for sub-issues.
    pub parent_id: Option<String>,
    /// Creation time (Unix seconds).
    pub created_at: i64,
    /// Last update time (Unix seconds).
    pub updated_at: i64,
    /// Time when the issue was closed (Unix seconds).
    pub closed_at: Option<i64>,
}

impl Issue {
    /// Create a new issue with defaults.
    pub fn new(number: u64, title: impl Into<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            number,
            title: title.into(),
            description: String::new(),
            status: IssueStatus::Backlog,
            priority: IssuePriority::NoPriority,
            labels: Vec::new(),
            assignee: None,
            project: None,
            parent_id: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
        }
    }
}

// ── IssuePatch ───────────────────────────────────────────────────────────

/// Partial update for an issue — all fields are optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct IssuePatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<IssueStatus>,
    pub priority: Option<IssuePriority>,
    pub labels: Option<Vec<String>>,
    pub assignee: Option<String>,
    /// Pass `""` to clear the assignee.
    pub clear_assignee: Option<bool>,
    pub project: Option<String>,
    /// Pass `""` to clear the project.
    pub clear_project: Option<bool>,
    pub parent_id: Option<String>,
    /// Pass `true` to clear the parent.
    pub clear_parent: Option<bool>,
}

// ── Comment ──────────────────────────────────────────────────────────────

/// A comment on an issue.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Comment {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// The issue this comment belongs to.
    pub issue_id: String,
    /// Author name or identifier.
    pub author: Option<String>,
    /// Comment body in Markdown.
    pub body: String,
    /// Creation time (Unix seconds).
    pub created_at: i64,
    /// Last update time (Unix seconds).
    pub updated_at: i64,
}

impl Comment {
    pub fn new(issue_id: impl Into<String>, body: impl Into<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            issue_id: issue_id.into(),
            author: None,
            body: body.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
