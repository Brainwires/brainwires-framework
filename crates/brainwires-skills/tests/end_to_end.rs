//! End-to-end integration tests.
//!
//! These tests exercise the full lifecycle: parse skill files from disk,
//! register them in the registry, route queries to find matches, and
//! execute matched skills through the executor.

use brainwires_skills::{
    MatchSource, SkillExecutor, SkillRegistry, SkillResult, SkillRouter, SkillSource,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Create a realistic set of skill files in a temp directory.
fn setup_skills_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // 1. review-pr (subagent mode, restricted tools)
    std::fs::write(
        dir.path().join("review-pr.md"),
        r#"---
name: review-pr
description: Reviews pull requests for code quality, security issues, and best practices
allowed-tools:
  - Read
  - Grep
model: claude-sonnet-4
metadata:
  category: code-review
  execution: subagent
---

# PR Review Instructions

When reviewing a pull request:

1. Read all changed files
2. Check for security vulnerabilities
3. Verify code quality and style
4. Ensure test coverage

Provide a structured review with severity levels.
"#,
    )
    .unwrap();

    // 2. commit (inline mode)
    std::fs::write(
        dir.path().join("commit.md"),
        r#"---
name: commit
description: Creates well-formatted git commits following conventional commit standards
metadata:
  category: git
---

# Commit Instructions

Create a commit message following this format:

type(scope): description

Where type is one of: feat, fix, docs, style, refactor, test, chore
"#,
    )
    .unwrap();

    // 3. explain-code (inline, no restrictions)
    std::fs::write(
        dir.path().join("explain-code.md"),
        r#"---
name: explain-code
description: Explains code functionality in detail with step-by-step breakdowns
metadata:
  category: documentation
---

# Code Explanation

Read the code at {{file_path}} and explain:
1. What it does
2. How it works
3. Key design decisions
"#,
    )
    .unwrap();

    // 4. deploy-app in subdirectory (script mode)
    let deploy_dir = dir.path().join("deploy-app");
    std::fs::create_dir(&deploy_dir).unwrap();
    std::fs::write(
        deploy_dir.join("SKILL.md"),
        r#"---
name: deploy-app
description: Deploys the application to staging or production environments
allowed-tools:
  - Bash
metadata:
  category: devops
  execution: script
---

let env = "{{environment}}";
let result = run_tool("Bash", #{"command": "deploy --env " + env});
result
"#,
    )
    .unwrap();

    dir
}

// ---------------------------------------------------------------------------
// Full lifecycle: discover -> route -> execute
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_lifecycle_discover_route_execute_inline() {
    let dir = setup_skills_dir();

    // Step 1: Discover skills
    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    assert_eq!(registry.len(), 4);
    let reg = Arc::new(RwLock::new(registry));

    // Step 2: Route a query
    let router = SkillRouter::new(Arc::clone(&reg));
    let matches = router.match_skills("create a commit message").await;
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.skill_name == "commit"));

    // Step 3: Execute the matched skill
    let executor = SkillExecutor::new(Arc::clone(&reg));
    let result = executor
        .execute_by_name("commit", HashMap::new())
        .await
        .unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("commit"));
            assert!(instructions.contains("conventional commit"));
        }
        _ => panic!("Expected Inline result for commit skill"),
    }
}

#[tokio::test]
async fn full_lifecycle_discover_route_execute_subagent() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    // Route
    let router = SkillRouter::new(Arc::clone(&reg));
    let matches = router.match_skills("review my pull request").await;
    assert!(matches.iter().any(|m| m.skill_name == "review-pr"));

    // Execute
    let executor = SkillExecutor::new(Arc::clone(&reg));
    let result = executor
        .execute_by_name("review-pr", HashMap::new())
        .await
        .unwrap();

    match result {
        SkillResult::Subagent { agent_id } => {
            assert!(agent_id.starts_with("skill-review-pr-"));
        }
        _ => panic!("Expected Subagent result for review-pr skill"),
    }
}

#[tokio::test]
async fn full_lifecycle_discover_route_execute_script() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    // Execute deploy-app with template args
    let executor = SkillExecutor::new(Arc::clone(&reg));
    let mut args = HashMap::new();
    args.insert("environment".to_string(), "staging".to_string());

    let result = executor
        .execute_by_name("deploy-app", args)
        .await
        .unwrap();

    match result {
        SkillResult::Script { output, is_error } => {
            assert!(!is_error);
            assert!(output.contains("staging"));
        }
        _ => panic!("Expected Script result for deploy-app skill"),
    }
}

#[tokio::test]
async fn full_lifecycle_with_template_args() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    let executor = SkillExecutor::new(Arc::clone(&reg));
    let mut args = HashMap::new();
    args.insert("file_path".to_string(), "src/main.rs".to_string());

    let result = executor
        .execute_by_name("explain-code", args)
        .await
        .unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("src/main.rs"));
            assert!(instructions.contains("explain-code"));
        }
        _ => panic!("Expected Inline result"),
    }
}

// ---------------------------------------------------------------------------
// Explicit invocation (user types /skill-name directly)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn explicit_invocation_bypasses_routing() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    let router = SkillRouter::new(Arc::clone(&reg));

    // Verify the skill exists
    assert!(router.skill_exists("review-pr").await);

    // Create an explicit match (simulating /review-pr command)
    let explicit = router.explicit_match("review-pr");
    assert_eq!(explicit.confidence, 1.0);
    assert_eq!(explicit.source, MatchSource::Explicit);

    // Execute
    let executor = SkillExecutor::new(Arc::clone(&reg));
    let result = executor
        .execute_by_name(&explicit.skill_name, HashMap::new())
        .await
        .unwrap();

    assert!(matches!(result, SkillResult::Subagent { .. }));
}

