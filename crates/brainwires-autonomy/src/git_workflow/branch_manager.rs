//! Branch management for autonomous fix workflows.

use serde::{Deserialize, Serialize};

/// Information about a created branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub base_branch: String,
    pub worktree_path: Option<String>,
}

/// Manages branch creation for autonomous fix workflows.
pub struct BranchManager {
    branch_prefix: String,
}

impl BranchManager {
    pub fn new(branch_prefix: String) -> Self {
        Self { branch_prefix }
    }

    /// Create a branch name from an issue number and slug.
    pub fn branch_name(&self, issue_number: u64, slug: &str) -> String {
        let clean_slug: String = slug
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .take(40)
            .collect();
        let clean_slug = clean_slug.trim_matches('-').to_string();
        format!("{}issue-{}-{}", self.branch_prefix, issue_number, clean_slug)
    }

    /// Create a new branch from the current HEAD.
    pub async fn create_branch(
        &self,
        repo_path: &str,
        branch_name: &str,
        base_branch: &str,
    ) -> anyhow::Result<BranchInfo> {
        // Fetch latest
        let _ = tokio::process::Command::new("git")
            .args(["fetch", "origin", base_branch])
            .current_dir(repo_path)
            .output()
            .await;

        // Create branch
        let output = tokio::process::Command::new("git")
            .args(["checkout", "-b", branch_name, &format!("origin/{base_branch}")])
            .current_dir(repo_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create branch {branch_name}: {stderr}");
        }

        Ok(BranchInfo {
            name: branch_name.to_string(),
            base_branch: base_branch.to_string(),
            worktree_path: None,
        })
    }

    /// Push a branch to the remote.
    pub async fn push_branch(
        &self,
        repo_path: &str,
        branch_name: &str,
    ) -> anyhow::Result<()> {
        let output = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", branch_name])
            .current_dir(repo_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to push branch {branch_name}: {stderr}");
        }

        Ok(())
    }

    /// Clean up a branch after merge.
    pub async fn delete_branch(
        &self,
        repo_path: &str,
        branch_name: &str,
    ) -> anyhow::Result<()> {
        let _ = tokio::process::Command::new("git")
            .args(["branch", "-D", branch_name])
            .current_dir(repo_path)
            .output()
            .await;

        let _ = tokio::process::Command::new("git")
            .args(["push", "origin", "--delete", branch_name])
            .current_dir(repo_path)
            .output()
            .await;

        Ok(())
    }
}
