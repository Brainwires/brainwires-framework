//! Integration tests for the unified Executor API.
//!
//! Tests the public `Executor` struct and helper functions
//! that are not language-specific.

use brainwires_code_interpreters::{
    ExecutionLimits, ExecutionRequest, Executor, Language, is_language_supported,
    supported_languages,
};

// ---------------------------------------------------------------------------
// Executor construction
// ---------------------------------------------------------------------------

#[test]
fn executor_default_and_new_are_equivalent() {
    let a = Executor::new();
    let b = Executor::default();
    assert_eq!(a.limits().max_timeout_ms, b.limits().max_timeout_ms,);
    assert_eq!(a.limits().max_memory_mb, b.limits().max_memory_mb,);
}

#[test]
fn executor_with_strict_limits() {
    let strict = ExecutionLimits::strict();
    let executor = Executor::with_limits(strict.clone());
    assert_eq!(executor.limits().max_timeout_ms, 5_000);
    assert_eq!(executor.limits().max_memory_mb, 64);
    assert_eq!(executor.limits().max_operations, 100_000);
}

#[test]
fn executor_with_relaxed_limits() {
    let relaxed = ExecutionLimits::relaxed();
    let executor = Executor::with_limits(relaxed.clone());
    assert_eq!(executor.limits().max_timeout_ms, 120_000);
    assert_eq!(executor.limits().max_memory_mb, 512);
}

// ---------------------------------------------------------------------------
// Supported-languages helpers
// ---------------------------------------------------------------------------

#[test]
fn supported_languages_reflects_features() {
    let langs = supported_languages();
    // At least the default features (rhai, lua) should be present
    // unless compiled with --no-default-features.
    assert!(!langs.is_empty(), "At least one language must be enabled");
}

#[test]
fn is_language_supported_matches_supported_list() {
    let langs = supported_languages();
    for lang in &langs {
        assert!(
            is_language_supported(*lang),
            "{:?} reported supported but is_language_supported returned false",
            lang,
        );
    }
}

#[test]
fn executor_supported_languages_delegates() {
    let executor = Executor::new();
    assert_eq!(executor.supported_languages(), supported_languages());
}

// ---------------------------------------------------------------------------
// Language enum
// ---------------------------------------------------------------------------

#[test]
fn language_parse_roundtrip() {
    let cases = [
        ("rhai", Language::Rhai),
        ("lua", Language::Lua),
        ("javascript", Language::JavaScript),
        ("js", Language::JavaScript),
        ("python", Language::Python),
        ("py", Language::Python),
    ];
    for (input, expected) in &cases {
        assert_eq!(Language::parse(input), Some(*expected), "input: {}", input);
    }
}

#[test]
fn language_parse_case_insensitive() {
    assert_eq!(Language::parse("RHAI"), Some(Language::Rhai));
    assert_eq!(Language::parse("LUA"), Some(Language::Lua));
    assert_eq!(Language::parse("Python"), Some(Language::Python));
    assert_eq!(Language::parse("JavaScript"), Some(Language::JavaScript));
}

#[test]
fn language_parse_unknown_returns_none() {
    assert_eq!(Language::parse("ruby"), None);
    assert_eq!(Language::parse(""), None);
    assert_eq!(Language::parse("c++"), None);
}

#[test]
fn language_as_str_and_display() {
    assert_eq!(Language::Rhai.as_str(), "rhai");
    assert_eq!(Language::Lua.as_str(), "lua");
    assert_eq!(Language::JavaScript.as_str(), "javascript");
    assert_eq!(Language::Python.as_str(), "python");
    assert_eq!(format!("{}", Language::Rhai), "rhai");
}

#[test]
fn language_extension() {
    assert_eq!(Language::Rhai.extension(), "rhai");
    assert_eq!(Language::Lua.extension(), "lua");
    assert_eq!(Language::JavaScript.extension(), "js");
    assert_eq!(Language::Python.extension(), "py");
}

// ---------------------------------------------------------------------------
// Unsupported-language handling via Executor
// ---------------------------------------------------------------------------

