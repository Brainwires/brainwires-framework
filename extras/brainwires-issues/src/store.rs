//! Storage layer for issues and comments using the brainwires-storage backend.

use anyhow::{Context, Result};
use brainwires_storage::{
    FieldDef, FieldType, FieldValue, Filter, Record, StorageBackend, bm25_search::BM25Search,
    record_get,
};
use chrono::Utc;
use std::sync::Arc;

use crate::types::{Comment, Issue, IssuePatch, IssuePriority, IssueStatus};

const ISSUES_TABLE: &str = "issues";
const COMMENTS_TABLE: &str = "comments";

// ── Schema ───────────────────────────────────────────────────────────────

fn issues_field_defs() -> Vec<FieldDef> {
    vec![
        FieldDef::required("issue_id", FieldType::Utf8),
        FieldDef::required("number", FieldType::UInt64),
        FieldDef::required("title", FieldType::Utf8),
        FieldDef::required("description", FieldType::Utf8),
        FieldDef::required("status", FieldType::Utf8),
        FieldDef::required("priority", FieldType::Utf8),
        FieldDef::required("labels", FieldType::Utf8), // JSON array
        FieldDef::optional("assignee", FieldType::Utf8),
        FieldDef::optional("project", FieldType::Utf8),
        FieldDef::optional("parent_id", FieldType::Utf8),
        FieldDef::required("created_at", FieldType::Int64),
        FieldDef::required("updated_at", FieldType::Int64),
        FieldDef::optional("closed_at", FieldType::Int64),
    ]
}

fn comments_field_defs() -> Vec<FieldDef> {
    vec![
        FieldDef::required("comment_id", FieldType::Utf8),
        FieldDef::required("issue_id", FieldType::Utf8),
        FieldDef::optional("author", FieldType::Utf8),
        FieldDef::required("body", FieldType::Utf8),
        FieldDef::required("created_at", FieldType::Int64),
        FieldDef::required("updated_at", FieldType::Int64),
    ]
}

// ── Record conversions ───────────────────────────────────────────────────

fn issue_to_record(issue: &Issue) -> Result<Record> {
    let labels_json =
        serde_json::to_string(&issue.labels).context("Failed to serialize labels")?;
    Ok(vec![
        ("issue_id".into(), FieldValue::Utf8(Some(issue.id.clone()))),
        ("number".into(), FieldValue::UInt64(Some(issue.number))),
        ("title".into(), FieldValue::Utf8(Some(issue.title.clone()))),
        (
            "description".into(),
            FieldValue::Utf8(Some(issue.description.clone())),
        ),
        (
            "status".into(),
            FieldValue::Utf8(Some(issue.status.as_str().to_string())),
        ),
        (
            "priority".into(),
            FieldValue::Utf8(Some(issue.priority.as_str().to_string())),
        ),
        ("labels".into(), FieldValue::Utf8(Some(labels_json))),
        ("assignee".into(), FieldValue::Utf8(issue.assignee.clone())),
        ("project".into(), FieldValue::Utf8(issue.project.clone())),
        ("parent_id".into(), FieldValue::Utf8(issue.parent_id.clone())),
        ("created_at".into(), FieldValue::Int64(Some(issue.created_at))),
        ("updated_at".into(), FieldValue::Int64(Some(issue.updated_at))),
        ("closed_at".into(), FieldValue::Int64(issue.closed_at)),
    ])
}

