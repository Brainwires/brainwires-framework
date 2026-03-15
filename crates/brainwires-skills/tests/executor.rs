//! Integration tests for the SkillExecutor.
//!
//! Tests inline/subagent/script execution, template rendering through
//! the executor, tool filtering, and prepare_* methods.

use brainwires_skills::{
    Skill, SkillExecutionMode, SkillExecutor, SkillMetadata, SkillRegistry, SkillResult,
    SkillSource,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Helper: build a Skill directly (without file I/O).
fn make_skill(
    name: &str,
    description: &str,
    instructions: &str,
    mode: SkillExecutionMode,
    allowed_tools: Option<Vec<String>>,
    model: Option<String>,
) -> Skill {
    let mut meta = SkillMetadata::new(name.to_string(), description.to_string());
    meta.allowed_tools = allowed_tools;
    meta.model = model;

    if mode == SkillExecutionMode::Subagent {
        let mut m = HashMap::new();
        m.insert("execution".to_string(), "subagent".to_string());
        meta.metadata = Some(m);
    } else if mode == SkillExecutionMode::Script {
        let mut m = HashMap::new();
        m.insert("execution".to_string(), "script".to_string());
        meta.metadata = Some(m);
    }

    Skill {
        metadata: meta,
        instructions: instructions.to_string(),
        execution_mode: mode,
    }
}

fn empty_registry() -> Arc<RwLock<SkillRegistry>> {
    Arc::new(RwLock::new(SkillRegistry::new()))
}

// ---------------------------------------------------------------------------
// Inline execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn execute_inline_returns_instructions_with_skill_name() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "greet",
        "Greets the user",
        "Hello, welcome!",
        SkillExecutionMode::Inline,
        None,
        None,
    );

    let result = executor.execute(&skill, HashMap::new()).await.unwrap();

    match result {
        SkillResult::Inline {
            instructions,
            model_override,
        } => {
            assert!(instructions.contains("greet"));
            assert!(instructions.contains("Hello, welcome!"));
            assert!(model_override.is_none());
        }
        _ => panic!("Expected Inline result"),
    }
}

#[tokio::test]
async fn execute_inline_with_model_override() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "smart",
        "Uses a specific model",
        "Think deeply.",
        SkillExecutionMode::Inline,
        None,
        Some("claude-opus-4".to_string()),
    );

    let result = executor.execute(&skill, HashMap::new()).await.unwrap();

    match result {
        SkillResult::Inline { model_override, .. } => {
            assert_eq!(model_override, Some("claude-opus-4".to_string()));
        }
        _ => panic!("Expected Inline result"),
    }
}

#[tokio::test]
async fn execute_inline_renders_template_args() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "pr-review",
        "Reviews PRs",
        "Review PR #{{pr_number}} in {{repo}}",
        SkillExecutionMode::Inline,
        None,
        None,
    );

    let mut args = HashMap::new();
    args.insert("pr_number".to_string(), "42".to_string());
    args.insert("repo".to_string(), "brainwires".to_string());

    let result = executor.execute(&skill, args).await.unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("PR #42"));
            assert!(instructions.contains("brainwires"));
        }
        _ => panic!("Expected Inline result"),
    }
}

// ---------------------------------------------------------------------------
// Subagent execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn execute_subagent_returns_agent_id() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "background-task",
        "Runs in background",
        "Do complex work.",
        SkillExecutionMode::Subagent,
        None,
        None,
    );

    let result = executor.execute(&skill, HashMap::new()).await.unwrap();

    match result {
        SkillResult::Subagent { ref agent_id } => {
            assert!(agent_id.starts_with("skill-background-task-"));
            assert!(!result.is_error());
        }
        _ => panic!("Expected Subagent result"),
    }
}

// ---------------------------------------------------------------------------
// Script execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn execute_script_returns_script_content() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "auto-lint",
        "Runs linting",
        r#"let result = run_tool("Bash", #{"command": "cargo clippy"}); result"#,
        SkillExecutionMode::Script,
        None,
        None,
    );

    let result = executor.execute(&skill, HashMap::new()).await.unwrap();

    match result {
        SkillResult::Script { output, is_error } => {
            assert!(!is_error);
            assert!(output.contains("cargo clippy"));
        }
        _ => panic!("Expected Script result"),
    }
}

#[tokio::test]
async fn execute_script_renders_template_args() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "run-cmd",
        "Runs a command",
        r#"let cmd = "{{command}}"; run(cmd);"#,
        SkillExecutionMode::Script,
        None,
        None,
    );

    let mut args = HashMap::new();
    args.insert("command".to_string(), "cargo test".to_string());

    let result = executor.execute(&skill, args).await.unwrap();

    match result {
        SkillResult::Script { output, .. } => {
            assert!(output.contains("cargo test"));
        }
        _ => panic!("Expected Script result"),
    }
}