// ---------------------------------------------------------------------------
// Prepare subagent with tool filtering (end-to-end)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prepare_subagent_end_to_end() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    // Load the full skill
    let skill = registry.get_skill("review-pr").unwrap().clone();
    let reg = Arc::new(RwLock::new(registry));

    let executor = SkillExecutor::new(reg);
    let available_tools = vec![
        "Read".to_string(),
        "Write".to_string(),
        "Grep".to_string(),
        "Bash".to_string(),
        "Edit".to_string(),
    ];

    let prepared = executor
        .prepare_subagent(&skill, &available_tools, HashMap::new())
        .await
        .unwrap();

    // review-pr only allows Read and Grep
    assert_eq!(prepared.allowed_tool_names.len(), 2);
    assert!(prepared.allowed_tool_names.contains(&"Read".to_string()));
    assert!(prepared.allowed_tool_names.contains(&"Grep".to_string()));
    assert!(!prepared
        .allowed_tool_names
        .contains(&"Write".to_string()));

    assert!(prepared.system_prompt.contains("review-pr"));
    assert_eq!(
        prepared.model_override,
        Some("claude-sonnet-4".to_string())
    );
}

// ---------------------------------------------------------------------------
// Project overrides personal in end-to-end scenario
// ---------------------------------------------------------------------------

#[tokio::test]
async fn project_overrides_personal_end_to_end() {
    let root = TempDir::new().unwrap();
    let personal = root.path().join("personal");
    let project = root.path().join("project");
    std::fs::create_dir(&personal).unwrap();
    std::fs::create_dir(&project).unwrap();

    // Personal version
    std::fs::write(
        personal.join("commit.md"),
        r#"---
name: commit
description: Personal commit helper - basic format
---

Just write a simple commit message.
"#,
    )
    .unwrap();

    // Project version (overrides personal)
    std::fs::write(
        project.join("commit.md"),
        r#"---
name: commit
description: Project commit helper - enforces conventional commits with scope
---

Use conventional commits with mandatory scope: type(scope): description
"#,
    )
    .unwrap();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[
            (personal, SkillSource::Personal),
            (project, SkillSource::Project),
        ])
        .unwrap();

    // Should have only 1 "commit" skill (project version)
    assert_eq!(registry.len(), 1);
    let meta = registry.get_metadata("commit").unwrap();
    assert_eq!(meta.source, SkillSource::Project);
    assert!(meta.description.contains("conventional commits"));

    let reg = Arc::new(RwLock::new(registry));
    let executor = SkillExecutor::new(reg);

    let result = executor
        .execute_by_name("commit", HashMap::new())
        .await
        .unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("conventional commits"));
            assert!(instructions.contains("mandatory scope"));
        }
        _ => panic!("Expected Inline result"),
    }
}

// ---------------------------------------------------------------------------
// Router suggestions formatting end-to-end
// ---------------------------------------------------------------------------

#[tokio::test]
async fn router_suggestions_end_to_end() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    let router = SkillRouter::new(reg);

    let matches = router.match_skills("review code quality").await;
    let suggestion = router.format_suggestions(&matches);

    if let Some(text) = suggestion {
        // Should suggest at least review-pr
        assert!(text.contains("may help"));
    }
}

// ---------------------------------------------------------------------------
// Reload picks up changes end-to-end
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reload_and_route_new_skill() {
    let dir = TempDir::new().unwrap();

    // Start with one skill
    std::fs::write(
        dir.path().join("original.md"),
        "---\nname: original\ndescription: Original skill\n---\n\nInstructions\n",
    )
    .unwrap();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    assert_eq!(registry.len(), 1);

    // Add a new skill
    std::fs::write(
        dir.path().join("new-feature.md"),
        "---\nname: new-feature\ndescription: A brand new feature for testing\n---\n\nNew instructions\n",
    )
    .unwrap();

    // Reload
    registry.reload().unwrap();
    assert_eq!(registry.len(), 2);

    let reg = Arc::new(RwLock::new(registry));
    let router = SkillRouter::new(Arc::clone(&reg));

    // Should be able to route to the new skill
    assert!(router.skill_exists("new-feature").await);

    // Should be able to execute it
    let executor = SkillExecutor::new(reg);
    let result = executor
        .execute_by_name("new-feature", HashMap::new())
        .await
        .unwrap();

    match result {
        SkillResult::Inline { instructions, .. } => {
            assert!(instructions.contains("New instructions"));
        }
        _ => panic!("Expected Inline result"),
    }
}

// ---------------------------------------------------------------------------
// Multiple execution modes in one registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mixed_execution_modes_coexist() {
    let dir = setup_skills_dir();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    let reg = Arc::new(RwLock::new(registry));

    let executor = SkillExecutor::new(Arc::clone(&reg));

    // Inline skill
    let inline_result = executor
        .execute_by_name("commit", HashMap::new())
        .await
        .unwrap();
    assert!(matches!(inline_result, SkillResult::Inline { .. }));

    // Subagent skill
    let subagent_result = executor
        .execute_by_name("review-pr", HashMap::new())
        .await
        .unwrap();
    assert!(matches!(subagent_result, SkillResult::Subagent { .. }));

    // Script skill
    let mut args = HashMap::new();
    args.insert("environment".to_string(), "prod".to_string());
    let script_result = executor
        .execute_by_name("deploy-app", args)
        .await
        .unwrap();
    assert!(matches!(script_result, SkillResult::Script { .. }));
}
