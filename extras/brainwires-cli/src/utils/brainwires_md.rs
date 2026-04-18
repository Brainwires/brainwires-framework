//! BRAINWIRES.md / CLAUDE.md Parser
//!
//! Parses project-specific instruction files with @file.md import support.
//!
//! # Auto-discovery
//!
//! [`discover_project_instructions`] walks from the working directory toward
//! the filesystem root collecting `BRAINWIRES.md` and `CLAUDE.md` files,
//! then adds global instructions from `~/.claude/` and `~/.brainwires/`.
//! This matches Claude Code's behaviour so users migrating from Claude Code
//! get their existing `CLAUDE.md` picked up automatically.
//!
//! Precedence (highest wins when rules conflict — we just concatenate, the
//! model applies later rules on top of earlier ones):
//!
//! 1. Global user rules (`~/.claude/CLAUDE.md`, `~/.brainwires/CLAUDE.md`, `~/.brainwires/BRAINWIRES.md`).
//! 2. Ancestor directories, outermost first, working toward cwd.
//! 3. The cwd files themselves (applied last so they override ancestors).

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum depth for recursive imports to prevent infinite loops
const MAX_IMPORT_DEPTH: usize = 10;

/// Maximum number of directories we walk upward looking for instruction files.
/// Kept low to avoid pathological hangs on deep filesystems / network mounts.
const MAX_WALK_UP_DEPTH: usize = 32;

/// Instruction file names we recognize, in lookup priority order.
///
/// When both `BRAINWIRES.md` and `CLAUDE.md` live in the same directory, both
/// are loaded (BRAINWIRES.md first as the native name) — we don't pick one.
const INSTRUCTION_FILENAMES: &[&str] = &["BRAINWIRES.md", "CLAUDE.md"];

/// A single loaded instruction source.
#[derive(Debug, Clone)]
pub struct InstructionSource {
    pub path: PathBuf,
    pub contents: String,
}

/// Parse a BRAINWIRES.md file and resolve all @file.md imports
pub fn load_brainwires_instructions(base_path: &Path) -> Result<String> {
    let brainwires_path = base_path.join("BRAINWIRES.md");

    if !brainwires_path.exists() {
        return Ok(String::new());
    }

    let mut visited = HashSet::new();
    parse_file_with_imports(&brainwires_path, &mut visited, 0)
}

/// Discover and load all project and global instruction files.
///
/// Walks from `cwd` toward the filesystem root, collecting `BRAINWIRES.md`
/// and `CLAUDE.md` files along the way. Also reads any files present in
/// `~/.claude/` and `~/.brainwires/` as global user-level instructions.
///
/// Returns a vector in application order: global user instructions first,
/// then ancestor-directory instructions from root-ward to cwd. Empty if
/// nothing was found. Errors from individual files are logged but do not
/// abort the walk — one malformed file should not kill the session.
pub fn discover_project_instructions(cwd: &Path) -> Vec<InstructionSource> {
    let mut sources = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    // 1. Global user instructions.
    for dir in global_instruction_dirs() {
        collect_from_dir(&dir, &mut sources, &mut seen);
    }

    // 2. Ancestor directories (root-ward), then cwd last.
    let ancestors: Vec<&Path> = cwd.ancestors().take(MAX_WALK_UP_DEPTH).collect();
    for dir in ancestors.into_iter().rev() {
        collect_from_dir(dir, &mut sources, &mut seen);
    }

    sources
}

/// Render a list of discovered instruction sources into a single block
/// suitable for injection into a system prompt.
///
/// Each source is prefixed with a `## From {path}` header so the model can
/// reason about where a given rule came from. Returns an empty string if
/// the input is empty.
pub fn render_instructions(sources: &[InstructionSource]) -> String {
    if sources.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("## Project and User Instructions\n\n");
    out.push_str(
        "The following instructions come from CLAUDE.md / BRAINWIRES.md files in your working directory tree and user home. Follow them unless they conflict with the user's current message.\n\n",
    );
    for src in sources {
        out.push_str(&format!("### From {}\n\n", src.path.display()));
        out.push_str(src.contents.trim());
        out.push_str("\n\n");
    }
    out
}

/// Return the list of global instruction directories in priority order.
fn global_instruction_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs_home() {
        // Claude Code's home for migrators — read-only compatibility.
        dirs.push(home.join(".claude"));
        // Brainwires native home.
        dirs.push(home.join(".brainwires"));
    }
    dirs
}

fn dirs_home() -> Option<PathBuf> {
    // Prefer $HOME (respected on Unix and macOS). Fall back to USERPROFILE on
    // Windows. We deliberately avoid the `dirs` crate here to keep this
    // module dependency-free.
    if let Ok(h) = std::env::var("HOME")
        && !h.is_empty()
    {
        return Some(PathBuf::from(h));
    }
    if let Ok(h) = std::env::var("USERPROFILE")
        && !h.is_empty()
    {
        return Some(PathBuf::from(h));
    }
    None
}

/// Look for each recognized instruction filename in `dir` and append any
/// successfully parsed ones to `sources`. Silently skips missing files and
/// logs (but does not propagate) parse errors.
fn collect_from_dir(dir: &Path, sources: &mut Vec<InstructionSource>, seen: &mut HashSet<PathBuf>) {
    for name in INSTRUCTION_FILENAMES {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }
        let canonical = match path.canonicalize() {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !seen.insert(canonical.clone()) {
            continue;
        }

        let mut visited = HashSet::new();
        match parse_file_with_imports(&path, &mut visited, 0) {
            Ok(contents) if !contents.trim().is_empty() => {
                sources.push(InstructionSource {
                    path: canonical,
                    contents,
                });
            }
            Ok(_) => {
                // Empty file — skip.
            }
            Err(e) => {
                tracing::warn!("failed to parse instruction file {}: {}", path.display(), e);
            }
        }
    }
}