// ---------------------------------------------------------------------------
// execute_by_name (loads from registry)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn execute_by_name_loads_from_registry() {
    let dir = TempDir::new().unwrap();
    let content = r#"---
name: from-disk
description: Loaded from disk via registry
---

Disk-based instructions with {{arg}}.
"#;
    std::fs::write(dir.path().join("from-disk.md"), content).unwrap();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    let executor = SkillExecutor::new(reg);
    let mut args = HashMap::new();
    args.insert("arg".to_string(), "hello".to_string());

    let result = executor.execute_by_name("from-disk", args).await.unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("hello"));
            assert!(instructions.contains("from-disk"));
        }
        _ => panic!("Expected Inline result"),
    }
}

#[tokio::test]
async fn execute_by_name_nonexistent_skill_errors() {
    let executor = SkillExecutor::new(empty_registry());
    let result = executor.execute_by_name("ghost", HashMap::new()).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tool filtering via prepare_subagent / prepare_script
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prepare_subagent_filters_tools() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "restricted-agent",
        "Runs with limited tools",
        "Do restricted work with {{task}}.",
        SkillExecutionMode::Subagent,
        Some(vec!["Read".to_string(), "Grep".to_string()]),
        Some("claude-sonnet-4".to_string()),
    );

    let available = vec![
        "Read".to_string(),
        "Write".to_string(),
        "Grep".to_string(),
        "Bash".to_string(),
        "Edit".to_string(),
    ];

    let mut args = HashMap::new();
    args.insert("task".to_string(), "analysis".to_string());

    let prepared = executor
        .prepare_subagent(&skill, &available, args)
        .await
        .unwrap();

    assert_eq!(prepared.allowed_tool_names.len(), 2);
    assert!(prepared.allowed_tool_names.contains(&"Read".to_string()));
    assert!(prepared.allowed_tool_names.contains(&"Grep".to_string()));
    assert!(!prepared.allowed_tool_names.contains(&"Write".to_string()));
    assert!(prepared.task_description.contains("analysis"));
    assert!(prepared.system_prompt.contains("restricted-agent"));
    assert_eq!(prepared.model_override, Some("claude-sonnet-4".to_string()));
}

#[tokio::test]
async fn prepare_subagent_no_restrictions_passes_all_tools() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "unrestricted-agent",
        "No tool limits",
        "Do anything.",
        SkillExecutionMode::Subagent,
        None,
        None,
    );

    let available = vec!["Read".to_string(), "Write".to_string(), "Bash".to_string()];

    let prepared = executor
        .prepare_subagent(&skill, &available, HashMap::new())
        .await
        .unwrap();

    assert_eq!(prepared.allowed_tool_names.len(), 3);
}

#[tokio::test]
async fn prepare_script_filters_tools_and_renders() {
    let executor = SkillExecutor::new(empty_registry());
    let skill = make_skill(
        "script-skill",
        "A script skill",
        r#"let x = {{value}}; x + 1;"#,
        SkillExecutionMode::Script,
        Some(vec!["Bash".to_string()]),
        None,
    );

    let available = vec!["Read".to_string(), "Bash".to_string(), "Write".to_string()];

    let mut args = HashMap::new();
    args.insert("value".to_string(), "10".to_string());

    let prepared = executor
        .prepare_script(&skill, &available, args)
        .await
        .unwrap();

    assert!(prepared.script_content.contains("let x = 10"));
    assert_eq!(prepared.allowed_tool_names, vec!["Bash".to_string()]);
    assert_eq!(prepared.skill_name, "script-skill");
    assert!(prepared.model_override.is_none());
}

// ---------------------------------------------------------------------------
// get_execution_mode
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_execution_mode_from_registered_skill() {
    let mut registry = SkillRegistry::new();

    let mut meta = SkillMetadata::new("sub-skill".to_string(), "Subagent skill".to_string());
    let mut m = HashMap::new();
    m.insert("execution".to_string(), "subagent".to_string());
    meta.metadata = Some(m);
    registry.register(meta);

    registry.register(SkillMetadata::new(
        "inline-skill".to_string(),
        "Inline skill".to_string(),
    ));

    let reg = Arc::new(RwLock::new(registry));
    let executor = SkillExecutor::new(reg);

    assert_eq!(
        executor.get_execution_mode("sub-skill").await.unwrap(),
        SkillExecutionMode::Subagent
    );
    assert_eq!(
        executor.get_execution_mode("inline-skill").await.unwrap(),
        SkillExecutionMode::Inline
    );
    assert!(executor.get_execution_mode("nope").await.is_err());
}
