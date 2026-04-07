//! Integration tests for SKILL.md parsing.
//!
//! Tests the public `parse_skill_file` and `parse_skill_metadata` functions
//! using skill content written to temporary files.

use brainwires_agents::skills::{
    SkillExecutionMode, parse_skill_file, parse_skill_metadata, render_template,
};
use std::collections::HashMap;
use tempfile::TempDir;

/// Helper: write a SKILL.md string to a temp file and return the path.
fn write_skill(dir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(filename);
    std::fs::write(&path, content).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Metadata-only parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_metadata_extracts_all_fields() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: deploy-app
description: Deploys the application to staging or production environments
allowed-tools:
  - Bash
  - Read
license: MIT
compatibility: Requires docker and kubectl
model: claude-sonnet-4
metadata:
  category: devops
  execution: subagent
  author: test-user
hooks:
  - agent_started
  - tool_after_execute
---

# Deploy Instructions

Run the deploy pipeline.
"#;
    let path = write_skill(&dir, "deploy-app.md", content);
    let meta = parse_skill_metadata(&path).unwrap();

    assert_eq!(meta.name, "deploy-app");
    assert_eq!(
        meta.description,
        "Deploys the application to staging or production environments"
    );
    assert_eq!(
        meta.allowed_tools,
        Some(vec!["Bash".to_string(), "Read".to_string()])
    );
    assert_eq!(meta.license, Some("MIT".to_string()));
    assert_eq!(
        meta.compatibility,
        Some("Requires docker and kubectl".to_string())
    );
    assert_eq!(meta.model, Some("claude-sonnet-4".to_string()));
    assert_eq!(meta.get_metadata("category"), Some(&"devops".to_string()));
    assert_eq!(meta.get_metadata("author"), Some(&"test-user".to_string()));
    assert_eq!(meta.execution_mode(), SkillExecutionMode::Subagent);
    assert_eq!(
        meta.hooks,
        Some(vec![
            "agent_started".to_string(),
            "tool_after_execute".to_string()
        ])
    );
}

#[test]
fn parse_metadata_minimal_skill() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: hello
description: A minimal skill with no optional fields
---

Say hello.
"#;
    let path = write_skill(&dir, "hello.md", content);
    let meta = parse_skill_metadata(&path).unwrap();

    assert_eq!(meta.name, "hello");
    assert!(meta.allowed_tools.is_none());
    assert!(meta.license.is_none());
    assert!(meta.compatibility.is_none());
    assert!(meta.model.is_none());
    assert!(meta.metadata.is_none());
    assert!(meta.hooks.is_none());
    assert_eq!(meta.execution_mode(), SkillExecutionMode::Inline);
}

#[test]
fn parse_metadata_space_delimited_allowed_tools() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: git-helper
description: Helps with git operations
allowed-tools: Bash(git:*) Read Grep
---

Help with git.
"#;
    let path = write_skill(&dir, "git-helper.md", content);
    let meta = parse_skill_metadata(&path).unwrap();

    assert_eq!(
        meta.allowed_tools,
        Some(vec![
            "Bash(git:*)".to_string(),
            "Read".to_string(),
            "Grep".to_string(),
        ])
    );
}

// ---------------------------------------------------------------------------
// Full skill parsing (metadata + instructions)
// ---------------------------------------------------------------------------

#[test]
fn parse_full_skill_extracts_instructions() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: review-pr
description: Reviews pull requests for code quality
metadata:
  execution: subagent
---

# PR Review Instructions

When reviewing a pull request:

1. Check for code quality issues
2. Look for security vulnerabilities
3. Verify test coverage

## Output Format

Provide a structured review with sections for each concern.
"#;
    let path = write_skill(&dir, "review-pr.md", content);
    let skill = parse_skill_file(&path).unwrap();

    assert_eq!(skill.name(), "review-pr");
    assert_eq!(skill.execution_mode, SkillExecutionMode::Subagent);
    assert!(skill.runs_as_subagent());
    assert!(!skill.is_script());
    assert!(skill.instructions.contains("PR Review Instructions"));
    assert!(skill.instructions.contains("security vulnerabilities"));
    assert!(skill.instructions.contains("## Output Format"));
}

#[test]
fn parse_skill_with_script_execution_mode() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: lint-check
description: Runs linting on the project
metadata:
  execution: script
---

let result = run_tool("Bash", #{"command": "cargo clippy"});
result
"#;
    let path = write_skill(&dir, "lint-check.md", content);
    let skill = parse_skill_file(&path).unwrap();

    assert_eq!(skill.execution_mode, SkillExecutionMode::Script);
    assert!(skill.is_script());
    assert!(!skill.runs_as_subagent());
    assert!(skill.instructions.contains("cargo clippy"));
}

#[test]
fn parse_skill_default_inline_mode() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: explain
description: Explains code in detail
---

Read the code and explain it step by step.
"#;
    let path = write_skill(&dir, "explain.md", content);
    let skill = parse_skill_file(&path).unwrap();

    assert_eq!(skill.execution_mode, SkillExecutionMode::Inline);
    assert!(!skill.runs_as_subagent());
    assert!(!skill.is_script());
}

