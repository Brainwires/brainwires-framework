//! Workflow trigger sources — webhook events, programmatic triggers, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::forge::{Comment, CommitRef, Issue, PullRequest, RepoRef};

/// Events that can trigger a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    /// A new issue was opened.
    IssueOpened {
        /// The opened issue.
        issue: Issue,
        /// Repository where the issue was opened.
        repo: RepoRef,
    },
    /// A comment was added to an issue.
    IssueCommented {
        /// The issue that was commented on.
        issue: Issue,
        /// The new comment.
        comment: Comment,
        /// Repository of the issue.
        repo: RepoRef,
    },
    /// Commits were pushed to a branch.
    PushReceived {
        /// Branch that received the push.
        branch: String,
        /// Commits that were pushed.
        commits: Vec<CommitRef>,
        /// Repository of the push.
        repo: RepoRef,
    },
    /// A PR review was approved.
    PrReviewApproved {
        /// The approved pull request.
        pr: PullRequest,
        /// Repository of the PR.
        repo: RepoRef,
    },
    /// A manually triggered event.
    Manual {
        /// Description of the manual trigger.
        description: String,
        /// Target repository.
        repo: RepoRef,
    },
}

/// Trait for event sources that emit workflow events.
#[async_trait]
pub trait WorkflowTrigger: Send + Sync {
    /// Start listening for events and send them to the given channel.
    async fn start(&self, tx: mpsc::Sender<WorkflowEvent>) -> anyhow::Result<()>;
}

/// Programmatic trigger — allows sending events directly from code.
pub struct ProgrammaticTrigger {
    tx: Option<mpsc::Sender<WorkflowEvent>>,
}

impl ProgrammaticTrigger {
    /// Create a new programmatic trigger (not yet connected to a channel).
    pub fn new() -> Self {
        Self { tx: None }
    }

    /// Send an event manually.
    pub async fn emit(&self, event: WorkflowEvent) -> anyhow::Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(event).await.map_err(|e| anyhow::anyhow!("Failed to send event: {e}"))?;
        }
        Ok(())
    }
}

impl Default for ProgrammaticTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowTrigger for ProgrammaticTrigger {
    async fn start(&self, _tx: mpsc::Sender<WorkflowEvent>) -> anyhow::Result<()> {
        // Programmatic trigger doesn't listen — events are emitted via emit()
        Ok(())
    }
}