fn issue_from_record(r: &Record) -> Result<Issue> {
    let labels_json = record_get(r, "labels")
        .and_then(|v| v.as_str())
        .unwrap_or("[]");
    let labels: Vec<String> = serde_json::from_str(labels_json).unwrap_or_default();

    let number = record_get(r, "number")
        .and_then(|v| match v {
            FieldValue::UInt64(Some(n)) => Some(*n),
            _ => None,
        })
        .context("missing number")?;

    Ok(Issue {
        id: record_get(r, "issue_id")
            .and_then(|v| v.as_str())
            .context("missing issue_id")?
            .to_string(),
        number,
        title: record_get(r, "title")
            .and_then(|v| v.as_str())
            .context("missing title")?
            .to_string(),
        description: record_get(r, "description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        status: IssueStatus::from_str(
            record_get(r, "status")
                .and_then(|v| v.as_str())
                .unwrap_or("backlog"),
        ),
        priority: IssuePriority::from_str(
            record_get(r, "priority")
                .and_then(|v| v.as_str())
                .unwrap_or("no_priority"),
        ),
        labels,
        assignee: record_get(r, "assignee").and_then(|v| v.as_str()).map(String::from),
        project: record_get(r, "project").and_then(|v| v.as_str()).map(String::from),
        parent_id: record_get(r, "parent_id").and_then(|v| v.as_str()).map(String::from),
        created_at: record_get(r, "created_at")
            .and_then(|v| v.as_i64())
            .context("missing created_at")?,
        updated_at: record_get(r, "updated_at")
            .and_then(|v| v.as_i64())
            .context("missing updated_at")?,
        closed_at: record_get(r, "closed_at").and_then(|v| v.as_i64()),
    })
}

fn comment_to_record(c: &Comment) -> Record {
    vec![
        ("comment_id".into(), FieldValue::Utf8(Some(c.id.clone()))),
        ("issue_id".into(), FieldValue::Utf8(Some(c.issue_id.clone()))),
        ("author".into(), FieldValue::Utf8(c.author.clone())),
        ("body".into(), FieldValue::Utf8(Some(c.body.clone()))),
        ("created_at".into(), FieldValue::Int64(Some(c.created_at))),
        ("updated_at".into(), FieldValue::Int64(Some(c.updated_at))),
    ]
}

fn comment_from_record(r: &Record) -> Result<Comment> {
    Ok(Comment {
        id: record_get(r, "comment_id")
            .and_then(|v| v.as_str())
            .context("missing comment_id")?
            .to_string(),
        issue_id: record_get(r, "issue_id")
            .and_then(|v| v.as_str())
            .context("missing issue_id")?
            .to_string(),
        author: record_get(r, "author").and_then(|v| v.as_str()).map(String::from),
        body: record_get(r, "body")
            .and_then(|v| v.as_str())
            .context("missing body")?
            .to_string(),
        created_at: record_get(r, "created_at")
            .and_then(|v| v.as_i64())
            .context("missing created_at")?,
        updated_at: record_get(r, "updated_at")
            .and_then(|v| v.as_i64())
            .context("missing updated_at")?,
    })
}

// ── IssueStore ───────────────────────────────────────────────────────────

/// Persists issues to a backend-agnostic storage layer.
pub struct IssueStore<B: StorageBackend + 'static = brainwires_storage::LanceDatabase> {
    backend: Arc<B>,
    /// Optional BM25 full-text search index for keyword search.
    bm25: Option<BM25Search>,
}

impl<B: StorageBackend + 'static> Clone for IssueStore<B> {
    fn clone(&self) -> Self {
        // BM25Search is not Clone — the BM25 index is only used for search,
        // so cloned instances fall back to in-memory search.
        Self {
            backend: Arc::clone(&self.backend),
            bm25: None,
        }
    }
}