// ---------------------------------------------------------------------------
// Validation errors
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_missing_frontmatter() {
    let dir = TempDir::new().unwrap();
    let path = write_skill(&dir, "bad.md", "No frontmatter here at all.");
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_invalid_name_uppercase() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: Bad-Name
description: Has uppercase letters
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_name_with_consecutive_hyphens() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: bad--name
description: Has consecutive hyphens
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_name_starting_with_hyphen() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: -leading
description: Starts with a hyphen
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_name_ending_with_hyphen() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: trailing-
description: Ends with a hyphen
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_empty_description() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: empty-desc
description: ""
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_description_too_long() {
    let dir = TempDir::new().unwrap();
    let long_desc = "a".repeat(1025);
    let content = format!(
        "---\nname: long-desc\ndescription: {}\n---\n\nInstructions\n",
        long_desc
    );
    let path = write_skill(&dir, "bad.md", &content);
    assert!(parse_skill_file(&path).is_err());
}

#[test]
fn parse_rejects_name_with_underscore() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: bad_name
description: Uses underscore
---

Instructions
"#;
    let path = write_skill(&dir, "bad.md", content);
    assert!(parse_skill_file(&path).is_err());
}

// ---------------------------------------------------------------------------
// Tool restriction checks via parsed metadata
// ---------------------------------------------------------------------------

#[test]
fn parsed_skill_tool_restrictions() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: restricted
description: Skill with tool restrictions
allowed-tools:
  - Read
  - Grep
---

Instructions
"#;
    let path = write_skill(&dir, "restricted.md", content);
    let skill = parse_skill_file(&path).unwrap();

    assert!(skill.metadata.has_tool_restrictions());
    assert!(skill.metadata.is_tool_allowed("Read"));
    assert!(skill.metadata.is_tool_allowed("Grep"));
    assert!(!skill.metadata.is_tool_allowed("Write"));
    assert!(!skill.metadata.is_tool_allowed("Bash"));
}

#[test]
fn parsed_skill_no_tool_restrictions_allows_all() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: unrestricted
description: Skill without tool restrictions
---

Instructions
"#;
    let path = write_skill(&dir, "unrestricted.md", content);
    let skill = parse_skill_file(&path).unwrap();

    assert!(!skill.metadata.has_tool_restrictions());
    assert!(skill.metadata.is_tool_allowed("anything"));
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

#[test]
fn render_template_substitutes_variables() {
    let template = "Review PR #{{pr_number}} in repo {{repo}}";
    let mut args = HashMap::new();
    args.insert("pr_number".to_string(), "42".to_string());
    args.insert("repo".to_string(), "brainwires".to_string());

    let rendered = render_template(template, &args);
    assert_eq!(rendered, "Review PR #42 in repo brainwires");
}

#[test]
fn render_template_preserves_unmatched_placeholders() {
    let template = "Hello {{name}}, welcome to {{place}}";
    let mut args = HashMap::new();
    args.insert("name".to_string(), "Alice".to_string());

    let rendered = render_template(template, &args);
    assert!(rendered.contains("Alice"));
    // Unmatched placeholder stays
    assert!(rendered.contains("{{place}}"));
}

#[test]
fn render_template_conditional_truthy() {
    let template = "Start{{#if verbose}} with details: {{verbose}}{{/if}} end";
    let mut args = HashMap::new();
    args.insert("verbose".to_string(), "yes".to_string());

    let rendered = render_template(template, &args);
    assert!(rendered.contains("with details: yes"));
}

#[test]
fn render_template_conditional_falsy() {
    let template = "Start{{#if verbose}} with details{{/if}} end";
    let mut args = HashMap::new();
    args.insert("verbose".to_string(), "".to_string());

    let rendered = render_template(template, &args);
    assert_eq!(rendered, "Start end");
}

#[test]
fn render_template_conditional_missing_var_removed() {
    let template = "Start{{#if missing}} hidden{{/if}} end";
    let args = HashMap::new();

    let rendered = render_template(template, &args);
    assert_eq!(rendered, "Start end");
}

#[test]
fn render_template_with_empty_args() {
    let template = "No placeholders here";
    let args = HashMap::new();

    let rendered = render_template(template, &args);
    assert_eq!(rendered, "No placeholders here");
}

// ---------------------------------------------------------------------------
// Multiline description parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_multiline_yaml_description() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: multi
description: |
  A multiline description
  that spans several lines
  and preserves newlines.
---

Instructions here.
"#;
    let path = write_skill(&dir, "multi.md", content);
    let meta = parse_skill_metadata(&path).unwrap();

    assert!(meta.description.contains("multiline description"));
    assert!(meta.description.contains("spans several lines"));
}

// ---------------------------------------------------------------------------
// Subdirectory layout (SKILL.md inside a named directory)
// ---------------------------------------------------------------------------

#[test]
fn parse_skill_from_subdirectory_layout() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("my-skill");
    std::fs::create_dir(&skill_dir).unwrap();

    let content = r#"---
name: my-skill
description: A skill in a named subdirectory
allowed-tools:
  - Read
---

Do my-skill things.
"#;
    let path = skill_dir.join("SKILL.md");
    std::fs::write(&path, content).unwrap();

    let skill = parse_skill_file(&path).unwrap();
    assert_eq!(skill.name(), "my-skill");
    assert!(skill.instructions.contains("Do my-skill things."));
}
