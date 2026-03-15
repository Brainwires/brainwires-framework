//! Persistent state for crash recovery — checkpoints and resume tracking.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// State of the git repository at crash time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    /// Current branch name.
    pub branch: String,
    /// Last commit hash.
    pub last_commit: String,
    /// Files with uncommitted changes.
    pub dirty_files: Vec<String>,
    /// Whether there are uncommitted changes.
    pub has_uncommitted_changes: bool,
}

/// Checkpoint persisted before each improvement cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleCheckpoint {
    /// Index of the current cycle (0-based).
    pub cycle_index: u32,
    /// Total number of cycles planned.
    pub total_cycles: u32,
    /// ID of the task being executed.
    pub task_id: Option<String>,
    /// Strategy name of the current task.
    pub strategy: Option<String>,
    /// Git state at checkpoint time.
    pub git_state: GitState,
    /// Timestamp of the checkpoint.
    pub timestamp: DateTime<Utc>,
}

/// Crash context captured when a self-improvement session fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashContext {
    /// When the crash occurred.
    pub crash_time: DateTime<Utc>,
    /// Process exit code, if available.
    pub exit_code: Option<i32>,
    /// Signal that killed the process, if applicable.
    pub signal: Option<i32>,
    /// Last N lines of stderr output.
    pub stderr_tail: String,
    /// Index of the cycle that was running when the crash occurred.
    pub last_cycle_index: u32,
    /// ID of the task that was being executed.
    pub last_task_id: Option<String>,
    /// Strategy that was running.
    pub last_strategy: Option<String>,
    /// Working directory of the session.
    pub working_directory: String,
    /// Git state at crash time.
    pub git_state: GitState,
}

/// Recovery state file persisted across process restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryState {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Unique crash identifier.
    pub crash_id: String,
    /// The crash context.
    pub crash_context: CrashContext,
    /// Number of fix attempts already made for this crash.
    pub fix_attempts: u32,
    /// Maximum fix attempts allowed.
    pub max_fix_attempts: u32,
    /// Recovery plan, populated after diagnosis.
    pub recovery_plan: Option<RecoveryPlanState>,
}

/// Serializable recovery plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlanState {
    /// Root cause analysis from the AI.
    pub root_cause: String,
    /// Strategy to apply.
    pub fix_strategy: String,
    /// Files that need fixing.
    pub files_to_fix: Vec<String>,
    /// Whether a git rollback is needed before fixing.
    pub rollback_needed: bool,
    /// Cycle index to resume from after fix.
    pub resume_from_cycle: u32,
}

impl RecoveryState {
    /// Load recovery state from a file, returning `None` if the file doesn't exist.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    /// Save recovery state to a file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Delete the recovery state file.
    pub fn cleanup(path: &Path) -> anyhow::Result<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if this is a meta-crash (crash handler itself crashed).
    pub fn is_meta_crash(&self) -> bool {
        self.fix_attempts >= self.max_fix_attempts
    }
}

impl CycleCheckpoint {
    /// Save checkpoint to a file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load checkpoint from a file.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let checkpoint: Self = serde_json::from_str(&content)?;
        Ok(Some(checkpoint))
    }
}

/// Capture the current git state of a repository.
pub async fn capture_git_state(repo_path: &str) -> anyhow::Result<GitState> {
    let branch = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .await?;
    let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();

    let commit = tokio::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .await?;
    let last_commit = String::from_utf8_lossy(&commit.stdout).trim().to_string();

    let status = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .await?;
    let status_output = String::from_utf8_lossy(&status.stdout);
    let dirty_files: Vec<String> = status_output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect();
    let has_uncommitted_changes = !dirty_files.is_empty();

    Ok(GitState {
        branch,
        last_commit,
        dirty_files,
        has_uncommitted_changes,
    })
}

/// Derive a checkpoint file path from the recovery state file path.
pub fn checkpoint_path(state_file: &Path) -> PathBuf {
    let stem = state_file.file_stem().unwrap_or_default().to_string_lossy();
    state_file.with_file_name(format!("{stem}-checkpoint.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_state_roundtrip() {
        let state = RecoveryState {
            version: 1,
            crash_id: "test-123".to_string(),
            crash_context: CrashContext {
                crash_time: Utc::now(),
                exit_code: Some(1),
                signal: None,
                stderr_tail: "panicked at 'test'".to_string(),
                last_cycle_index: 3,
                last_task_id: Some("task-1".to_string()),
                last_strategy: Some("clippy".to_string()),
                working_directory: "/tmp/test".to_string(),
                git_state: GitState {
                    branch: "self-improve/test".to_string(),
                    last_commit: "abc123".to_string(),
                    dirty_files: vec!["src/main.rs".to_string()],
                    has_uncommitted_changes: true,
                },
            },
            fix_attempts: 0,
            max_fix_attempts: 3,
            recovery_plan: None,
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: RecoveryState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.crash_id, "test-123");
        assert_eq!(deserialized.crash_context.last_cycle_index, 3);
    }

    #[test]
    fn is_meta_crash_when_max_attempts_reached() {
        let state = RecoveryState {
            version: 1,
            crash_id: "test".to_string(),
            crash_context: CrashContext {
                crash_time: Utc::now(),
                exit_code: None,
                signal: None,
                stderr_tail: String::new(),
                last_cycle_index: 0,
                last_task_id: None,
                last_strategy: None,
                working_directory: ".".to_string(),
                git_state: GitState {
                    branch: "main".to_string(),
                    last_commit: "abc".to_string(),
                    dirty_files: Vec::new(),
                    has_uncommitted_changes: false,
                },
            },
            fix_attempts: 3,
            max_fix_attempts: 3,
            recovery_plan: None,
        };
        assert!(state.is_meta_crash());
    }
}