impl<B: StorageBackend + 'static> IssueStore<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend, bm25: None }
    }

    pub fn new_with_bm25(backend: Arc<B>, bm25: BM25Search) -> Self {
        Self { backend, bm25: Some(bm25) }
    }

    /// Ensure the issues table exists.
    pub async fn ensure_table(&self) -> Result<()> {
        self.backend
            .ensure_table(ISSUES_TABLE, &issues_field_defs())
            .await
    }

    /// Determine the next issue number (max existing + 1).
    pub async fn next_number(&self) -> Result<u64> {
        let records = self.backend.query(ISSUES_TABLE, None, None).await?;
        let max = records
            .iter()
            .filter_map(|r| match record_get(r, "number") {
                Some(FieldValue::UInt64(Some(n))) => Some(*n),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    /// Insert a new issue and add it to the BM25 index.
    pub async fn create(&self, issue: &Issue) -> Result<()> {
        let record = issue_to_record(issue)?;
        self.backend
            .insert(ISSUES_TABLE, vec![record])
            .await
            .context("Failed to create issue")?;

        if let Some(bm25) = &self.bm25 {
            let content = format!("{} {}", issue.title, issue.description);
            if let Err(e) = bm25.add_documents(vec![(issue.number, content, issue.id.clone())]) {
                tracing::warn!("BM25 index failed on create for issue {}: {}", issue.number, e);
            }
        }

        Ok(())
    }

    /// Get a single issue by UUID.
    pub async fn get(&self, id: &str) -> Result<Option<Issue>> {
        let filter = Filter::Eq("issue_id".into(), FieldValue::Utf8(Some(id.to_string())));
        let records = self.backend.query(ISSUES_TABLE, Some(&filter), Some(1)).await?;
        match records.first() {
            Some(r) => Ok(Some(issue_from_record(r)?)),
            None => Ok(None),
        }
    }

    /// Get an issue by its display number.
    pub async fn get_by_number(&self, number: u64) -> Result<Option<Issue>> {
        let filter = Filter::Eq("number".into(), FieldValue::UInt64(Some(number)));
        let records = self.backend.query(ISSUES_TABLE, Some(&filter), Some(1)).await?;
        match records.first() {
            Some(r) => Ok(Some(issue_from_record(r)?)),
            None => Ok(None),
        }
    }

    /// List issues with optional filters and offset-based pagination.
    ///
    /// `offset` is the number of records to skip; pass `None` or `Some(0)` for the first page.
    /// Returns `(issues, next_offset)` where `next_offset` is `Some(offset + limit)` if more
    /// records exist.
    pub async fn list(
        &self,
        project: Option<&str>,
        status: Option<&IssueStatus>,
        assignee: Option<&str>,
        label: Option<&str>,
        offset: Option<usize>,
        limit: usize,
    ) -> Result<(Vec<Issue>, Option<usize>)> {
        let offset = offset.unwrap_or(0);

        // Build filter from typed predicates (no label — handled in-memory after fetch)
        let mut filters = Vec::new();
        if let Some(p) = project {
            filters.push(Filter::Eq(
                "project".into(),
                FieldValue::Utf8(Some(p.to_string())),
            ));
        }
        if let Some(s) = status {
            filters.push(Filter::Eq(
                "status".into(),
                FieldValue::Utf8(Some(s.as_str().to_string())),
            ));
        }
        if let Some(a) = assignee {
            filters.push(Filter::Eq(
                "assignee".into(),
                FieldValue::Utf8(Some(a.to_string())),
            ));
        }

        let filter = match filters.len() {
            0 => None,
            1 => Some(filters.remove(0)),
            _ => Some(Filter::And(filters)),
        };

        // When a label filter is active, fetch without a backend limit so the in-memory
        // label filter sees all matching records before we apply offset + limit.
        let fetch_limit = if label.is_some() {
            None
        } else {
            Some(offset + limit + 1)
        };

        let records = self
            .backend
            .query(ISSUES_TABLE, filter.as_ref(), fetch_limit)
            .await?;

        let mut issues: Vec<Issue> = records
            .iter()
            .map(issue_from_record)
            .collect::<Result<Vec<_>>>()?;

        // Sort by updated_at descending (newest first)
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply label filter in-memory (labels stored as JSON array)
        if let Some(lbl) = label {
            issues.retain(|i| i.labels.iter().any(|l| l == lbl));
        }

        // Apply offset
        if offset > 0 {
            issues = issues.into_iter().skip(offset).collect();
        }

        // Determine next offset
        let next_offset = if issues.len() > limit {
            issues.truncate(limit);
            Some(offset + limit)
        } else {
            None
        };

        Ok((issues, next_offset))
    }

    /// Search issues using BM25 keyword search, falling back to in-memory substring match.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Issue>> {
        if let Some(bm25) = &self.bm25 {
            let results = bm25.search(query, limit).context("BM25 search failed")?;
            let mut issues = Vec::with_capacity(results.len());
            for hit in results {
                match self.get_by_number(hit.id).await {
                    Ok(Some(issue)) => issues.push(issue),
                    Ok(None) => tracing::warn!(
                        "BM25 returned issue number {} but it was not found in the store",
                        hit.id
                    ),
                    Err(e) => tracing::warn!("Failed to fetch issue {}: {}", hit.id, e),
                }
            }
            Ok(issues)
        } else {
            // Fallback: in-memory case-insensitive substring search
            let query_lower = query.to_lowercase();
            let (all_issues, _) = self.list(None, None, None, None, None, usize::MAX).await?;
            let mut matches: Vec<Issue> = all_issues
                .into_iter()
                .filter(|i| {
                    i.title.to_lowercase().contains(&query_lower)
                        || i.description.to_lowercase().contains(&query_lower)
                })
                .take(limit)
                .collect();
            // Rank: title matches first
            matches.sort_by_key(|i| {
                if i.title.to_lowercase().contains(&query_lower) { 0u8 } else { 1u8 }
            });
            Ok(matches)
        }
    }

    /// Apply a patch to an existing issue, persist it, and update the BM25 index.
    pub async fn update(&self, id: &str, patch: IssuePatch) -> Result<Issue> {
        let mut issue = self
            .get(id)
            .await?
            .with_context(|| format!("Issue not found: {}", id))?;

        if let Some(t) = patch.title {
            issue.title = t;
        }
        if let Some(d) = patch.description {
            issue.description = d;
        }
        if let Some(s) = patch.status {
            if s.is_closed() && issue.closed_at.is_none() {
                issue.closed_at = Some(Utc::now().timestamp());
            } else if !s.is_closed() {
                issue.closed_at = None;
            }
            issue.status = s;
        }
        if let Some(p) = patch.priority {
            issue.priority = p;
        }
        if let Some(l) = patch.labels {
            issue.labels = l;
        }
        if patch.clear_assignee.unwrap_or(false) {
            issue.assignee = None;
        } else if let Some(a) = patch.assignee {
            issue.assignee = Some(a);
        }
        if patch.clear_project.unwrap_or(false) {
            issue.project = None;
        } else if let Some(p) = patch.project {
            issue.project = Some(p);
        }
        if patch.clear_parent.unwrap_or(false) {
            issue.parent_id = None;
        } else if let Some(p) = patch.parent_id {
            issue.parent_id = Some(p);
        }
        issue.updated_at = Utc::now().timestamp();

        // Delete + re-insert (LanceDB upsert pattern)
        self.backend
            .delete(
                ISSUES_TABLE,
                &Filter::Eq("issue_id".into(), FieldValue::Utf8(Some(id.to_string()))),
            )
            .await
            .context("Failed to delete old issue record during update")?;
        let record = issue_to_record(&issue)?;
        self.backend
            .insert(ISSUES_TABLE, vec![record])
            .await
            .context("Failed to re-insert issue record during update")?;

        // Update BM25 index: remove old entry, add new one
        if let Some(bm25) = &self.bm25 {
            if let Err(e) = bm25.delete_by_id(issue.number) {
                tracing::warn!("BM25 delete failed for issue {} during update: {}", issue.number, e);
            }
            let content = format!("{} {}", issue.title, issue.description);
            if let Err(e) = bm25.add_documents(vec![(issue.number, content, issue.id.clone())]) {
                tracing::warn!("BM25 index failed on update for issue {}: {}", issue.number, e);
            }
        }

        Ok(issue)
    }

    /// Delete an issue by UUID and remove it from the BM25 index.
    pub async fn delete(&self, id: &str) -> Result<()> {
        // Fetch the issue first so we can get the number for BM25 removal
        let number = if let Some(_bm25) = &self.bm25 {
            match self.get(id).await {
                Ok(Some(issue)) => Some(issue.number),
                _ => None,
            }
        } else {
            None
        };

        let filter = Filter::Eq("issue_id".into(), FieldValue::Utf8(Some(id.to_string())));
        self.backend
            .delete(ISSUES_TABLE, &filter)
            .await
            .context("Failed to delete issue")?;

        if let (Some(bm25), Some(num)) = (&self.bm25, number) {
            if let Err(e) = bm25.delete_by_id(num) {
                tracing::warn!("BM25 delete failed for issue {} during delete: {}", num, e);
            }
        }

        Ok(())
    }
}

// ── CommentStore ─────────────────────────────────────────────────────────

/// Persists comments to a backend-agnostic storage layer.
pub struct CommentStore<B: StorageBackend + 'static = brainwires_storage::LanceDatabase> {
    backend: Arc<B>,
}

