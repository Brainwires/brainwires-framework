//! Git Source Control Management State
//!
//! Manages the state for the TUI Git SCM integration.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Git file status
#[derive(Debug, Clone, PartialEq)]
pub enum GitFileStatus {
    /// Modified but not staged
    Modified,
    /// Staged for commit
    Staged,
    /// Both staged and has unstaged changes
    StagedModified,
    /// Untracked file
    Untracked,
    /// Deleted file
    Deleted,
    /// Deleted and staged
    StagedDeleted,
    /// Renamed file
    Renamed,
    /// Copied file
    Copied,
    /// Unmerged (conflict)
    Conflict,
    /// Ignored file
    Ignored,
}

impl GitFileStatus {
    /// Get the status indicator character (like git status --short)
    pub fn indicator(&self) -> &'static str {
        match self {
            GitFileStatus::Modified => " M",
            GitFileStatus::Staged => "M ",
            GitFileStatus::StagedModified => "MM",
            GitFileStatus::Untracked => "??",
            GitFileStatus::Deleted => " D",
            GitFileStatus::StagedDeleted => "D ",
            GitFileStatus::Renamed => "R ",
            GitFileStatus::Copied => "C ",
            GitFileStatus::Conflict => "UU",
            GitFileStatus::Ignored => "!!",
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            GitFileStatus::Modified => "Modified",
            GitFileStatus::Staged => "Staged",
            GitFileStatus::StagedModified => "Staged+Modified",
            GitFileStatus::Untracked => "Untracked",
            GitFileStatus::Deleted => "Deleted",
            GitFileStatus::StagedDeleted => "Staged Deletion",
            GitFileStatus::Renamed => "Renamed",
            GitFileStatus::Copied => "Copied",
            GitFileStatus::Conflict => "Conflict",
            GitFileStatus::Ignored => "Ignored",
        }
    }
}

/// A file entry in the Git SCM view
#[derive(Debug, Clone)]
pub struct GitFileEntry {
    /// File path relative to repo root
    pub path: PathBuf,
    /// Git status
    pub status: GitFileStatus,
    /// Original path (for renamed files)
    pub original_path: Option<PathBuf>,
    /// Whether the file is selected for batch operations
    pub selected: bool,
}

/// Current panel/section in the SCM view
#[derive(Debug, Clone, PartialEq)]
pub enum ScmPanel {
    /// Staged changes panel
    Staged,
    /// Unstaged changes panel
    Changes,
    /// Untracked files panel
    Untracked,
}

/// Git operation mode
#[derive(Debug, Clone, PartialEq)]
pub enum GitOperationMode {
    /// Normal browsing mode
    Browse,
    /// Entering commit message
    CommitMessage,
    /// Confirming an operation
    Confirm { message: String, action: GitAction },
}

/// Git actions that can be performed
#[derive(Debug, Clone, PartialEq)]
pub enum GitAction {
    Push,
    Pull,
    Fetch,
    Commit,
    ResetHard,
    DiscardAll,
    Discard(Vec<PathBuf>),
}

/// State for the Git SCM view
#[derive(Debug, Clone)]
pub struct GitScmState {
    /// Repository root path
    pub repo_root: PathBuf,
    /// Current branch name
    pub current_branch: String,
    /// Remote tracking branch (if any)
    pub upstream_branch: Option<String>,
    /// Ahead/behind counts
    pub ahead: usize,
    pub behind: usize,
    /// Staged files
    pub staged_files: Vec<GitFileEntry>,
    /// Changed (unstaged) files
    pub changed_files: Vec<GitFileEntry>,
    /// Untracked files
    pub untracked_files: Vec<GitFileEntry>,
    /// Current panel
    pub current_panel: ScmPanel,
    /// Cursor index within current panel
    pub cursor_index: usize,
    /// Scroll offset
    pub scroll: u16,
    /// Operation mode
    pub mode: GitOperationMode,
    /// Commit message being typed
    pub commit_message: String,
    /// Status message to display
    pub status_message: Option<String>,
    /// Error message
    pub error_message: Option<String>,
    /// Last refresh time
    pub last_refresh: std::time::Instant,
}

