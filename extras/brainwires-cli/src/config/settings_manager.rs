//! Discovery + merging of layered `settings.json` files.
//!
//! See [`crate::config::settings`] for the merge order and file locations.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::settings::Settings;
use crate::utils::paths::PlatformPaths;

/// Result of a settings load — merged `Settings` plus the list of files that
/// contributed (for diagnostics / `brainwires config settings --explain`).
#[derive(Debug, Clone, Default)]
pub struct SettingsManager {
    pub merged: Settings,
    pub sources: Vec<PathBuf>,
}

impl SettingsManager {
    /// Discover and merge all `settings.json` files for the given working
    /// directory. Missing files are silently skipped; malformed JSON in any
    /// single file is reported via `tracing::warn!` but does not abort the
    /// load — one bad file should not disable every other rule.
    pub fn load(cwd: &Path) -> Self {
        Self::load_from_paths(Self::candidate_paths(cwd))
    }

    /// Load from an explicit list of candidate paths (lowest precedence
    /// first). Useful in tests to avoid touching the user's real home
    /// directory.
    pub fn load_from_paths(paths: Vec<PathBuf>) -> Self {
        let mut merged = Settings::default();
        let mut sources = Vec::new();

        for path in paths {
            match read_settings(&path) {
                Ok(Some(s)) => {
                    sources.push(path);
                    merged.merge(s);
                }
                Ok(None) => {} // file missing — fine
                Err(e) => {
                    tracing::warn!(
                        "Ignoring malformed settings file {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Self { merged, sources }
    }

    /// The ordered list of filesystem locations we look at. Earlier entries
    /// are lower-precedence (later merges overwrite scalars / extend arrays).
    pub fn candidate_paths(cwd: &Path) -> Vec<PathBuf> {
        let mut v = Vec::new();

        // 1. User-wide brainwires settings.
        if let Ok(home) = PlatformPaths::dot_brainwires_dir() {
            v.push(home.join("settings.json"));
        }

        // 2. Migrator-compat: ~/.claude/settings.json.
        if let Some(home) = dirs::home_dir() {
            v.push(home.join(".claude").join("settings.json"));
        }

        // 3+4. Project root — shared settings, then local override.
        let project_root = PlatformPaths::find_project_root(cwd);
        v.push(project_root.join(".brainwires").join("settings.json"));
        v.push(project_root.join(".brainwires").join("settings.local.json"));

        v
    }
}

fn read_settings(path: &Path) -> Result<Option<Settings>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let s: Settings = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::{HookCommand, HookMatcher, Hooks, Permissions};
    use tempfile::TempDir;

    fn write_json(path: &Path, s: &Settings) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string(s).unwrap()).unwrap();
    }

    /// Isolate tests from the user's real home directory by passing only the
    /// project-scoped candidate paths explicitly.
    fn project_only_paths(project: &Path) -> Vec<PathBuf> {
        vec![
            project.join(".brainwires").join("settings.json"),
            project.join(".brainwires").join("settings.local.json"),
        ]
    }

    #[test]
    fn missing_settings_yields_default() {
        let tmp = TempDir::new().unwrap();
        let mgr = SettingsManager::load_from_paths(project_only_paths(tmp.path()));
        assert_eq!(mgr.merged, Settings::default());
    }

    #[test]
    fn project_local_overrides_project_shared() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path();
        write_json(
            &project.join(".brainwires").join("settings.json"),
            &Settings {
                permissions: Some(Permissions {
                    allow: vec!["Read".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        write_json(
            &project.join(".brainwires").join("settings.local.json"),
            &Settings {
                permissions: Some(Permissions {
                    allow: vec!["Edit".into()],
                    deny: vec!["Bash(rm:*)".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let mgr = SettingsManager::load_from_paths(project_only_paths(project));
        let p = mgr.merged.permissions.unwrap();
        // Arrays concatenate: both shared + local entries present.
        assert!(p.allow.contains(&"Read".to_string()));
        assert!(p.allow.contains(&"Edit".to_string()));
        assert!(p.deny.contains(&"Bash(rm:*)".to_string()));
        assert_eq!(mgr.sources.len(), 2);
    }

    #[test]
    fn hooks_merge_across_files() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path();
        write_json(
            &project.join(".brainwires").join("settings.json"),
            &Settings {
                hooks: Some(Hooks {
                    pre_tool_use: vec![HookMatcher {
                        matcher: Some("Bash".into()),
                        hooks: vec![HookCommand {
                            kind: "command".into(),
                            command: "echo a".into(),
                            timeout_ms: None,
                        }],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        write_json(
            &project.join(".brainwires").join("settings.local.json"),
            &Settings {
                hooks: Some(Hooks {
                    pre_tool_use: vec![HookMatcher {
                        matcher: None,
                        hooks: vec![HookCommand {
                            kind: "command".into(),
                            command: "echo b".into(),
                            timeout_ms: None,
                        }],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let mgr = SettingsManager::load_from_paths(project_only_paths(project));
        let h = mgr.merged.hooks.unwrap();
        assert_eq!(h.pre_tool_use.len(), 2);
    }

    #[test]
    fn docs_example_parses() {
        // Guard against drift between the documented example and the schema.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/harness/settings.example.json");
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed to read {}: {}", path.display(), e)
        });
        let parsed: Settings = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!("failed to parse {}: {}", path.display(), e)
        });
        let perms = parsed.permissions.expect("example should define permissions");
        assert!(!perms.allow.is_empty());
        assert!(!perms.deny.is_empty());
        let hooks = parsed.hooks.expect("example should define hooks");
        assert!(!hooks.pre_tool_use.is_empty());
    }

    #[test]
    fn malformed_json_is_skipped_not_fatal() {
        let tmp = TempDir::new().unwrap();
        let bad = tmp.path().join(".brainwires").join("settings.json");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(&bad, "not valid json {[").unwrap();

        // Also a valid local file — it should still apply.
        write_json(
            &tmp.path().join(".brainwires").join("settings.local.json"),
            &Settings {
                permissions: Some(Permissions {
                    allow: vec!["Read".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let mgr = SettingsManager::load_from_paths(project_only_paths(tmp.path()));
        let p = mgr.merged.permissions.unwrap();
        assert_eq!(p.allow, vec!["Read".to_string()]);
    }
}
