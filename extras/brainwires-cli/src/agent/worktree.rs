//! Git-worktree isolation primitive for background agents.
//!
//! Provides a RAII [`WorktreeGuard`] that creates a scratch `git worktree`
//! under `~/.brainwires/worktrees/<uuid>/` and removes it on drop. Agents
//! that want isolation can spawn with their working directory pointed at
//! the guard's path; the guard's [`Drop`] impl runs `git worktree remove`
//! so orphans don't accumulate.
//!
//! This is a primitive; full `Agent({isolation: "worktree"})` parity with
//! Claude Code (automatic per-agent isolation, file-lock interaction,
//! permission scoping) is a separate pass — see the FUTURE section of
//! the pass-5 plan.
//!
//! ## Usage
//!
//! ```no_run
//! # use std::path::Path;
//! # use anyhow::Result;
//! # fn demo(repo: &Path) -> Result<()> {
//! use brainwires_cli::agent::worktree::WorktreeGuard;
//! let guard = WorktreeGuard::create(repo, "my-agent")?;
//! // Use `guard.path()` as the agent's working_directory...
//! // Drop (at end of scope) runs `git worktree remove`.
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::paths::PlatformPaths;

/// A git worktree owned by the current process. Best-effort cleanup on drop.
pub struct WorktreeGuard {
    /// Absolute path to the worktree root.
    path: PathBuf,
    /// Path to the source repository — used for `git worktree remove`.
    repo: PathBuf,
    /// Whether cleanup has already been performed (so `Drop` is a no-op after
    /// an explicit [`remove`](Self::remove) call).
    removed: bool,
}

impl WorktreeGuard {
    /// Create a new worktree rooted at `~/.brainwires/worktrees/<uuid>/` off
    /// of `repo`. `label` is mixed into the uuid directory name for easier
    /// debugging when worktrees leak.
    ///
    /// Fails if `repo` is not a git repository or if the `git` CLI is not
    /// on the `$PATH`.
    pub fn create(repo: &Path, label: &str) -> Result<Self> {
        let repo = repo
            .canonicalize()
            .with_context(|| format!("repo path not found: {}", repo.display()))?;

        let root = PlatformPaths::dot_brainwires_dir()?.join("worktrees");
        std::fs::create_dir_all(&root)
            .with_context(|| format!("failed to create {}", root.display()))?;

        let slug: String = label
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let id = format!("{}-{}", slug, uuid::Uuid::new_v4());
        let path = root.join(&id);

        // Create detached worktree at HEAD. `--detach` keeps us on a
        // disposable detached HEAD so branch bookkeeping stays clean.
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("worktree")
            .arg("add")
            .arg("--detach")
            .arg(&path)
            .arg("HEAD")
            .status()
            .with_context(|| format!("failed to run `git worktree add` for {}", path.display()))?;
        if !status.success() {
            anyhow::bail!(
                "git worktree add failed (exit {}) for {}",
                status.code().unwrap_or(-1),
                path.display()
            );
        }

        Ok(Self {
            path,
            repo,
            removed: false,
        })
    }

    /// The absolute path to the worktree root.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Remove the worktree eagerly (consumes `self`). Returns `Ok(())` on
    /// success and `Err` on failure; either way, the `Drop` impl will not
    /// re-run the removal.
    pub fn remove(mut self) -> Result<()> {
        self.removed = true;
        Self::remove_inner(&self.path, &self.repo)
    }