impl GitScmState {
    /// Create a new GitScmState by detecting the repo root
    pub fn new() -> Result<Self> {
        let repo_root = Self::find_repo_root()?;
        let mut state = Self {
            repo_root,
            current_branch: String::new(),
            upstream_branch: None,
            ahead: 0,
            behind: 0,
            staged_files: Vec::new(),
            changed_files: Vec::new(),
            untracked_files: Vec::new(),
            current_panel: ScmPanel::Changes,
            cursor_index: 0,
            scroll: 0,
            mode: GitOperationMode::Browse,
            commit_message: String::new(),
            status_message: None,
            error_message: None,
            last_refresh: std::time::Instant::now(),
        };
        state.refresh()?;
        Ok(state)
    }

    /// Find the repository root
    fn find_repo_root() -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to run git rev-parse")?;

        if !output.status.success() {
            anyhow::bail!("Not in a git repository");
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path))
    }

    /// Refresh all git status information
    pub fn refresh(&mut self) -> Result<()> {
        self.refresh_branch_info()?;
        self.refresh_file_status()?;
        self.last_refresh = std::time::Instant::now();
        self.error_message = None;
        Ok(())
    }

    /// Refresh branch information
    fn refresh_branch_info(&mut self) -> Result<()> {
        // Get current branch
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.repo_root)
            .output()?;
        self.current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // If in detached HEAD state, get the short SHA
        if self.current_branch.is_empty() {
            let output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .current_dir(&self.repo_root)
                .output()?;
            self.current_branch = format!("HEAD:{}", String::from_utf8_lossy(&output.stdout).trim());
        }

        // Get upstream branch and ahead/behind
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "@{u}"])
            .current_dir(&self.repo_root)
            .output()?;

        if output.status.success() {
            self.upstream_branch = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());

            // Get ahead/behind counts
            let output = Command::new("git")
                .args(["rev-list", "--left-right", "--count", "@{u}...HEAD"])
                .current_dir(&self.repo_root)
                .output()?;

            if output.status.success() {
                let counts = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = counts.trim().split_whitespace().collect();
                if parts.len() == 2 {
                    self.behind = parts[0].parse().unwrap_or(0);
                    self.ahead = parts[1].parse().unwrap_or(0);
                }
            }
        } else {
            self.upstream_branch = None;
            self.ahead = 0;
            self.behind = 0;
        }

        Ok(())
    }

    /// Refresh file status
    fn refresh_file_status(&mut self) -> Result<()> {
        self.staged_files.clear();
        self.changed_files.clear();
        self.untracked_files.clear();

        // Get status in porcelain format
        let output = Command::new("git")
            .args(["status", "--porcelain=v1"])
            .current_dir(&self.repo_root)
            .output()?;

        let status_output = String::from_utf8_lossy(&output.stdout);

        for line in status_output.lines() {
            if line.len() < 4 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let path_part = &line[3..];

            // Handle renamed files (format: "R  old -> new")
            let (path, original_path) = if path_part.contains(" -> ") {
                let parts: Vec<&str> = path_part.split(" -> ").collect();
                (
                    PathBuf::from(parts.get(1).unwrap_or(&"")),
                    Some(PathBuf::from(parts.get(0).unwrap_or(&""))),
                )
            } else {
                (PathBuf::from(path_part), None)
            };

            // Determine status and which list to add to
            match (index_status, worktree_status) {
                ('?', '?') => {
                    self.untracked_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Untracked,
                        original_path,
                        selected: false,
                    });
                }
                ('!', '!') => {
                    // Ignored - skip
                }
                ('M', ' ') | ('A', ' ') => {
                    self.staged_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Staged,
                        original_path,
                        selected: false,
                    });
                }
                ('M', 'M') => {
                    self.staged_files.push(GitFileEntry {
                        path: path.clone(),
                        status: GitFileStatus::StagedModified,
                        original_path: original_path.clone(),
                        selected: false,
                    });
                    self.changed_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Modified,
                        original_path,
                        selected: false,
                    });
                }
                (' ', 'M') => {
                    self.changed_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Modified,
                        original_path,
                        selected: false,
                    });
                }
                ('D', ' ') => {
                    self.staged_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::StagedDeleted,
                        original_path,
                        selected: false,
                    });
                }
                (' ', 'D') => {
                    self.changed_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Deleted,
                        original_path,
                        selected: false,
                    });
                }
                ('R', ' ') | ('R', 'M') => {
                    self.staged_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Renamed,
                        original_path,
                        selected: false,
                    });
                }
                ('C', ' ') => {
                    self.staged_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Copied,
                        original_path,
                        selected: false,
                    });
                }
                ('U', 'U') | ('A', 'A') | ('D', 'D') | ('A', 'U') | ('U', 'A') | ('D', 'U') | ('U', 'D') => {
                    self.changed_files.push(GitFileEntry {
                        path,
                        status: GitFileStatus::Conflict,
                        original_path,
                        selected: false,
                    });
                }
                _ => {
                    // Other statuses - add to changed files as modified
                    if worktree_status != ' ' {
                        self.changed_files.push(GitFileEntry {
                            path,
                            status: GitFileStatus::Modified,
                            original_path,
                            selected: false,
                        });
                    }
                }
            }
        }

        // Adjust cursor if out of bounds
        self.adjust_cursor();

        Ok(())
    }

    /// Get the list for the current panel
    pub fn current_list(&self) -> &[GitFileEntry] {
        match self.current_panel {
            ScmPanel::Staged => &self.staged_files,
            ScmPanel::Changes => &self.changed_files,
            ScmPanel::Untracked => &self.untracked_files,
        }
    }

    /// Get mutable list for the current panel
    fn current_list_mut(&mut self) -> &mut Vec<GitFileEntry> {
        match self.current_panel {
            ScmPanel::Staged => &mut self.staged_files,
            ScmPanel::Changes => &mut self.changed_files,
            ScmPanel::Untracked => &mut self.untracked_files,
        }
    }

    /// Get the currently selected file entry
    pub fn current_entry(&self) -> Option<&GitFileEntry> {
        self.current_list().get(self.cursor_index)
    }

    /// Adjust cursor to be within bounds
    fn adjust_cursor(&mut self) {
        let len = self.current_list().len();
        if len == 0 {
            self.cursor_index = 0;
        } else if self.cursor_index >= len {
            self.cursor_index = len.saturating_sub(1);
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
        }
        self.adjust_scroll();
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let len = self.current_list().len();
        if self.cursor_index + 1 < len {
            self.cursor_index += 1;
        }
        self.adjust_scroll();
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor_index = self.cursor_index.saturating_sub(page_size);
        self.adjust_scroll();
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        let len = self.current_list().len();
        self.cursor_index = (self.cursor_index + page_size).min(len.saturating_sub(1));
        self.adjust_scroll();
    }

    /// Adjust scroll to keep cursor visible
    fn adjust_scroll(&mut self) {
        let visible_height = 15u16;
        let cursor = self.cursor_index as u16;

        if cursor < self.scroll {
            self.scroll = cursor;
        } else if cursor >= self.scroll + visible_height {
            self.scroll = cursor.saturating_sub(visible_height - 1);
        }
    }

    /// Switch to next panel
    pub fn next_panel(&mut self) {
        self.current_panel = match self.current_panel {
            ScmPanel::Staged => ScmPanel::Changes,
            ScmPanel::Changes => ScmPanel::Untracked,
            ScmPanel::Untracked => ScmPanel::Staged,
        };
        self.cursor_index = 0;
        self.scroll = 0;
    }

    /// Switch to previous panel
    pub fn prev_panel(&mut self) {
        self.current_panel = match self.current_panel {
            ScmPanel::Staged => ScmPanel::Untracked,
            ScmPanel::Changes => ScmPanel::Staged,
            ScmPanel::Untracked => ScmPanel::Changes,
        };
        self.cursor_index = 0;
        self.scroll = 0;
    }

    /// Toggle selection on current entry
    pub fn toggle_selection(&mut self) {
        let idx = self.cursor_index;
        let list = match self.current_panel {
            ScmPanel::Staged => &mut self.staged_files,
            ScmPanel::Changes => &mut self.changed_files,
            ScmPanel::Untracked => &mut self.untracked_files,
        };
        if let Some(entry) = list.get_mut(idx) {
            entry.selected = !entry.selected;
        }
    }

    /// Select all in current panel
    pub fn select_all(&mut self) {
        for entry in self.current_list_mut() {
            entry.selected = true;
        }
    }

    /// Clear selection in current panel
    pub fn clear_selection(&mut self) {
        for entry in self.current_list_mut() {
            entry.selected = false;
        }
    }

    /// Stage the current file or selected files
    pub fn stage_current(&mut self) -> Result<()> {
        let files_to_stage: Vec<PathBuf> = match self.current_panel {
            ScmPanel::Changes | ScmPanel::Untracked => {
                let list = self.current_list();
                let selected: Vec<PathBuf> = list
                    .iter()
                    .filter(|e| e.selected)
                    .map(|e| e.path.clone())
                    .collect();

                if selected.is_empty() {
                    // Stage current file only
                    list.get(self.cursor_index)
                        .map(|e| vec![e.path.clone()])
                        .unwrap_or_default()
                } else {
                    selected
                }
            }
            ScmPanel::Staged => return Ok(()), // Already staged
        };

        if files_to_stage.is_empty() {
            return Ok(());
        }

        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(files_to_stage.iter().map(|p| p.to_string_lossy().to_string()));

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to stage: {}", stderr));
        } else {
            self.status_message = Some(format!("Staged {} file(s)", files_to_stage.len()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Unstage the current file or selected files
    pub fn unstage_current(&mut self) -> Result<()> {
        let files_to_unstage: Vec<PathBuf> = match self.current_panel {
            ScmPanel::Staged => {
                let list = self.current_list();
                let selected: Vec<PathBuf> = list
                    .iter()
                    .filter(|e| e.selected)
                    .map(|e| e.path.clone())
                    .collect();

                if selected.is_empty() {
                    list.get(self.cursor_index)
                        .map(|e| vec![e.path.clone()])
                        .unwrap_or_default()
                } else {
                    selected
                }
            }
            _ => return Ok(()), // Not staged
        };

        if files_to_unstage.is_empty() {
            return Ok(());
        }

        let mut args = vec!["reset".to_string(), "HEAD".to_string(), "--".to_string()];
        args.extend(files_to_unstage.iter().map(|p| p.to_string_lossy().to_string()));

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to unstage: {}", stderr));
        } else {
            self.status_message = Some(format!("Unstaged {} file(s)", files_to_unstage.len()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Discard changes to current file
    pub fn discard_current(&mut self) -> Result<()> {
        let file = match self.current_panel {
            ScmPanel::Changes => {
                self.current_entry().map(|e| e.path.clone())
            }
            _ => None,
        };

        let Some(file) = file else {
            return Ok(());
        };

        let output = Command::new("git")
            .args(["checkout", "--", &file.to_string_lossy()])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to discard: {}", stderr));
        } else {
            self.status_message = Some(format!("Discarded changes to {}", file.display()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Stage all files
    pub fn stage_all(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to stage all: {}", stderr));
        } else {
            self.status_message = Some("Staged all changes".to_string());
        }

        self.refresh()?;
        Ok(())
    }

    /// Unstage all files
    pub fn unstage_all(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["reset", "HEAD"])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to unstage all: {}", stderr));
        } else {
            self.status_message = Some("Unstaged all changes".to_string());
        }

        self.refresh()?;
        Ok(())
    }

    /// Start commit message entry
    pub fn start_commit(&mut self) {
        if self.staged_files.is_empty() {
            self.error_message = Some("Nothing staged to commit".to_string());
            return;
        }
        self.mode = GitOperationMode::CommitMessage;
        self.commit_message.clear();
    }

    /// Perform the commit
    pub fn do_commit(&mut self) -> Result<()> {
        if self.commit_message.trim().is_empty() {
            self.error_message = Some("Commit message cannot be empty".to_string());
            return Ok(());
        }

        let output = Command::new("git")
            .args(["commit", "-m", &self.commit_message])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Commit failed: {}", stderr));
        } else {
            self.status_message = Some("Committed successfully".to_string());
            self.commit_message.clear();
        }

        self.mode = GitOperationMode::Browse;
        self.refresh()?;
        Ok(())
    }

    /// Cancel current operation
    pub fn cancel_operation(&mut self) {
        self.mode = GitOperationMode::Browse;
        self.commit_message.clear();
    }

    /// Push to remote
    pub fn push(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["push"])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Push failed: {}", stderr));
        } else {
            self.status_message = Some("Pushed successfully".to_string());
        }

        self.refresh()?;
        Ok(())
    }

    /// Pull from remote
    pub fn pull(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["pull"])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Pull failed: {}", stderr));
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Already up to date") {
                self.status_message = Some("Already up to date".to_string());
            } else {
                self.status_message = Some("Pulled successfully".to_string());
            }
        }

        self.refresh()?;
        Ok(())
    }

    /// Fetch from remote
    pub fn fetch(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["fetch"])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Fetch failed: {}", stderr));
        } else {
            self.status_message = Some("Fetched successfully".to_string());
        }

        self.refresh()?;
        Ok(())
    }

    /// Get total change count
    pub fn total_changes(&self) -> usize {
        self.staged_files.len() + self.changed_files.len() + self.untracked_files.len()
    }

    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> bool {
        !self.staged_files.is_empty() || !self.changed_files.is_empty()
    }

    /// Get diff for current file
    pub fn get_current_diff(&self) -> Result<String> {
        let entry = self.current_entry();
        let Some(entry) = entry else {
            return Ok(String::new());
        };

        let path_str = entry.path.to_string_lossy().to_string();
        let args: Vec<&str> = match self.current_panel {
            ScmPanel::Staged => vec!["diff", "--cached", "--", &path_str],
            ScmPanel::Changes => vec!["diff", "--", &path_str],
            ScmPanel::Untracked => {
                // For untracked files, show the file content
                let path = self.repo_root.join(&entry.path);
                return std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e));
            }
        };

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Stage specific files
    pub async fn stage_files(&mut self, files: &[PathBuf]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(files.iter().map(|p| p.to_string_lossy().to_string()));

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to stage: {}", stderr));
        } else {
            self.status_message = Some(format!("Staged {} file(s)", files.len()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Unstage specific files
    pub async fn unstage_files(&mut self, files: &[PathBuf]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let mut args = vec!["reset".to_string(), "HEAD".to_string(), "--".to_string()];
        args.extend(files.iter().map(|p| p.to_string_lossy().to_string()));

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to unstage: {}", stderr));
        } else {
            self.status_message = Some(format!("Unstaged {} file(s)", files.len()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Discard changes to specific files
    pub async fn discard_files(&mut self, files: &[PathBuf]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let mut args = vec!["checkout".to_string(), "--".to_string()];
        args.extend(files.iter().map(|p| p.to_string_lossy().to_string()));

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.error_message = Some(format!("Failed to discard: {}", stderr));
        } else {
            self.status_message = Some(format!("Discarded {} file(s)", files.len()));
        }

        self.refresh()?;
        Ok(())
    }

    /// Execute a git action
    pub async fn execute_action(&mut self, action: GitAction) -> Result<()> {
        match action {
            GitAction::Push => self.push()?,
            GitAction::Pull => self.pull()?,
            GitAction::Fetch => self.fetch()?,
            GitAction::Commit => self.do_commit()?,
            GitAction::Discard(files) => self.discard_files(&files).await?,
            GitAction::ResetHard | GitAction::DiscardAll => {
                // Not implemented for safety
                self.error_message = Some("Operation not implemented".to_string());
            }
        }
        self.mode = GitOperationMode::Browse;
        Ok(())
    }

    /// Clear status and error messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_file_status_indicator() {
        assert_eq!(GitFileStatus::Modified.indicator(), " M");
        assert_eq!(GitFileStatus::Staged.indicator(), "M ");
        assert_eq!(GitFileStatus::Untracked.indicator(), "??");
    }

    #[test]
    fn test_git_file_status_description() {
        assert_eq!(GitFileStatus::Modified.description(), "Modified");
        assert_eq!(GitFileStatus::Staged.description(), "Staged");
        assert_eq!(GitFileStatus::Conflict.description(), "Conflict");
    }

    #[test]
    fn test_scm_panel_cycle() {
        let mut state = GitScmState {
            repo_root: PathBuf::new(),
            current_branch: String::new(),
            upstream_branch: None,
            ahead: 0,
            behind: 0,
            staged_files: Vec::new(),
            changed_files: Vec::new(),
            untracked_files: Vec::new(),
            current_panel: ScmPanel::Staged,
            cursor_index: 0,
            scroll: 0,
            mode: GitOperationMode::Browse,
            commit_message: String::new(),
            status_message: None,
            error_message: None,
            last_refresh: std::time::Instant::now(),
        };

        assert_eq!(state.current_panel, ScmPanel::Staged);
        state.next_panel();
        assert_eq!(state.current_panel, ScmPanel::Changes);
        state.next_panel();
        assert_eq!(state.current_panel, ScmPanel::Untracked);
        state.next_panel();
        assert_eq!(state.current_panel, ScmPanel::Staged);
    }
}
