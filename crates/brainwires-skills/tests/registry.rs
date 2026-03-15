//! Integration tests for the SkillRegistry.
//!
//! Tests skill discovery from directories, registration, lookup, listing,
//! category filtering, source overrides, and lazy loading.

use brainwires_skills::{SkillExecutionMode, SkillMetadata, SkillRegistry, SkillSource};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: create a SKILL.md file in a directory with the given name and description.
fn create_skill_file(dir: &std::path::Path, name: &str, description: &str) {
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {} Instructions\n\nDo the thing.\n",
        name, description, name
    );
    let path = dir.join(format!("{}.md", name));
    std::fs::write(path, content).unwrap();
}

/// Helper: create a skill with custom metadata fields.
fn create_skill_file_with_metadata(
    dir: &std::path::Path,
    name: &str,
    description: &str,
    extra_yaml: &str,
) {
    let content = format!(
        "---\nname: {}\ndescription: {}\n{}\n---\n\n# {} Instructions\n\nDo the thing.\n",
        name, description, extra_yaml, name
    );
    let path = dir.join(format!("{}.md", name));
    std::fs::write(path, content).unwrap();
}

// ---------------------------------------------------------------------------
// Basic registration and lookup
// ---------------------------------------------------------------------------

#[test]
fn register_and_lookup_skill() {
    let mut registry = SkillRegistry::new();
    assert!(registry.is_empty());

    let meta = SkillMetadata::new("test-skill".to_string(), "A test skill".to_string());
    registry.register(meta);

    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains("test-skill"));
    assert!(!registry.contains("nonexistent"));

    let retrieved = registry.get_metadata("test-skill").unwrap();
    assert_eq!(retrieved.name, "test-skill");
    assert_eq!(retrieved.description, "A test skill");
}

#[test]
fn register_multiple_skills_listed_sorted() {
    let mut registry = SkillRegistry::new();

    registry.register(SkillMetadata::new("zeta".to_string(), "Z".to_string()));
    registry.register(SkillMetadata::new("alpha".to_string(), "A".to_string()));
    registry.register(SkillMetadata::new("middle".to_string(), "M".to_string()));

    let names = registry.list_skills();
    assert_eq!(names, vec!["alpha", "middle", "zeta"]);
}

#[test]
fn remove_skill() {
    let mut registry = SkillRegistry::new();
    registry.register(SkillMetadata::new(
        "removable".to_string(),
        "Will be removed".to_string(),
    ));
    assert!(registry.contains("removable"));

    let removed = registry.remove("removable");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "removable");
    assert!(!registry.contains("removable"));
    assert!(registry.is_empty());
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut registry = SkillRegistry::new();
    assert!(registry.remove("ghost").is_none());
}

// ---------------------------------------------------------------------------
// Discovery from directories
// ---------------------------------------------------------------------------

#[test]
fn discover_skills_from_flat_files() {
    let dir = TempDir::new().unwrap();
    create_skill_file(dir.path(), "skill-a", "First skill");
    create_skill_file(dir.path(), "skill-b", "Second skill");

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    assert_eq!(registry.len(), 2);
    assert!(registry.contains("skill-a"));
    assert!(registry.contains("skill-b"));

    // Source should be set correctly
    assert_eq!(
        registry.get_metadata("skill-a").unwrap().source,
        SkillSource::Personal
    );
}

#[test]
fn discover_skills_from_subdirectories() {
    let dir = TempDir::new().unwrap();

    let sub = dir.path().join("my-skill");
    std::fs::create_dir(&sub).unwrap();
    let content = "---\nname: my-skill\ndescription: A subdirectory skill\n---\n\nInstructions\n";
    std::fs::write(sub.join("SKILL.md"), content).unwrap();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Project)])
        .unwrap();

    assert!(registry.contains("my-skill"));
    assert_eq!(
        registry.get_metadata("my-skill").unwrap().source,
        SkillSource::Project
    );
}

#[test]
fn discover_from_nonexistent_directory_is_ok() {
    let mut registry = SkillRegistry::new();
    let result = registry.discover_from(&[(
        PathBuf::from("/nonexistent/path/that/does/not/exist"),
        SkillSource::Personal,
    )]);
    assert!(result.is_ok());
    assert!(registry.is_empty());
}

// ---------------------------------------------------------------------------
// Source priority: project overrides personal
// ---------------------------------------------------------------------------