impl<B: StorageBackend + 'static> Clone for CommentStore<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<B: StorageBackend + 'static> CommentStore<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    /// Ensure the comments table exists.
    pub async fn ensure_table(&self) -> Result<()> {
        self.backend
            .ensure_table(COMMENTS_TABLE, &comments_field_defs())
            .await
    }

    /// Add a comment.
    pub async fn add(&self, comment: &Comment) -> Result<()> {
        self.backend
            .insert(COMMENTS_TABLE, vec![comment_to_record(comment)])
            .await
            .context("Failed to add comment")
    }

    /// Get a single comment by UUID.
    pub async fn get(&self, id: &str) -> Result<Option<Comment>> {
        let filter =
            Filter::Eq("comment_id".into(), FieldValue::Utf8(Some(id.to_string())));
        let records = self
            .backend
            .query(COMMENTS_TABLE, Some(&filter), Some(1))
            .await?;
        match records.first() {
            Some(r) => Ok(Some(comment_from_record(r)?)),
            None => Ok(None),
        }
    }

    /// List comments for an issue with offset-based pagination.
    ///
    /// Returns `(comments, next_offset)` where `next_offset` is `Some(offset + limit)` if more
    /// records exist.
    pub async fn list_for_issue(
        &self,
        issue_id: &str,
        offset: Option<usize>,
        limit: usize,
    ) -> Result<(Vec<Comment>, Option<usize>)> {
        let offset = offset.unwrap_or(0);

        let filter = Filter::Eq(
            "issue_id".into(),
            FieldValue::Utf8(Some(issue_id.to_string())),
        );
        let records = self
            .backend
            .query(COMMENTS_TABLE, Some(&filter), Some(offset + limit + 1))
            .await?;

        let mut comments: Vec<Comment> = records
            .iter()
            .map(comment_from_record)
            .collect::<Result<Vec<_>>>()?;

        // Sort oldest first
        comments.sort_by_key(|c| c.created_at);

        // Apply offset
        if offset > 0 {
            comments = comments.into_iter().skip(offset).collect();
        }

        let next_offset = if comments.len() > limit {
            comments.truncate(limit);
            Some(offset + limit)
        } else {
            None
        };

        Ok((comments, next_offset))
    }

    /// Delete a comment by UUID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let filter = Filter::Eq("comment_id".into(), FieldValue::Utf8(Some(id.to_string())));
        self.backend
            .delete(COMMENTS_TABLE, &filter)
            .await
            .context("Failed to delete comment")
    }

    /// Delete all comments for an issue.
    pub async fn delete_by_issue(&self, issue_id: &str) -> Result<()> {
        let filter = Filter::Eq(
            "issue_id".into(),
            FieldValue::Utf8(Some(issue_id.to_string())),
        );
        self.backend
            .delete(COMMENTS_TABLE, &filter)
            .await
            .context("Failed to delete comments for issue")
    }
}