#[test]
fn execute_str_unsupported_language_returns_error() {
    let executor = Executor::new();
    let result = executor.execute_str("cobol", "DISPLAY 'HELLO'");
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("not supported"));
}

#[test]
fn execute_str_empty_language_returns_error() {
    let executor = Executor::new();
    let result = executor.execute_str("", "code");
    assert!(!result.success);
    assert!(result.error.is_some());
}

// ---------------------------------------------------------------------------
// ExecutionRequest / ExecutionResult defaults
// ---------------------------------------------------------------------------

#[test]
fn execution_request_default_values() {
    let req = ExecutionRequest::default();
    assert_eq!(req.timeout_ms, 30_000);
    assert_eq!(req.memory_limit_mb, 256);
    assert!(req.stdin.is_none());
    assert!(req.context.is_none());
    assert!(req.limits.is_none());
    assert!(req.code.is_empty());
}

#[test]
fn execution_limits_default_vs_strict_vs_relaxed() {
    let def = ExecutionLimits::default();
    let strict = ExecutionLimits::strict();
    let relaxed = ExecutionLimits::relaxed();

    assert!(strict.max_timeout_ms < def.max_timeout_ms);
    assert!(def.max_timeout_ms < relaxed.max_timeout_ms);

    assert!(strict.max_memory_mb < def.max_memory_mb);
    assert!(def.max_memory_mb < relaxed.max_memory_mb);

    assert!(strict.max_operations < def.max_operations);
    assert!(def.max_operations < relaxed.max_operations);
}

// ---------------------------------------------------------------------------
// SandboxProfile
// ---------------------------------------------------------------------------

#[test]
fn sandbox_profile_allowed_modules() {
    use brainwires_code_interpreters::SandboxProfile;

    let minimal = SandboxProfile::Minimal.allowed_modules();
    let standard = SandboxProfile::Standard.allowed_modules();
    let extended = SandboxProfile::Extended.allowed_modules();

    assert!(minimal.contains(&"math"));
    assert!(!minimal.contains(&"print"));

    assert!(standard.contains(&"print"));
    assert!(standard.contains(&"json"));

    assert!(extended.contains(&"regex"));
    assert!(extended.contains(&"datetime"));
    assert!(extended.len() > standard.len());
}

// ---------------------------------------------------------------------------
// ExecutionError -> ExecutionResult conversion
// ---------------------------------------------------------------------------

#[test]
fn execution_error_to_result() {
    use brainwires_code_interpreters::ExecutionError;

    let err = ExecutionError::Timeout(5000);
    let result = err.to_result(42);
    assert!(!result.success);
    assert_eq!(result.timing_ms, 42);
    assert!(result.error.as_deref().unwrap().contains("5000"));

    let err2 = ExecutionError::SyntaxError("unexpected token".into());
    let result2 = err2.to_result(10);
    assert!(
        result2
            .error
            .as_deref()
            .unwrap()
            .contains("unexpected token")
    );
}

// ---------------------------------------------------------------------------
// ExecutionResult constructors
// ---------------------------------------------------------------------------

#[test]
fn execution_result_success_constructor() {
    use brainwires_code_interpreters::ExecutionResult;

    let r = ExecutionResult::success("hello".into(), Some(serde_json::json!(42)), 100);
    assert!(r.success);
    assert_eq!(r.stdout, "hello");
    assert!(r.error.is_none());
    assert_eq!(r.timing_ms, 100);
    assert_eq!(r.result, Some(serde_json::json!(42)));
}

#[test]
fn execution_result_error_with_output_constructor() {
    use brainwires_code_interpreters::ExecutionResult;

    let r = ExecutionResult::error_with_output(
        "boom".into(),
        "partial stdout".into(),
        "partial stderr".into(),
        55,
    );
    assert!(!r.success);
    assert_eq!(r.stdout, "partial stdout");
    assert_eq!(r.stderr, "partial stderr");
    assert_eq!(r.error.as_deref(), Some("boom"));
    assert_eq!(r.timing_ms, 55);
}
