//! Workflow trigger sources — webhook events, programmatic triggers, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::forge::{Comment, CommitRef, Issue, PullRequest, RepoRef};

/// Events that can trigger a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    IssueOpened { issue: Issue, repo: RepoRef },
    IssueCommented { issue: Issue, comment: Comment, repo: RepoRef },
    PushReceived { branch: String, commits: Vec<CommitRef>, repo: RepoRef },
    PrReviewApproved { pr: PullRequest, repo: RepoRef },
    Manual { description: String, repo: RepoRef },
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