#[test]
fn project_skills_override_personal_with_same_name() {
    let root = TempDir::new().unwrap();

    let personal_dir = root.path().join("personal");
    let project_dir = root.path().join("project");
    std::fs::create_dir(&personal_dir).unwrap();
    std::fs::create_dir(&project_dir).unwrap();

    create_skill_file(&personal_dir, "shared", "Personal version");
    create_skill_file(&project_dir, "shared", "Project version");

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[
            (personal_dir, SkillSource::Personal),
            (project_dir, SkillSource::Project),
        ])
        .unwrap();

    assert_eq!(registry.len(), 1);
    let meta = registry.get_metadata("shared").unwrap();
    assert_eq!(meta.source, SkillSource::Project);
    assert_eq!(meta.description, "Project version");
}

// ---------------------------------------------------------------------------
// Lazy loading (get_skill)
// ---------------------------------------------------------------------------

#[test]
fn lazy_load_skill_from_disk() {
    let dir = TempDir::new().unwrap();
    create_skill_file(dir.path(), "lazy", "A lazily loaded skill");

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    // Metadata should be available without loading full content
    assert!(registry.get_metadata("lazy").is_some());

    // Load full skill
    let skill = registry.get_skill("lazy").unwrap();
    assert_eq!(skill.name(), "lazy");
    assert!(skill.instructions.contains("Instructions"));
    assert!(skill.instructions.contains("Do the thing."));
}

#[test]
fn get_skill_nonexistent_returns_error() {
    let mut registry = SkillRegistry::new();
    assert!(registry.get_skill("does-not-exist").is_err());
}

// ---------------------------------------------------------------------------
// Reload
// ---------------------------------------------------------------------------

#[test]
fn reload_picks_up_new_skills() {
    let dir = TempDir::new().unwrap();
    create_skill_file(dir.path(), "original", "Original skill");

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();
    assert_eq!(registry.len(), 1);

    // Add another skill to disk
    create_skill_file(dir.path(), "added-later", "Added after initial discovery");

    registry.reload().unwrap();
    assert_eq!(registry.len(), 2);
    assert!(registry.contains("added-later"));
}

// ---------------------------------------------------------------------------
// Category filtering
// ---------------------------------------------------------------------------

#[test]
fn filter_skills_by_category() {
    let mut registry = SkillRegistry::new();

    let mut devops_meta = HashMap::new();
    devops_meta.insert("category".to_string(), "devops".to_string());
    let mut s1 = SkillMetadata::new("deploy".to_string(), "Deploy app".to_string());
    s1.metadata = Some(devops_meta);

    let mut testing_meta = HashMap::new();
    testing_meta.insert("category".to_string(), "testing".to_string());
    let mut s2 = SkillMetadata::new("test-runner".to_string(), "Run tests".to_string());
    s2.metadata = Some(testing_meta);

    let s3 = SkillMetadata::new("no-category".to_string(), "No category".to_string());

    registry.register(s1);
    registry.register(s2);
    registry.register(s3);

    let devops = registry.skills_by_category("devops");
    assert_eq!(devops.len(), 1);
    assert_eq!(devops[0].name, "deploy");

    let testing = registry.skills_by_category("testing");
    assert_eq!(testing.len(), 1);
    assert_eq!(testing[0].name, "test-runner");

    let unknown = registry.skills_by_category("unknown");
    assert!(unknown.is_empty());
}

// ---------------------------------------------------------------------------
// Source filtering
// ---------------------------------------------------------------------------

#[test]
fn filter_skills_by_source() {
    let mut registry = SkillRegistry::new();

    let personal = SkillMetadata::new("personal-skill".to_string(), "Personal".to_string())
        .with_source(SkillSource::Personal);
    let project = SkillMetadata::new("project-skill".to_string(), "Project".to_string())
        .with_source(SkillSource::Project);
    let builtin = SkillMetadata::new("builtin-skill".to_string(), "Builtin".to_string())
        .with_source(SkillSource::Builtin);

    registry.register(personal);
    registry.register(project);
    registry.register(builtin);

    assert_eq!(registry.skills_by_source(SkillSource::Personal).len(), 1);
    assert_eq!(registry.skills_by_source(SkillSource::Project).len(), 1);
    assert_eq!(registry.skills_by_source(SkillSource::Builtin).len(), 1);
}

