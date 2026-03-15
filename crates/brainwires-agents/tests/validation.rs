//! Integration tests for the validation loop.
//!
//! Tests ValidationConfig construction, disabled validation, file-existence
//! checks, and feedback formatting -- all through the public API.

use brainwires_agents::validation_loop::{
    ValidationCheck, ValidationConfig, ValidationIssue, ValidationResult, ValidationSeverity,
    format_validation_feedback, run_validation,
};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// ValidationConfig builder
// ---------------------------------------------------------------------------

#[test]
fn config_default_has_duplicates_and_syntax_checks() {
    let config = ValidationConfig::default();
    assert!(config.enabled);
    assert_eq!(config.checks.len(), 2);
    assert!(config.working_set_files.is_empty());
}

#[test]
fn config_with_build_appends_build_check() {
    let config = ValidationConfig::default().with_build("typescript");
    assert_eq!(config.checks.len(), 3);
    match &config.checks[2] {
        ValidationCheck::BuildSuccess { build_type } => {
            assert_eq!(build_type, "typescript");
        }
        _ => panic!("Expected BuildSuccess check"),
    }
}

#[test]
fn config_disabled_creates_disabled_config() {
    let config = ValidationConfig::disabled();
    assert!(!config.enabled);
}

#[test]
fn config_with_working_set_files() {
    let config = ValidationConfig::default()
        .with_working_set_files(vec!["src/main.rs".into(), "lib.rs".into()]);
    assert_eq!(config.working_set_files.len(), 2);
}

// ---------------------------------------------------------------------------
// Disabled validation always passes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn disabled_validation_passes_immediately() {
    let config = ValidationConfig::disabled();
    let result = run_validation(&config).await.unwrap();
    assert!(result.passed);
    assert!(result.issues.is_empty());
}

// ---------------------------------------------------------------------------
// File existence check (Bug #5 prevention)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn validation_catches_missing_working_set_files() {
    let dir = tempdir().unwrap();

    // Working set claims a file exists that does NOT
    let config = ValidationConfig {
        checks: vec![], // No duplicates/syntax checks -- just file existence
        working_directory: dir.path().to_str().unwrap().to_string(),
        max_retries: 3,
        enabled: true,
        working_set_files: vec!["nonexistent.rs".into()],
    };

    let result = run_validation(&config).await.unwrap();
    assert!(!result.passed);
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].check, "file_existence");
    assert_eq!(result.issues[0].severity, ValidationSeverity::Error);
    assert!(result.issues[0].message.contains("does not exist"));
}

#[tokio::test]
async fn validation_passes_when_working_set_files_exist() {
    let dir = tempdir().unwrap();

    // Create the file
    let file_path = dir.path().join("exists.txt");
    std::fs::write(&file_path, "content").unwrap();

    let config = ValidationConfig {
        checks: vec![], // No other checks
        working_directory: dir.path().to_str().unwrap().to_string(),
        max_retries: 3,
        enabled: true,
        working_set_files: vec!["exists.txt".into()],
    };

    let result = run_validation(&config).await.unwrap();
    assert!(result.passed);
    assert!(result.issues.is_empty());
}

#[tokio::test]
async fn validation_mixed_existing_and_missing_files() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("real.rs"), "fn main() {}").unwrap();

    let config = ValidationConfig {
        checks: vec![],
        working_directory: dir.path().to_str().unwrap().to_string(),
        max_retries: 3,
        enabled: true,
        working_set_files: vec!["real.rs".into(), "ghost.rs".into()],
    };

    let result = run_validation(&config).await.unwrap();
    assert!(!result.passed);
    assert_eq!(result.issues.len(), 1);
    assert!(result.issues[0].file.as_deref() == Some("ghost.rs"));
}

// ---------------------------------------------------------------------------
// Feedback formatting
// ---------------------------------------------------------------------------

#[test]
fn format_feedback_for_passed_validation() {
    let result = ValidationResult {
        passed: true,
        issues: vec![],
    };
    let feedback = format_validation_feedback(&result);
    assert!(feedback.contains("passed"));
}

#[test]
fn format_feedback_includes_all_issues() {
    let result = ValidationResult {
        passed: false,
        issues: vec![
            ValidationIssue {
                check: "duplicate_check".into(),
                severity: ValidationSeverity::Error,
                message: "Duplicate export 'Foo'".into(),
                file: Some("src/lib.rs".into()),
                line: Some(42),
            },
            ValidationIssue {
                check: "file_existence".into(),
                severity: ValidationSeverity::Error,
                message: "File does not exist".into(),
                file: Some("missing.rs".into()),
                line: None,
            },
        ],
    };

    let feedback = format_validation_feedback(&result);
    assert!(feedback.contains("VALIDATION FAILED"));
    assert!(feedback.contains("src/lib.rs:42:"));
    assert!(feedback.contains("Duplicate export 'Foo'"));
    assert!(feedback.contains("missing.rs:"));
    assert!(feedback.contains("File does not exist"));
    assert!(feedback.contains("MUST fix ALL"));
}

// ---------------------------------------------------------------------------
// Empty working set with no checks passes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_working_set_no_checks_passes() {
    let dir = tempdir().unwrap();

    let config = ValidationConfig {
        checks: vec![],
        working_directory: dir.path().to_str().unwrap().to_string(),
        max_retries: 3,
        enabled: true,
        working_set_files: vec![],
    };

    let result = run_validation(&config).await.unwrap();
    assert!(result.passed);
}
