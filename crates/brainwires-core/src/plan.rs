use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a plan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Abandoned,
}

impl Default for PlanStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Draft => write!(f, "draft"),
            PlanStatus::Active => write!(f, "active"),
            PlanStatus::Paused => write!(f, "paused"),
            PlanStatus::Completed => write!(f, "completed"),
            PlanStatus::Abandoned => write!(f, "abandoned"),
        }
    }
}

impl std::str::FromStr for PlanStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(PlanStatus::Draft),
            "active" => Ok(PlanStatus::Active),
            "paused" => Ok(PlanStatus::Paused),
            "completed" => Ok(PlanStatus::Completed),
            "abandoned" => Ok(PlanStatus::Abandoned),
            _ => Err(format!("Unknown plan status: {}", s)),
        }
    }
}

/// Metadata for a persisted execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub plan_id: String,
    pub conversation_id: String,
    pub title: String,
    pub task_description: String,
    pub plan_content: String,
    pub model_id: Option<String>,
    pub status: PlanStatus,
    pub executed: bool,
    pub iterations_used: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_plan_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_plan_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub depth: u32,
}

impl PlanMetadata {
    /// Create a new plan with the given task and content
    pub fn new(
        conversation_id: String,
        task_description: String,
        plan_content: String,
    ) -> Self {
        let now = Utc::now().timestamp();
        let plan_id = uuid::Uuid::new_v4().to_string();

        let title = task_description
            .lines()
            .next()
            .unwrap_or(&task_description)
            .chars()
            .take(50)
            .collect::<String>();

        Self {
            plan_id,
            conversation_id,
            title,
            task_description,
            plan_content,
            model_id: None,
            status: PlanStatus::Draft,
            executed: false,
            iterations_used: 0,
            created_at: now,
            updated_at: now,
            file_path: None,
            embedding: None,
            parent_plan_id: None,
            child_plan_ids: Vec::new(),
            branch_name: None,
            merged: false,
            depth: 0,
        }
    }

    /// Create a branch (sub-plan) from this plan
    pub fn create_branch(
        &self,
        branch_name: String,
        task_description: String,
        plan_content: String,
    ) -> Self {
        let mut branch = Self::new(
            self.conversation_id.clone(),
            task_description,
            plan_content,
        );
        branch.parent_plan_id = Some(self.plan_id.clone());
        branch.branch_name = Some(branch_name);
        branch.depth = self.depth + 1;
        branch
    }

    /// Add a child plan ID
    pub fn add_child(&mut self, child_id: String) {
        if !self.child_plan_ids.contains(&child_id) {
            self.child_plan_ids.push(child_id);
            self.updated_at = Utc::now().timestamp();
        }
    }

    /// Mark as merged
    pub fn mark_merged(&mut self) {
        self.merged = true;
        self.status = PlanStatus::Completed;
        self.updated_at = Utc::now().timestamp();
    }

    /// Check if this is a root plan
    pub fn is_root(&self) -> bool {
        self.parent_plan_id.is_none()
    }

    /// Check if this plan has children
    pub fn has_children(&self) -> bool {
        !self.child_plan_ids.is_empty()
    }

    /// Set the model used
    pub fn with_model(mut self, model_id: String) -> Self {
        self.model_id = Some(model_id);
        self
    }

    /// Set iterations used
    pub fn with_iterations(mut self, iterations: u32) -> Self {
        self.iterations_used = iterations;
        self
    }

    /// Mark as executed
    pub fn mark_executed(&mut self) {
        self.executed = true;
        self.status = PlanStatus::Completed;
        self.updated_at = Utc::now().timestamp();
    }

    /// Update status
    pub fn set_status(&mut self, status: PlanStatus) {
        self.status = status;
        self.updated_at = Utc::now().timestamp();
    }

    /// Set file path after export
    pub fn set_file_path(&mut self, path: String) {
        self.file_path = Some(path);
        self.updated_at = Utc::now().timestamp();
    }

    /// Get created_at as DateTime
    pub fn created_at_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.created_at, 0)
            .unwrap_or_else(Utc::now)
    }

    /// Generate markdown export with YAML frontmatter
    pub fn to_markdown(&self) -> String {
        let created = self.created_at_datetime().format("%Y-%m-%dT%H:%M:%SZ");
        let model = self.model_id.as_deref().unwrap_or("unknown");

        format!(
            r#"---
plan_id: {}
conversation_id: {}
title: "{}"
status: {}
executed: {}
iterations: {}
created_at: {}
model: {}
---

# Execution Plan: {}

## Original Task

{}

## Plan

{}

---
*Generated by Brainwires Agent Framework*
"#,
            self.plan_id,
            self.conversation_id,
            self.title.replace('"', r#"\""#),
            self.status,
            self.executed,
            self.iterations_used,
            created,
            model,
            self.title,
            self.task_description,
            self.plan_content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_metadata_new() {
        let plan = PlanMetadata::new(
            "conv-123".to_string(),
            "Implement auth".to_string(),
            "Step 1".to_string(),
        );
        assert!(!plan.plan_id.is_empty());
        assert_eq!(plan.status, PlanStatus::Draft);
        assert!(plan.is_root());
    }

    #[test]
    fn test_plan_branching() {
        let parent = PlanMetadata::new(
            "conv-123".to_string(),
            "Main".to_string(),
            "Plan".to_string(),
        );
        let branch = parent.create_branch(
            "feature-x".to_string(),
            "Feature X".to_string(),
            "Branch plan".to_string(),
        );
        assert_eq!(branch.parent_plan_id, Some(parent.plan_id));
        assert_eq!(branch.depth, 1);
        assert!(!branch.is_root());
    }
}
