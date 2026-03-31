use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Platform-specific paths for Brainwires CLI (XDG-compliant)
pub struct PlatformPaths;

impl PlatformPaths {
    /// Get the appropriate data directory for the current platform
    ///
    /// - Windows: %LOCALAPPDATA%
    /// - macOS: ~/Library/Application Support
    /// - Linux/Unix: $XDG_DATA_HOME or ~/.local/share
    pub fn data_dir() -> Result<PathBuf> {
        if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .context("Failed to get LOCALAPPDATA")
        } else if cfg!(target_os = "macos") {
            dirs::home_dir()
                .map(|home| home.join("Library/Application Support"))
                .context("Failed to get home directory")
        } else {
            // Linux/Unix - follow XDG Base Directory specification
            std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    dirs::home_dir()
                        .map(|home| home.join(".local/share"))
                        .context("Failed to get home directory")
                })
        }
    }

    /// Get the appropriate cache directory for the current platform
    ///
    /// - Windows: %LOCALAPPDATA%
    /// - macOS: ~/Library/Caches
    /// - Linux/Unix: $XDG_CACHE_HOME or ~/.cache
    pub fn cache_dir() -> Result<PathBuf> {
        if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .context("Failed to get LOCALAPPDATA")
        } else if cfg!(target_os = "macos") {
            dirs::home_dir()
                .map(|home| home.join("Library/Caches"))
                .context("Failed to get home directory")
        } else {
            // Linux/Unix - follow XDG Base Directory specification
            std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    dirs::home_dir()
                        .map(|home| home.join(".cache"))
                        .context("Failed to get home directory")
                })
        }
    }

    /// Get the appropriate config directory for the current platform
    ///
    /// - Windows: %APPDATA%
    /// - macOS: ~/Library/Application Support
    /// - Linux/Unix: $XDG_CONFIG_HOME or ~/.config
    pub fn config_dir() -> Result<PathBuf> {
        if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .context("Failed to get APPDATA")
        } else if cfg!(target_os = "macos") {
            dirs::home_dir()
                .map(|home| home.join("Library/Application Support"))
                .context("Failed to get home directory")
        } else {
            // Linux/Unix - follow XDG Base Directory specification
            std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    dirs::home_dir()
                        .map(|home| home.join(".config"))
                        .context("Failed to get home directory")
                })
        }
    }

    /// Get Brainwires-specific data directory
    ///
    /// Returns: {data_dir}/brainwires
    pub fn brainwires_data_dir() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("brainwires"))
    }

    /// Get Brainwires-specific cache directory
    ///
    /// Returns: {cache_dir}/brainwires
    pub fn brainwires_cache_dir() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("brainwires"))
    }

    /// Get Brainwires-specific config directory
    ///
    /// Returns: {config_dir}/brainwires
    pub fn brainwires_config_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("brainwires"))
    }

    /// Get the home-based .brainwires directory (~/.brainwires/)
    ///
    /// This is used for user-facing config files like permissions.toml
    /// that users might want to edit manually.
    pub fn dot_brainwires_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(".brainwires"))
    }

    /// Ensure the ~/.brainwires directory exists
    pub fn ensure_dot_brainwires_dir() -> Result<PathBuf> {
        let dir = Self::dot_brainwires_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create ~/.brainwires directory")?;
        }
        Ok(dir)
    }

    /// Get the permissions config file path
    ///
    /// Returns: ~/.brainwires/permissions.toml
    pub fn permissions_file() -> Result<PathBuf> {
        Ok(Self::dot_brainwires_dir()?.join("permissions.toml"))
    }

    /// Get the audit log directory
    ///
    /// Returns: {data_dir}/brainwires/audit
    pub fn audit_log_dir() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("audit"))
    }

    /// Get the trust store file path
    ///
    /// Returns: {data_dir}/brainwires/trust.json
    pub fn trust_store_file() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("trust.json"))
    }

    /// Ensure the audit log directory exists
    pub fn ensure_audit_dir() -> Result<PathBuf> {
        let dir = Self::audit_log_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create audit log directory")?;
        }
        Ok(dir)
    }

    /// Get the old home directory (~/.brainwires/) for migration
    #[deprecated(note = "Use XDG-compliant directories instead")]
    pub fn old_home_dir() -> Result<PathBuf> {
        Self::dot_brainwires_dir()
    }

    /// Get the config file path
    ///
    /// Returns: {config_dir}/brainwires/config.json
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::brainwires_config_dir()?.join("config.json"))
    }

    /// Get the MCP config file path
    ///
    /// Returns: {config_dir}/brainwires/mcp-config.json
    pub fn mcp_config_file() -> Result<PathBuf> {
        Ok(Self::brainwires_config_dir()?.join("mcp-config.json"))
    }

    /// Get the usage tracking file path
    ///
    /// Returns: {cache_dir}/brainwires/usage.json
    pub fn usage_file() -> Result<PathBuf> {
        Ok(Self::brainwires_cache_dir()?.join("usage.json"))
    }

    /// Get the session file path
    ///
    /// Returns: {data_dir}/brainwires/session.json
    pub fn session_file() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("session.json"))
    }

    /// Get the checkpoint directory path
    ///
    /// Returns: {data_dir}/brainwires/checkpoints
    pub fn checkpoints_dir() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("checkpoints"))
    }

    /// Get the execution history file path
    ///
    /// Returns: {cache_dir}/brainwires/history.json
    pub fn history_file() -> Result<PathBuf> {
        Ok(Self::brainwires_cache_dir()?.join("history.json"))
    }

    /// Get the conversation database path (for LanceDB)
    ///
    /// Returns: {data_dir}/brainwires/conversations.lance
    pub fn conversations_db_path() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("conversations.lance"))
    }

    /// Get the knowledge database path (SQLite for BKS)
    ///
    /// Returns: {data_dir}/brainwires/knowledge.db
    pub fn knowledge_db() -> Result<PathBuf> {
        Self::ensure_data_dir()?;
        Ok(Self::brainwires_data_dir()?.join("knowledge.db"))
    }

    /// Get the personal knowledge database path (SQLite for PKS)
    ///
    /// Returns: {data_dir}/brainwires/personal_knowledge.db
    pub fn personal_knowledge_db() -> Result<PathBuf> {
        Self::ensure_data_dir()?;
        Ok(Self::brainwires_data_dir()?.join("personal_knowledge.db"))
    }

    /// Get the plans directory path
    ///
    /// Returns: {data_dir}/brainwires/plans
    pub fn plans_dir() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("plans"))
    }

    /// Get the sessions directory path (for backgrounded session sockets)
    ///
    /// Returns: {data_dir}/brainwires/sessions
    pub fn sessions_dir() -> Result<PathBuf> {
        Ok(Self::brainwires_data_dir()?.join("sessions"))
    }

    /// Ensure the sessions directory exists
    pub fn ensure_sessions_dir() -> Result<PathBuf> {
        let dir = Self::sessions_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create sessions directory")?;
        }
        Ok(dir)
    }

    /// Get the socket path for a specific session
    ///
    /// Returns: {data_dir}/brainwires/sessions/{session_id}.sock
    pub fn session_socket(session_id: &str) -> Result<PathBuf> {
        Ok(Self::sessions_dir()?.join(format!("{}.sock", session_id)))
    }

    /// Get the path for a specific plan markdown file
    ///
    /// Returns: {data_dir}/brainwires/plans/{plan_id}.md
    pub fn plan_file(plan_id: &str) -> Result<PathBuf> {
        Ok(Self::plans_dir()?.join(format!("{}.md", plan_id)))
    }

    /// Ensure the plans directory exists
    pub fn ensure_plans_dir() -> Result<PathBuf> {
        let dir = Self::plans_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create plans directory")?;
        }
        Ok(dir)
    }

    /// Ensure the Brainwires data directory exists with secure permissions
    pub fn ensure_data_dir() -> Result<PathBuf> {
        let dir = Self::brainwires_data_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create Brainwires data directory")?;
        }

        // Set secure permissions on Unix (owner read/write/execute only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .context("Failed to set data directory permissions")?;
        }

        Ok(dir)
    }

    /// Ensure the Brainwires config directory exists with secure permissions
    pub fn ensure_config_dir() -> Result<PathBuf> {
        let dir = Self::brainwires_config_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .context("Failed to create Brainwires config directory")?;
        }

        // Set secure permissions on Unix (owner read/write/execute only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .context("Failed to set config directory permissions")?;
        }

        Ok(dir)
    }

    /// Ensure the Brainwires cache directory exists with secure permissions
    pub fn ensure_cache_dir() -> Result<PathBuf> {
        let dir = Self::brainwires_cache_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create Brainwires cache directory")?;
        }

        // Set secure permissions on Unix (owner read/write/execute only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .context("Failed to set cache directory permissions")?;
        }

        Ok(dir)
    }

    /// Ensure a directory exists with secure permissions (0700 on Unix)
    ///
    /// Creates the directory and all parent directories if needed,
    /// then sets permissions to owner-only access.
    pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir_all(path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        }

        // Set secure permissions on Unix (owner read/write/execute only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).with_context(
                || format!("Failed to set directory permissions: {}", path.display()),
            )?;
        }

        Ok(path.to_path_buf())
    }

    /// Ensure a directory exists (without permission changes, for shared dirs)
    ///
    /// Use this for directories that need default permissions (e.g., shared dirs).
    pub fn ensure_dir_default_perms<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir_all(path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        }
        Ok(path.to_path_buf())
    }

    /// Migrate data from old ~/.brainwires/ to new XDG locations
    ///
    /// This moves:
    /// - config.json, mcp-config.json -> config_dir
    /// - session.json, checkpoints/ -> data_dir
    /// - usage.json, history.json -> cache_dir
    ///
    /// After migration, the old directory is renamed to ~/.brainwires.old
    pub fn migrate_from_old_paths() -> Result<()> {
        #[allow(deprecated)]
        let old_home = Self::old_home_dir()?;

        if !old_home.exists() {
            // Nothing to migrate
            return Ok(());
        }

        tracing::info!(
            "Migrating data from {} to XDG directories",
            old_home.display()
        );

        // Ensure new directories exist
        Self::ensure_data_dir()?;
        Self::ensure_config_dir()?;
        Self::ensure_cache_dir()?;

        // Track if any files were migrated
        let mut migrated = false;

        // Migrate config files
        migrated |= Self::migrate_file(&old_home.join("config.json"), &Self::config_file()?)?;
        migrated |=
            Self::migrate_file(&old_home.join("mcp-config.json"), &Self::mcp_config_file()?)?;

        // Migrate data files
        migrated |= Self::migrate_file(&old_home.join("session.json"), &Self::session_file()?)?;
        migrated |= Self::migrate_dir(&old_home.join("checkpoints"), &Self::checkpoints_dir()?)?;

        // Migrate cache files
        migrated |= Self::migrate_file(&old_home.join("usage.json"), &Self::usage_file()?)?;
        migrated |= Self::migrate_file(&old_home.join("history.json"), &Self::history_file()?)?;

        if migrated {
            tracing::info!("Migration complete");
        }

        // Rename old directory to prevent repeated migration attempts
        let old_backup = old_home
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?
            .join(".brainwires.old");

        std::fs::rename(&old_home, &old_backup).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                old_home.display(),
                old_backup.display()
            )
        })?;

        tracing::info!("Old directory renamed to: {}", old_backup.display());

        Ok(())
    }

    /// Helper to migrate a file
    /// Returns true if file was migrated, false if skipped
    fn migrate_file(from: &Path, to: &Path) -> Result<bool> {
        if from.exists() && !to.exists() {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(from, to).with_context(|| {
                format!("Failed to migrate {} to {}", from.display(), to.display())
            })?;
            tracing::info!("Migrated: {} -> {}", from.display(), to.display());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Helper to migrate a directory
    /// Returns true if directory was migrated, false if skipped
    fn migrate_dir(from: &Path, to: &Path) -> Result<bool> {
        if from.exists() && from.is_dir() && !to.exists() {
            std::fs::create_dir_all(to)?;
            copy_dir_recursive(from, to)?;
            tracing::info!("Migrated directory: {} -> {}", from.display(), to.display());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // INFALLIBLE PATH METHODS (for serde defaults and RAG module)
    //
    // These methods return PathBuf directly, falling back to "." on error.
    // Use these when you need a path for serde default values or when
    // failure is acceptable (e.g., config file paths).
    // =========================================================================

    /// Get project data directory (infallible, falls back to ".")
    pub fn project_data_dir() -> PathBuf {
        Self::brainwires_data_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Get project cache directory (infallible, falls back to ".")
    pub fn project_cache_dir() -> PathBuf {
        Self::brainwires_cache_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Get project config directory (infallible, falls back to ".")
    pub fn project_config_dir() -> PathBuf {
        Self::brainwires_config_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Get default LanceDB path for RAG indexing (infallible)
    ///
    /// Returns: {data_dir}/brainwires/lancedb
    pub fn default_lancedb_path() -> PathBuf {
        Self::project_data_dir().join("lancedb")
    }

    /// Get default hash cache path (infallible)
    ///
    /// Returns: {cache_dir}/brainwires/hash_cache.json
    pub fn default_hash_cache_path() -> PathBuf {
        Self::project_cache_dir().join("hash_cache.json")
    }

    /// Get default git cache path (infallible)
    ///
    /// Returns: {cache_dir}/brainwires/git_cache.json
    pub fn default_git_cache_path() -> PathBuf {
        Self::project_cache_dir().join("git_cache.json")
    }

    /// Get default config path (infallible)
    ///
    /// Returns: {config_dir}/brainwires/config.toml
    pub fn default_config_path() -> PathBuf {
        Self::project_config_dir().join("config.toml")
    }

    // =========================================================================
    // PROJECT-SPECIFIC PATHS (for RAG indexing per-project)
    //
    // These paths are stored in .brainwires/ within the project root (CWD).
    // They contain project-specific data like RAG indexes and caches.
    // =========================================================================

    /// Get the project-specific brainwires directory (.brainwires/ in CWD)
    ///
    /// Returns: {cwd}/.brainwires
    pub fn project_brainwires_dir() -> Result<PathBuf> {
        Ok(std::env::current_dir()
            .context("Failed to get current working directory")?
            .join(".brainwires"))
    }

    /// Get the project-specific brainwires directory (infallible)
    ///
    /// Returns: {cwd}/.brainwires or ".brainwires" on error
    pub fn project_brainwires_dir_infallible() -> PathBuf {
        Self::project_brainwires_dir().unwrap_or_else(|_| PathBuf::from(".brainwires"))
    }

    /// Ensure the project-specific brainwires directory exists
    pub fn ensure_project_brainwires_dir() -> Result<PathBuf> {
        let dir = Self::project_brainwires_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .context("Failed to create project .brainwires directory")?;
        }
        Ok(dir)
    }

    /// Get project-specific LanceDB path for RAG indexing (infallible)
    ///
    /// Returns: {cwd}/.brainwires/lancedb
    pub fn project_lancedb_path() -> PathBuf {
        Self::project_brainwires_dir_infallible().join("lancedb")
    }

    /// Get project-specific hash cache path (infallible)
    ///
    /// Returns: {cwd}/.brainwires/hash_cache.json
    pub fn project_hash_cache_path() -> PathBuf {
        Self::project_brainwires_dir_infallible().join("hash_cache.json")
    }

    /// Get project-specific git cache path (infallible)
    ///
    /// Returns: {cwd}/.brainwires/git_cache.json
    pub fn project_git_cache_path() -> PathBuf {
        Self::project_brainwires_dir_infallible().join("git_cache.json")
    }

    // =========================================================================
    // GLOBAL FASTEMBED CACHE
    //
    // Embedding models are shared globally since they're large (~25MB each).
    // =========================================================================

    /// Get global fastembed cache directory
    ///
    /// Returns: {data_dir}/brainwires/fastembed
    pub fn fastembed_cache_dir() -> PathBuf {
        Self::project_data_dir().join("fastembed")
    }

    /// Ensure the fastembed cache directory exists
    pub fn ensure_fastembed_cache_dir() -> Result<PathBuf> {
        let dir = Self::brainwires_data_dir()?.join("fastembed");
        if !dir.exists() {
            std::fs::create_dir_all(&dir).context("Failed to create fastembed cache directory")?;
        }
        Ok(dir)
    }

    /// Migrate fastembed cache from project root to global location
    ///
    /// Moves .fastembed_cache/ from CWD to ~/.local/share/brainwires/fastembed/
    pub fn migrate_fastembed_cache() -> Result<bool> {
        let old_cache = std::env::current_dir()?.join(".fastembed_cache");
        let new_cache = Self::ensure_fastembed_cache_dir()?;

        if old_cache.exists() && old_cache.is_dir() {
            // Only migrate if new cache doesn't have the models yet
            if !new_cache
                .join("models--Qdrant--all-MiniLM-L6-v2-onnx")
                .exists()
            {
                tracing::info!(
                    "Migrating fastembed cache from {} to {}",
                    old_cache.display(),
                    new_cache.display()
                );
                copy_dir_recursive(&old_cache, &new_cache)?;
                // Remove old cache after successful migration
                std::fs::remove_dir_all(&old_cache)
                    .context("Failed to remove old fastembed cache")?;
                tracing::info!("Fastembed cache migration complete");
                return Ok(true);
            }
        }
        Ok(false)
    }

    // =========================================================================
    // SKILLS DIRECTORIES
    //
    // Agent Skills are stored in two locations:
    // - Personal: ~/.brainwires/skills/ (user-specific)
    // - Project: .brainwires/skills/ (project-specific, takes precedence)
    // =========================================================================

    /// Get the personal skills directory
    ///
    /// Returns: ~/.brainwires/skills/
    pub fn personal_skills_dir() -> Result<PathBuf> {
        Ok(Self::dot_brainwires_dir()?.join("skills"))
    }

    /// Ensure the personal skills directory exists
    pub fn ensure_personal_skills_dir() -> Result<PathBuf> {
        let dir = Self::personal_skills_dir()?;
        Self::ensure_dir_default_perms(&dir)
    }

    /// Get the project-specific skills directory
    ///
    /// Returns: {cwd}/.brainwires/skills/
    pub fn project_skills_dir() -> Result<PathBuf> {
        Ok(Self::project_brainwires_dir()?.join("skills"))
    }

    /// Ensure the project skills directory exists
    pub fn ensure_project_skills_dir() -> Result<PathBuf> {
        let dir = Self::project_skills_dir()?;
        Self::ensure_dir_default_perms(&dir)
    }

    /// Get personal skills directory (infallible)
    ///
    /// Returns: ~/.brainwires/skills/ or ".brainwires/skills" on error
    pub fn personal_skills_dir_infallible() -> PathBuf {
        Self::personal_skills_dir().unwrap_or_else(|_| PathBuf::from(".brainwires/skills"))
    }

    /// Get project skills directory (infallible)
    ///
    /// Returns: {cwd}/.brainwires/skills/ or ".brainwires/skills" on error
    pub fn project_skills_dir_infallible() -> PathBuf {
        Self::project_skills_dir().unwrap_or_else(|_| PathBuf::from(".brainwires/skills"))
    }

    /// Migrate data from old project-rag directories to new brainwires directories
    ///
    /// OLD locations:
    /// - ~/.local/share/project-rag/
    /// - ~/.cache/project-rag/
    /// - ~/.config/project-rag/
    ///
    /// NEW locations:
    /// - ~/.local/share/brainwires/
    /// - ~/.cache/brainwires/
    /// - ~/.config/brainwires/
    pub fn migrate_from_project_rag() -> std::result::Result<(), std::io::Error> {
        let data_base = Self::data_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cache_base = Self::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config_base = Self::config_dir().unwrap_or_else(|_| PathBuf::from("."));

        let old_data_dir = data_base.join("project-rag");
        let old_cache_dir = cache_base.join("project-rag");
        let old_config_dir = config_base.join("project-rag");

        let new_data_dir = Self::project_data_dir();
        let new_cache_dir = Self::project_cache_dir();
        let new_config_dir = Self::project_config_dir();

        let mut migrated = false;

        // Migrate data directory
        if old_data_dir.exists() && !new_data_dir.exists() {
            if let Some(parent) = new_data_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(&old_data_dir, &new_data_dir)?;
            eprintln!(
                "Migrated: {} -> {}",
                old_data_dir.display(),
                new_data_dir.display()
            );
            migrated = true;
        }

        // Migrate cache directory
        if old_cache_dir.exists() && !new_cache_dir.exists() {
            if let Some(parent) = new_cache_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(&old_cache_dir, &new_cache_dir)?;
            eprintln!(
                "Migrated: {} -> {}",
                old_cache_dir.display(),
                new_cache_dir.display()
            );
            migrated = true;
        }

        // Migrate config directory
        if old_config_dir.exists() && !new_config_dir.exists() {
            if let Some(parent) = new_config_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(&old_config_dir, &new_config_dir)?;
            eprintln!(
                "Migrated: {} -> {}",
                old_config_dir.display(),
                new_config_dir.display()
            );
            migrated = true;
        }

        if migrated {
            eprintln!("Migration from project-rag to brainwires complete");
        }

        Ok(())
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from_path = entry.path();
        let to_path = to.join(entry.file_name());

        if file_type.is_dir() {
            std::fs::create_dir_all(&to_path)?;
            copy_dir_recursive(&from_path, &to_path)?;
        } else {
            std::fs::copy(&from_path, &to_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_not_empty() {
        let dir = PlatformPaths::data_dir().unwrap();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_cache_dir_not_empty() {
        let dir = PlatformPaths::cache_dir().unwrap();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_config_dir_not_empty() {
        let dir = PlatformPaths::config_dir().unwrap();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_brainwires_dirs_contain_brainwires() {
        let data = PlatformPaths::brainwires_data_dir().unwrap();
        let cache = PlatformPaths::brainwires_cache_dir().unwrap();
        let config = PlatformPaths::brainwires_config_dir().unwrap();

        assert!(data.to_string_lossy().contains("brainwires"));
        assert!(cache.to_string_lossy().contains("brainwires"));
        assert!(config.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_config_file() {
        let config = PlatformPaths::config_file().unwrap();
        assert!(config.to_string_lossy().contains("config.json"));
        assert!(config.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_mcp_config_file() {
        let mcp_config = PlatformPaths::mcp_config_file().unwrap();
        assert!(mcp_config.to_string_lossy().contains("mcp-config.json"));
        assert!(mcp_config.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_usage_file() {
        let usage = PlatformPaths::usage_file().unwrap();
        assert!(usage.to_string_lossy().contains("usage.json"));
        assert!(usage.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_session_file() {
        let session = PlatformPaths::session_file().unwrap();
        assert!(session.to_string_lossy().contains("session.json"));
        assert!(session.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_checkpoints_dir() {
        let checkpoints = PlatformPaths::checkpoints_dir().unwrap();
        assert!(checkpoints.to_string_lossy().contains("checkpoints"));
        assert!(checkpoints.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_history_file() {
        let history = PlatformPaths::history_file().unwrap();
        assert!(history.to_string_lossy().contains("history.json"));
        assert!(history.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_conversations_db_path() {
        let conversations = PlatformPaths::conversations_db_path().unwrap();
        assert!(
            conversations
                .to_string_lossy()
                .contains("conversations.lance")
        );
        assert!(conversations.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_plans_dir() {
        let plans = PlatformPaths::plans_dir().unwrap();
        assert!(plans.to_string_lossy().contains("plans"));
        assert!(plans.to_string_lossy().contains("brainwires"));
    }

    #[test]
    fn test_plan_file() {
        let plan = PlatformPaths::plan_file("test-plan-123").unwrap();
        assert!(plan.to_string_lossy().contains("test-plan-123.md"));
        assert!(plan.to_string_lossy().contains("plans"));
    }

    #[test]
    fn test_ensure_dirs() {
        // These may succeed or fail depending on permissions
        let _ = PlatformPaths::ensure_data_dir();
        let _ = PlatformPaths::ensure_config_dir();
        let _ = PlatformPaths::ensure_cache_dir();
    }

    #[test]
    fn test_ensure_dir_with_tempdir() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let test_path = temp.path().join("test_subdir");

        let result = PlatformPaths::ensure_dir(&test_path).unwrap();
        assert!(result.exists());
        assert!(result.is_dir());
    }

    #[test]
    fn test_ensure_dir_existing() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();

        // Call ensure_dir on existing directory
        let result = PlatformPaths::ensure_dir(temp.path()).unwrap();
        assert_eq!(result, temp.path());
    }

    #[test]
    #[allow(deprecated)]
    fn test_old_home_dir() {
        let old_home = PlatformPaths::old_home_dir().unwrap();
        assert!(old_home.to_string_lossy().contains(".brainwires"));
    }

    #[test]
    fn test_migrate_from_old_paths() {
        // Just test it doesn't panic - actual migration tested manually
        let _ = PlatformPaths::migrate_from_old_paths();
    }

    #[test]
    fn test_project_brainwires_dir() {
        let project_dir = PlatformPaths::project_brainwires_dir().unwrap();
        assert!(project_dir.to_string_lossy().ends_with(".brainwires"));
    }

    #[test]
    fn test_project_brainwires_dir_infallible() {
        let project_dir = PlatformPaths::project_brainwires_dir_infallible();
        assert!(project_dir.to_string_lossy().ends_with(".brainwires"));
    }

    #[test]
    fn test_project_lancedb_path() {
        let lancedb_path = PlatformPaths::project_lancedb_path();
        assert!(lancedb_path.to_string_lossy().contains(".brainwires"));
        assert!(lancedb_path.to_string_lossy().ends_with("lancedb"));
    }

    #[test]
    fn test_project_hash_cache_path() {
        let hash_cache = PlatformPaths::project_hash_cache_path();
        assert!(hash_cache.to_string_lossy().contains(".brainwires"));
        assert!(hash_cache.to_string_lossy().ends_with("hash_cache.json"));
    }

    #[test]
    fn test_project_git_cache_path() {
        let git_cache = PlatformPaths::project_git_cache_path();
        assert!(git_cache.to_string_lossy().contains(".brainwires"));
        assert!(git_cache.to_string_lossy().ends_with("git_cache.json"));
    }

    #[test]
    fn test_fastembed_cache_dir() {
        let fastembed_cache = PlatformPaths::fastembed_cache_dir();
        assert!(fastembed_cache.to_string_lossy().contains("brainwires"));
        assert!(fastembed_cache.to_string_lossy().ends_with("fastembed"));
    }

    #[test]
    fn test_personal_skills_dir() {
        let skills_dir = PlatformPaths::personal_skills_dir().unwrap();
        assert!(skills_dir.to_string_lossy().contains(".brainwires"));
        assert!(skills_dir.to_string_lossy().ends_with("skills"));
    }

    #[test]
    fn test_project_skills_dir() {
        let skills_dir = PlatformPaths::project_skills_dir().unwrap();
        assert!(skills_dir.to_string_lossy().contains(".brainwires"));
        assert!(skills_dir.to_string_lossy().ends_with("skills"));
    }

    #[test]
    fn test_personal_skills_dir_infallible() {
        let skills_dir = PlatformPaths::personal_skills_dir_infallible();
        assert!(skills_dir.to_string_lossy().ends_with("skills"));
    }

    #[test]
    fn test_project_skills_dir_infallible() {
        let skills_dir = PlatformPaths::project_skills_dir_infallible();
        assert!(skills_dir.to_string_lossy().ends_with("skills"));
    }
}