    /// Internal helper — runs `git worktree remove --force`. Used by both
    /// explicit [`remove`](Self::remove) and the `Drop` impl.
    fn remove_inner(path: &Path, repo: &Path) -> Result<()> {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(path)
            .status()
            .with_context(|| {
                format!("failed to run `git worktree remove` for {}", path.display())
            })?;
        if !status.success() {
            // Fall back to removing the directory manually — git bookkeeping
            // gets stale but the files are gone. Caller can run
            // `git worktree prune` to clean up the metadata.
            if path.exists() {
                std::fs::remove_dir_all(path)
                    .with_context(|| format!("failed to manually remove {}", path.display()))?;
            }
        }
        Ok(())
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        if let Err(e) = Self::remove_inner(&self.path, &self.repo) {
            // Best-effort — a failed cleanup just leaks a dir; run
            // `git worktree prune` / `rm -rf` manually if it bites.
            tracing::warn!(
                "WorktreeGuard cleanup failed for {}: {}",
                self.path.display(),
                e
            );
        }
    }
}

/// Garbage-collect leaked worktrees under `~/.brainwires/worktrees/` by
/// asking git to prune the bookkeeping. Cheap; safe to call at startup.
pub fn prune_orphans() -> Result<()> {
    let root = PlatformPaths::dot_brainwires_dir()?.join("worktrees");
    if !root.exists() {
        return Ok(());
    }
    // We don't know which repo each orphan belongs to, but `git worktree
    // prune` run from the current cwd catches any whose git metadata refers
    // to this cwd. For broader cleanup the user should run `git worktree
    // prune` inside each affected repo.
    let _ = Command::new("git").arg("worktree").arg("prune").status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_util::{ENV_LOCK, EnvVarGuard};
    use tempfile::TempDir;

    fn init_repo(dir: &Path) {
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "git {:?} failed", args);
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(dir.join("README"), "hi").unwrap();
        run(&["add", "README"]);
        run(&["commit", "-q", "-m", "init"]);
    }

    fn setup_home() -> (TempDir, EnvVarGuard, std::sync::MutexGuard<'static, ()>) {
        // Use BRAINWIRES_HOME — it's a brainwires-specific override that no
        // other test reads. Mutating $HOME directly leaks into unrelated
        // tests (file_explorer, anything reading dirs::home_dir()) even
        // with a guard, because unrelated tests may read $HOME during our
        // window before we restore. ENV_LOCK still serialises with other
        // env-mutating tests.
        let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = TempDir::new().unwrap();
        let env = EnvVarGuard::set("BRAINWIRES_HOME", tmp.path().join(".brainwires"));
        (tmp, env, lock)
    }

    /// git availability — skip the lifecycle test when the CLI is absent
    /// so we don't fail on minimal CI images.
    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn create_and_drop_round_trip() {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return;
        }
        let (_home, _env, _lock) = setup_home();
        let repo_tmp = TempDir::new().unwrap();
        init_repo(repo_tmp.path());

        let path = {
            let g = WorktreeGuard::create(repo_tmp.path(), "test").unwrap();
            let p = g.path().to_path_buf();
            assert!(p.exists(), "worktree path must exist while guard is live");
            assert!(
                p.join("README").exists(),
                "worktree must have HEAD contents"
            );
            p
        };
        // After drop: path should be gone (best-effort; we don't assert
        // strict absence since `git worktree remove` could theoretically
        // leave metadata behind on some platforms).
        assert!(
            !path.exists(),
            "worktree path should be cleaned up on drop, still at {}",
            path.display()
        );
    }

    #[test]
    fn explicit_remove_prevents_double_cleanup() {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return;
        }
        let (_home, _env, _lock) = setup_home();
        let repo_tmp = TempDir::new().unwrap();
        init_repo(repo_tmp.path());

        let guard = WorktreeGuard::create(repo_tmp.path(), "explicit").unwrap();
        let path = guard.path().to_path_buf();
        guard.remove().expect("explicit remove ok");
        assert!(!path.exists());
        // Drop has now run on a consumed `guard`; no panic expected.
    }

    #[test]
    fn non_git_repo_fails_cleanly() {
        if !git_available() {
            return;
        }
        let (_home, _env, _lock) = setup_home();
        let not_a_repo = TempDir::new().unwrap();
        let result = WorktreeGuard::create(not_a_repo.path(), "nope");
        assert!(result.is_err(), "non-git path should fail");
    }
}