// ---------------------------------------------------------------------------
// all_metadata
// ---------------------------------------------------------------------------

#[test]
fn all_metadata_returns_every_skill() {
    let mut registry = SkillRegistry::new();
    registry.register(SkillMetadata::new("a".to_string(), "A".to_string()));
    registry.register(SkillMetadata::new("b".to_string(), "B".to_string()));
    registry.register(SkillMetadata::new("c".to_string(), "C".to_string()));

    let all = registry.all_metadata();
    assert_eq!(all.len(), 3);
}

// ---------------------------------------------------------------------------
// Format listings
// ---------------------------------------------------------------------------

#[test]
fn format_skill_list_empty() {
    let registry = SkillRegistry::new();
    let listing = registry.format_skill_list();
    assert!(listing.contains("No skills available"));
}

#[test]
fn format_skill_list_with_skills() {
    let mut registry = SkillRegistry::new();
    registry.register(
        SkillMetadata::new("my-skill".to_string(), "Does something useful".to_string())
            .with_source(SkillSource::Personal),
    );

    let listing = registry.format_skill_list();
    assert!(listing.contains("my-skill"));
    assert!(listing.contains("Does something useful"));
}

#[test]
fn format_skill_detail() {
    let mut registry = SkillRegistry::new();
    let mut meta = SkillMetadata::new(
        "detail-skill".to_string(),
        "A skill with all the details".to_string(),
    );
    meta.allowed_tools = Some(vec!["Read".to_string()]);
    meta.license = Some("Apache-2.0".to_string());
    meta.model = Some("gpt-4".to_string());

    let mut custom = HashMap::new();
    custom.insert("execution".to_string(), "subagent".to_string());
    meta.metadata = Some(custom);

    registry.register(meta);

    let detail = registry.format_skill_detail("detail-skill").unwrap();
    assert!(detail.contains("detail-skill"));
    assert!(detail.contains("A skill with all the details"));
    assert!(detail.contains("Read"));
    assert!(detail.contains("Apache-2.0"));
    assert!(detail.contains("gpt-4"));
    assert!(detail.contains("subagent"));
}

#[test]
fn format_skill_detail_nonexistent_errors() {
    let registry = SkillRegistry::new();
    assert!(registry.format_skill_detail("ghost").is_err());
}

// ---------------------------------------------------------------------------
// Clear cache
// ---------------------------------------------------------------------------

#[test]
fn clear_cache_forces_reload_from_disk() {
    let dir = TempDir::new().unwrap();
    create_skill_file(dir.path(), "cached", "A cached skill");

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    // Load into cache
    let _ = registry.get_skill("cached").unwrap();

    // Clear cache
    registry.clear_cache();

    // Should still be able to load (from disk again)
    let skill = registry.get_skill("cached").unwrap();
    assert_eq!(skill.name(), "cached");
}

// ---------------------------------------------------------------------------
// Discovery with mixed flat files and subdirectories
// ---------------------------------------------------------------------------

#[test]
fn discover_mixed_flat_and_subdirectory_skills() {
    let dir = TempDir::new().unwrap();

    // Flat file
    create_skill_file(dir.path(), "flat-skill", "A flat skill");

    // Subdirectory
    let sub = dir.path().join("sub-skill");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(
        sub.join("SKILL.md"),
        "---\nname: sub-skill\ndescription: A subdirectory skill\n---\n\nInstructions\n",
    )
    .unwrap();

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    assert_eq!(registry.len(), 2);
    assert!(registry.contains("flat-skill"));
    assert!(registry.contains("sub-skill"));
}

// ---------------------------------------------------------------------------
// Skill with execution mode loaded via registry
// ---------------------------------------------------------------------------

#[test]
fn registry_loads_skill_with_correct_execution_mode() {
    let dir = TempDir::new().unwrap();
    create_skill_file_with_metadata(
        dir.path(),
        "agent-skill",
        "Runs as a subagent",
        "metadata:\n  execution: subagent",
    );

    let mut registry = SkillRegistry::new();
    registry
        .discover_from(&[(dir.path().to_path_buf(), SkillSource::Personal)])
        .unwrap();

    let meta = registry.get_metadata("agent-skill").unwrap();
    assert_eq!(meta.execution_mode(), SkillExecutionMode::Subagent);

    let skill = registry.get_skill("agent-skill").unwrap();
    assert!(skill.runs_as_subagent());
}
