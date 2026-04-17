//! MemoryTool — agent-facing tools for managing per-project memory notes.
//!
//! Memory lives at `~/.brainwires/projects/<encoded-cwd>/memory/`:
//! - `MEMORY.md` — lightweight index, always injected into the system prompt.
//! - `<name>.md` — individual memory file with YAML frontmatter
//!   (`name`, `description`, `type`) and a markdown body.
//!
//! The index file is rewritten from scratch on every mutation so it stays
//! in sync with the actual files on disk — stray entries are pruned.
//!
//! Tools exposed:
//! - `memory_save(name, type, description, content)`
//! - `memory_delete(name)`
//! - `memory_list()`
//!
//! All three are local-filesystem writes; they do **not** run through the
//! approval flow. They are audit-logged by the executor like any other tool.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::types::tool::{Tool, ToolInputSchema, ToolResult};
use crate::utils::paths::PlatformPaths;

/// Recognized memory types — matches the categories used by the user's
/// global auto-memory CLAUDE.md.
const MEMORY_TYPES: &[&str] = &["user", "feedback", "project", "reference"];

/// The tool itself. Cheap to clone; all state lives on disk.
#[derive(Clone, Default)]
pub struct MemoryTool;

impl MemoryTool {
    pub fn new() -> Self {
        Self
    }

    pub fn get_tools() -> Vec<Tool> {
        vec![Self::save_tool(), Self::delete_tool(), Self::list_tool()]
    }

    fn save_tool() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            json!({
                "type": "string",
                "description": "Stable identifier for the memory. Used as the filename (slugified). Re-saving with the same name overwrites."
            }),
        );
        props.insert(
            "type".to_string(),
            json!({
                "type": "string",
                "enum": MEMORY_TYPES,
                "description": "One of: user, feedback, project, reference. See memory conventions for when to use each."
            }),
        );
        props.insert(
            "description".to_string(),
            json!({
                "type": "string",
                "description": "One-line hook — shown in MEMORY.md so future sessions can decide whether to load this memory."
            }),
        );
        props.insert(
            "content".to_string(),
            json!({
                "type": "string",
                "description": "The memory body in markdown. Lead with the rule/fact itself; for feedback/project types include a short **Why:** line."
            }),
        );
        Tool {
            name: "memory_save".to_string(),
            description: "Persist a typed memory note for this project. Written to ~/.brainwires/projects/<cwd>/memory/<name>.md and indexed in MEMORY.md. Re-saving with the same name updates in place."
                .to_string(),
            input_schema: ToolInputSchema::object(
                props,
                vec![
                    "name".to_string(),
                    "type".to_string(),
                    "description".to_string(),
                    "content".to_string(),
                ],
            ),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    fn delete_tool() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            json!({"type": "string", "description": "Name of the memory to delete."}),
        );
        Tool {
            name: "memory_delete".to_string(),
            description:
                "Remove a memory note and its entry in MEMORY.md. No-op if the memory doesn't exist."
                    .to_string(),
            input_schema: ToolInputSchema::object(props, vec!["name".to_string()]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    fn list_tool() -> Tool {
        Tool {
            name: "memory_list".to_string(),
            description:
                "Return the current MEMORY.md index as a string, so the agent can see what's stored."
                    .to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    pub async fn execute(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        input: &Value,
        cwd: &std::path::Path,
    ) -> ToolResult {
        match tool_name {
            "memory_save" => self.do_save(tool_use_id, input, cwd).await,
            "memory_delete" => self.do_delete(tool_use_id, input, cwd).await,
            "memory_list" => self.do_list(tool_use_id, cwd).await,
            other => ToolResult::error(
                tool_use_id.to_string(),
                format!("Unknown memory tool: {}", other),
            ),
        }
    }

    async fn do_save(
        &self,
        tool_use_id: &str,
        input: &Value,
        cwd: &std::path::Path,
    ) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            name: String,
            r#type: String,
            description: String,
            content: String,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid memory_save input: {}", e),
                );
            }
        };

        if !MEMORY_TYPES.contains(&args.r#type.as_str()) {
            return ToolResult::error(
                tool_use_id.to_string(),
                format!(
                    "type must be one of {:?}, got '{}'",
                    MEMORY_TYPES,
                    args.r#type
                ),
            );
        }

        let slug = slugify(&args.name);
        if slug.is_empty() {
            return ToolResult::error(
                tool_use_id.to_string(),
                "name must contain at least one alphanumeric character".to_string(),
            );
        }

        let dir = match PlatformPaths::ensure_project_memory_dir(cwd) {
            Ok(d) => d,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("failed to prepare memory dir: {}", e),
                );
            }
        };

        let file_path = dir.join(format!("{}.md", slug));
        let body = render_memory_file(&args.name, &args.description, &args.r#type, &args.content);

        if let Err(e) = std::fs::write(&file_path, body) {
            return ToolResult::error(
                tool_use_id.to_string(),
                format!("failed to write memory: {}", e),
            );
        }

        if let Err(e) = rewrite_index(cwd).await {
            return ToolResult::error(
                tool_use_id.to_string(),
                format!("failed to rewrite MEMORY.md: {}", e),
            );
        }

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({
                "saved": true,
                "name": args.name,
                "path": file_path,
            }))
            .unwrap_or_default(),
        )
    }

    async fn do_delete(
        &self,
        tool_use_id: &str,
        input: &Value,
        cwd: &std::path::Path,
    ) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            name: String,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid memory_delete input: {}", e),
                );
            }
        };

        let dir = match PlatformPaths::project_memory_dir(cwd) {
            Ok(d) => d,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("failed to locate memory dir: {}", e),
                );
            }
        };
        let file_path = dir.join(format!("{}.md", slugify(&args.name)));
        let existed = file_path.exists();
        if existed {
            if let Err(e) = std::fs::remove_file(&file_path) {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("failed to delete memory: {}", e),
                );
            }
        }

        if let Err(e) = rewrite_index(cwd).await {
            return ToolResult::error(
                tool_use_id.to_string(),
                format!("failed to rewrite MEMORY.md: {}", e),
            );
        }

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({
                "deleted": existed,
                "name": args.name,
            }))
            .unwrap_or_default(),
        )
    }

    async fn do_list(&self, tool_use_id: &str, cwd: &std::path::Path) -> ToolResult {
        let index = match PlatformPaths::memory_index_path(cwd) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("failed to locate memory index: {}", e),
                );
            }
        };
        let contents = std::fs::read_to_string(&index).unwrap_or_default();
        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({ "index": contents }))
                .unwrap_or_default(),
        )
    }
}

