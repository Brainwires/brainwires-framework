//! Loading side of the per-project memory system.
//!
//! The write side is in [`crate::tools::memory::MemoryTool`]. This module
//! handles discovery + rendering for injection into the system prompt.
//!
//! We deliberately stay light: load the `MEMORY.md` index if present,
//! optionally tail the individual memory files to give the model the full
//! typed notes. Truncation caps both the index and the body dump so a very
//! chatty project doesn't blow the context window.

use std::path::Path;

use crate::utils::paths::PlatformPaths;

/// Truncation caps chosen to match the harness behavior our global CLAUDE.md
/// describes. Large memory dirs get a "…truncated" marker.
const MAX_INDEX_LINES: usize = 200;
const MAX_BODY_FILES: usize = 25;
const MAX_BODY_BYTES: usize = 64 * 1024;

/// One typed memory file on disk, with an optional trimmed body for system
/// prompt display.
#[derive(Debug, Clone)]
pub struct LoadedMemoryFile {
    pub name: String,
    pub body: String,
}

/// The full set of memory state for a cwd. `index` is [`MEMORY.md`];
/// `files` are the typed memory bodies, in alphabetical order.
#[derive(Debug, Clone, Default)]
pub struct LoadedMemory {
    pub index: String,
    pub files: Vec<LoadedMemoryFile>,
}

impl LoadedMemory {
    pub fn is_empty(&self) -> bool {
        self.index.trim().is_empty() && self.files.is_empty()
    }
}

/// Load the index + (at most) `MAX_BODY_FILES` typed memories for `cwd`.
/// Missing directory is not an error — returns an empty `LoadedMemory`.
pub fn load_memory_for_cwd(cwd: &Path) -> LoadedMemory {
    let dir = match PlatformPaths::project_memory_dir(cwd) {
        Ok(d) => d,
        Err(_) => return LoadedMemory::default(),
    };

    if !dir.exists() {
        return LoadedMemory::default();
    }

    let index = std::fs::read_to_string(dir.join("MEMORY.md")).unwrap_or_default();
    let index = truncate_lines(&index, MAX_INDEX_LINES);

    let mut files = Vec::new();
    let mut total_bytes: usize = 0;
    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        let mut paths: Vec<_> = read_dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("MEMORY.md"))
            .collect();
        paths.sort();

        for path in paths.into_iter().take(MAX_BODY_FILES) {
            let body = match std::fs::read_to_string(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .trim_end_matches(".md")
                .to_string();
            total_bytes = total_bytes.saturating_add(body.len());
            if total_bytes > MAX_BODY_BYTES {
                break;
            }
            files.push(LoadedMemoryFile { name, body });
        }
    }

    LoadedMemory { index, files }
}

/// Render the loaded memory into a system-prompt-ready block. Empty input
/// produces an empty string so callers can unconditionally concatenate.
pub fn render_memory(loaded: &LoadedMemory) -> String {
    if loaded.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Auto Memory (project-scoped)\n\n");
    if !loaded.index.trim().is_empty() {
        out.push_str(loaded.index.trim());
        out.push_str("\n\n");
    }
    for file in &loaded.files {
        out.push_str(&format!("### From {}.md\n\n", file.name));
        // Drop frontmatter — it's already captured in the index.
        let body = strip_frontmatter(&file.body);
        out.push_str(body.trim());
        out.push_str("\n\n");
    }
    out
}

fn truncate_lines(text: &str, max: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max * 80));
    for (count, line) in text.lines().enumerate() {
        if count >= max {
            out.push_str("…(truncated)\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn strip_frontmatter(text: &str) -> &str {
    let trimmed = text.trim_start_matches('\n');
    if let Some(rest) = trimmed.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        return &rest[end + 4..];
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_util::ENV_LOCK;
    use tempfile::TempDir;

    fn setup_temp_home() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("BRAINWIRES_MEMORY_ROOT", tmp.path());
        }
        (tmp, guard)
    }

    #[test]
    fn empty_dir_yields_empty_memory() {
        let (_home, _guard) = setup_temp_home();
        let mem = load_memory_for_cwd(std::path::Path::new("/tmp/no-such-proj"));
        assert!(mem.is_empty());
        assert_eq!(render_memory(&mem), "");
    }

    #[test]
    fn loads_index_and_file() {
        let (_home, _guard) = setup_temp_home();
        let cwd = std::path::PathBuf::from("/tmp/mem-proj");
        let dir = PlatformPaths::ensure_project_memory_dir(&cwd).unwrap();
        std::fs::write(
            dir.join("MEMORY.md"),
            "# Memory Index\n\n- [user role](user_role.md) — backend engineer\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("user_role.md"),
            "---\nname: user role\ndescription: backend engineer\ntype: user\n---\n\nsenior backend eng on auth.\n",
        )
        .unwrap();

        let mem = load_memory_for_cwd(&cwd);
        assert!(!mem.is_empty());
        assert_eq!(mem.files.len(), 1);
        let rendered = render_memory(&mem);
        assert!(rendered.contains("## Auto Memory"));
        assert!(rendered.contains("backend engineer"));
        assert!(rendered.contains("senior backend eng on auth"));
        // Frontmatter should be stripped in the rendered body.
        assert!(!rendered.contains("name: user role"));
    }

    #[test]
    fn strip_frontmatter_handles_missing() {
        assert_eq!(strip_frontmatter("no frontmatter\n"), "no frontmatter\n");
        assert_eq!(
            strip_frontmatter("---\nname: x\n---\nbody"),
            "\nbody"
        );
    }

    #[test]
    fn truncate_lines_caps_output() {
        let input = (0..500).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
        let out = truncate_lines(&input, 10);
        let lines = out.lines().collect::<Vec<_>>();
        // 10 real lines + truncation marker
        assert_eq!(lines.len(), 11);
        assert!(lines.last().unwrap().contains("truncated"));
    }
}