/// Recursively parse a markdown file and resolve @file.md imports
fn parse_file_with_imports(
    file_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<String> {
    // Check recursion depth
    if depth > MAX_IMPORT_DEPTH {
        anyhow::bail!(
            "Maximum import depth exceeded ({}). Possible circular dependency.",
            MAX_IMPORT_DEPTH
        );
    }

    // Check for circular imports
    let canonical_path = file_path
        .canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", file_path.display()))?;

    if visited.contains(&canonical_path) {
        anyhow::bail!("Circular import detected: {}", file_path.display());
    }

    visited.insert(canonical_path.clone());

    // Read file content
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Process imports
    let mut result = String::new();
    let base_dir = file_path.parent().with_context(|| {
        format!(
            "Failed to get parent directory for: {}",
            file_path.display()
        )
    })?;

    for line in content.lines() {
        if let Some(import_path) = parse_import_line(line) {
            // Resolve relative path
            let imported_file = base_dir.join(import_path);

            if !imported_file.exists() {
                anyhow::bail!(
                    "Import file not found: {} (referenced in {})",
                    imported_file.display(),
                    file_path.display()
                );
            }

            // Recursively parse imported file
            let imported_content = parse_file_with_imports(&imported_file, visited, depth + 1)?;
            result.push_str(&imported_content);
            result.push('\n');
        } else {
            // Regular line
            result.push_str(line);
            result.push('\n');
        }
    }

    // Remove from visited set to allow the same file to be imported from different paths
    visited.remove(&canonical_path);

    Ok(result)
}

/// Parse an import line and extract the file path
/// Supports formats:
/// - @file.md
/// - @path/to/file.md
/// - @../relative/path.md
fn parse_import_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();

    if trimmed.starts_with('@') && !trimmed.starts_with("@@") {
        // Extract path after @
        let path = &trimmed[1..];

        // Skip if it looks like a mention or other @ syntax
        if path.contains(' ') || path.is_empty() {
            return None;
        }

        Some(path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    fn write(p: &Path, contents: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }

    #[test]
    fn discover_finds_cwd_brainwires_md() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("BRAINWIRES.md"), "project rule here\n");

        let sources = discover_project_instructions(tmp.path());
        assert!(!sources.is_empty(), "expected to find BRAINWIRES.md");
        assert!(
            sources
                .iter()
                .any(|s| s.contents.contains("project rule here")),
            "expected contents to be loaded"
        );
    }

    #[test]
    fn discover_walks_upward_to_find_parent_claude_md() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        write(&tmp.path().join("CLAUDE.md"), "ancestor rule\n");

        let sources = discover_project_instructions(&nested);
        assert!(
            sources.iter().any(|s| s.contents.contains("ancestor rule")),
            "ancestor CLAUDE.md should be discovered, got {:?}",
            sources
                .iter()
                .map(|s| s.path.display().to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn discover_orders_ancestors_before_cwd() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        write(&tmp.path().join("CLAUDE.md"), "TOP_LEVEL\n");
        write(&nested.join("CLAUDE.md"), "INNER\n");

        let sources = discover_project_instructions(&nested);
        // Both discovered.
        let texts: Vec<&str> = sources.iter().map(|s| s.contents.trim()).collect();
        let top_idx = texts
            .iter()
            .position(|t| t.contains("TOP_LEVEL"))
            .expect("top-level rule missing");
        let inner_idx = texts
            .iter()
            .position(|t| t.contains("INNER"))
            .expect("inner rule missing");
        assert!(
            top_idx < inner_idx,
            "ancestor should come before cwd so cwd wins on conflicts"
        );
    }

    #[test]
    fn discover_deduplicates_when_cwd_is_also_home() {
        // If ~/.claude/CLAUDE.md and cwd/CLAUDE.md are the same file via
        // symlink or the user runs from home itself, we must not add it twice.
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("CLAUDE.md"), "shared rule\n");

        let sources = discover_project_instructions(tmp.path());
        let count = sources
            .iter()
            .filter(|s| s.contents.contains("shared rule"))
            .count();
        assert_eq!(count, 1, "duplicate suppression failed");
    }

    #[test]
    fn render_instructions_produces_expected_headers() {
        let src = InstructionSource {
            path: PathBuf::from("/tmp/x/CLAUDE.md"),
            contents: "be concise\n".to_string(),
        };
        let rendered = render_instructions(&[src]);
        assert!(rendered.contains("## Project and User Instructions"));
        assert!(rendered.contains("### From /tmp/x/CLAUDE.md"));
        assert!(rendered.contains("be concise"));
    }

    #[test]
    fn render_instructions_empty_is_empty_string() {
        assert_eq!(render_instructions(&[]), "");
    }

    #[test]
    fn test_parse_import_line() {
        assert_eq!(parse_import_line("@file.md"), Some("file.md"));
        assert_eq!(
            parse_import_line("@path/to/file.md"),
            Some("path/to/file.md")
        );
        assert_eq!(
            parse_import_line("  @../relative.md  "),
            Some("../relative.md")
        );
        assert_eq!(parse_import_line("@@not-an-import"), None);
        assert_eq!(parse_import_line("@mention with spaces"), None);
        assert_eq!(parse_import_line("regular text"), None);
    }
}