/// Turn an arbitrary memory name into a filesystem-safe slug.
/// Keeps alphanumerics + `_`/`-`; replaces anything else with `_`.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn render_memory_file(name: &str, description: &str, kind: &str, content: &str) -> String {
    let trimmed = content.trim_end();
    format!(
        "---\nname: {}\ndescription: {}\ntype: {}\n---\n\n{}\n",
        name, description, kind, trimmed
    )
}

/// Per-file summary used to rebuild MEMORY.md.
struct MemoryEntry {
    slug: String,
    name: String,
    description: String,
}

fn parse_frontmatter(text: &str) -> Option<MemoryEntry> {
    let text = text.trim_start();
    if !text.starts_with("---") {
        return None;
    }
    let rest = &text[3..];
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    let mut name = None;
    let mut description = None;
    for line in fm.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().to_string());
        }
    }
    Some(MemoryEntry {
        slug: String::new(), // filled by caller
        name: name?,
        description: description.unwrap_or_default(),
    })
}

async fn rewrite_index(cwd: &std::path::Path) -> anyhow::Result<()> {
    let dir = PlatformPaths::ensure_project_memory_dir(cwd)?;
    let index_path = dir.join("MEMORY.md");

    // Collect entries in stable (alphabetical) slug order via BTreeMap.
    let mut entries: BTreeMap<String, MemoryEntry> = BTreeMap::new();
    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if file_name == "MEMORY.md" {
                continue;
            }
            let slug = file_name.trim_end_matches(".md").to_string();
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            if let Some(mut e) = parse_frontmatter(&contents) {
                e.slug = slug.clone();
                entries.insert(slug, e);
            }
        }
    }

    let mut out = String::from("# Memory Index\n\n");
    if entries.is_empty() {
        out.push_str(
            "_No memories stored yet. Agents can add entries via the `memory_save` tool._\n",
        );
    } else {
        for e in entries.values() {
            out.push_str(&format!(
                "- [{}]({}.md) — {}\n",
                e.name, e.slug, e.description
            ));
        }
    }

    std::fs::write(&index_path, out)?;
    let _ = index_path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_util::ENV_LOCK;
    use tempfile::TempDir;

    /// Redirect memory storage into a tempdir via `BRAINWIRES_MEMORY_ROOT`,
    /// holding the shared env lock so parallel tests don't stomp each other.
    fn setup_temp_home() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("BRAINWIRES_MEMORY_ROOT", tmp.path());
        }
        (tmp, guard)
    }

    #[test]
    fn slugify_handles_common_cases() {
        assert_eq!(slugify("user role"), "user_role");
        assert_eq!(slugify("Feedback/Testing"), "feedbacktesting");
        assert_eq!(slugify("AbC-123"), "abc-123");
        assert_eq!(slugify("!!!"), "");
    }

    #[tokio::test]
    async fn save_then_list_then_delete_roundtrip() {
        let (_home, _guard) = setup_temp_home();
        let cwd = std::path::PathBuf::from("/tmp/testproj");
        let tool = MemoryTool::new();

        let save = tool
            .do_save(
                "t1",
                &json!({
                    "name": "user role",
                    "type": "user",
                    "description": "backend engineer on auth",
                    "content": "user is a senior backend engineer focused on auth.",
                }),
                &cwd,
            )
            .await;
        assert!(!save.is_error, "save failed: {}", save.content);

        let list = tool.do_list("t2", &cwd).await;
        assert!(!list.is_error);
        let v: Value = serde_json::from_str(&list.content).unwrap();
        let index = v["index"].as_str().unwrap();
        assert!(
            index.contains("user role"),
            "expected name in index: {}",
            index
        );
        assert!(
            index.contains("backend engineer on auth"),
            "expected description in index: {}",
            index
        );

        // Second save with same name should update in place (not duplicate).
        let save2 = tool
            .do_save(
                "t3",
                &json!({
                    "name": "user role",
                    "type": "user",
                    "description": "backend engineer on auth — updated",
                    "content": "updated content.",
                }),
                &cwd,
            )
            .await;
        assert!(!save2.is_error);

        let list2 = tool.do_list("t4", &cwd).await;
        let v2: Value = serde_json::from_str(&list2.content).unwrap();
        let idx2 = v2["index"].as_str().unwrap();
        let occurrences = idx2.matches("user role").count();
        assert_eq!(occurrences, 1, "duplicate entry: {}", idx2);
        assert!(
            idx2.contains("updated"),
            "expected updated description: {}",
            idx2
        );

        // Delete.
        let del = tool
            .do_delete("t5", &json!({"name": "user role"}), &cwd)
            .await;
        assert!(!del.is_error);
        let v3: Value = serde_json::from_str(&del.content).unwrap();
        assert_eq!(v3["deleted"], true);

        let list3 = tool.do_list("t6", &cwd).await;
        let v4: Value = serde_json::from_str(&list3.content).unwrap();
        let idx3 = v4["index"].as_str().unwrap();
        assert!(!idx3.contains("user role"), "still in index: {}", idx3);
    }

    #[tokio::test]
    async fn invalid_type_is_error() {
        let (_home, _guard) = setup_temp_home();
        let cwd = std::path::PathBuf::from("/tmp/testproj-t");
        let tool = MemoryTool::new();
        let r = tool
            .do_save(
                "t1",
                &json!({
                    "name": "x",
                    "type": "not-a-type",
                    "description": "d",
                    "content": "c",
                }),
                &cwd,
            )
            .await;
        assert!(r.is_error);
    }

    #[tokio::test]
    async fn delete_nonexistent_is_no_op() {
        let (_home, _guard) = setup_temp_home();
        let cwd = std::path::PathBuf::from("/tmp/testproj-del");
        let tool = MemoryTool::new();
        let r = tool
            .do_delete("t1", &json!({"name": "ghost"}), &cwd)
            .await;
        assert!(!r.is_error);
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["deleted"], false);
    }

    #[tokio::test]
    async fn index_prunes_orphaned_manual_deletion() {
        let (_home, _guard) = setup_temp_home();
        let cwd = std::path::PathBuf::from("/tmp/testproj-prune");
        let tool = MemoryTool::new();
        tool.do_save(
            "t1",
            &json!({
                "name": "keeper",
                "type": "project",
                "description": "stays",
                "content": "body",
            }),
            &cwd,
        )
        .await;
        tool.do_save(
            "t2",
            &json!({
                "name": "goner",
                "type": "project",
                "description": "gone",
                "content": "body",
            }),
            &cwd,
        )
        .await;

        // Simulate the user manually removing a file outside the tool.
        let dir = PlatformPaths::project_memory_dir(&cwd).unwrap();
        std::fs::remove_file(dir.join("goner.md")).unwrap();

        // A save/delete of something else rewrites the index — index must
        // no longer reference "goner".
        tool.do_delete("t3", &json!({"name": "nothing"}), &cwd)
            .await;
        let contents = std::fs::read_to_string(dir.join("MEMORY.md")).unwrap();
        assert!(!contents.contains("goner"), "{}", contents);
        assert!(contents.contains("keeper"), "{}", contents);
    }
}
