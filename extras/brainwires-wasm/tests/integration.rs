//! Integration tests for brainwires-wasm public API (native target).
//!
//! These tests exercise the Rust-side logic of all `#[wasm_bindgen]` functions
//! without requiring wasm-pack or a browser/Node runtime.

use brainwires_wasm::{serialize_history, validate_message, validate_tool, version};

// ── version() ───────────────────────────────────────────────────────────

#[test]
fn version_returns_non_empty_string() {
    let v = version();
    assert!(!v.is_empty(), "version() must not be empty");
}

#[test]
fn version_is_valid_semver() {
    let v = version();
    let parts: Vec<&str> = v.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "version should have 3 semver components: {v}"
    );
    for part in &parts {
        part.parse::<u32>()
            .unwrap_or_else(|_| panic!("'{part}' is not a valid semver component in '{v}'"));
    }
}

// ── validate_message() ─────────────────────────────────────────────────

#[test]
fn validate_message_valid_user_text() {
    let json = r#"{"role":"user","content":"Hello"}"#;
    let result = validate_message(json);
    assert!(
        result.is_ok(),
        "valid user message should succeed: {result:?}"
    );
    let out = result.unwrap();
    assert!(out.contains("user"), "output should contain role");
    assert!(out.contains("Hello"), "output should contain content");
}

#[test]
fn validate_message_valid_assistant() {
    let json = r#"{"role":"assistant","content":"Hi there"}"#;
    let result = validate_message(json);
    assert!(
        result.is_ok(),
        "valid assistant message should succeed: {result:?}"
    );
}

#[test]
fn validate_message_valid_system() {
    let json = r#"{"role":"system","content":"You are helpful."}"#;
    let result = validate_message(json);
    assert!(
        result.is_ok(),
        "valid system message should succeed: {result:?}"
    );
}

#[test]
fn validate_message_with_optional_name() {
    let json = r#"{"role":"user","content":"Hello","name":"alice"}"#;
    let result = validate_message(json);
    assert!(
        result.is_ok(),
        "message with name field should succeed: {result:?}"
    );
    let out = result.unwrap();
    assert!(out.contains("alice"), "output should preserve name field");
}

#[test]
fn validate_message_with_metadata() {
    let json = r#"{"role":"user","content":"Hello","metadata":{"key":"value"}}"#;
    let result = validate_message(json);
    assert!(
        result.is_ok(),
        "message with metadata should succeed: {result:?}"
    );
}

#[test]
fn validate_message_invalid_role() {
    let json = r#"{"role":"unknown_role","content":"Hello"}"#;
    let result = validate_message(json);
    assert!(result.is_err(), "invalid role should fail");
}

#[test]
fn validate_message_missing_role() {
    let json = r#"{"content":"Hello"}"#;
    let result = validate_message(json);
    assert!(result.is_err(), "missing role should fail");
}

#[test]
fn validate_message_missing_content() {
    let json = r#"{"role":"user"}"#;
    let result = validate_message(json);
    assert!(result.is_err(), "missing content should fail");
}

#[test]
fn validate_message_empty_string() {
    let result = validate_message("");
    assert!(result.is_err(), "empty string should fail");
}

#[test]
fn validate_message_malformed_json() {
    let result = validate_message("{not valid json}");
    assert!(result.is_err(), "malformed JSON should fail");
}

#[test]
fn validate_message_null_input() {
    let result = validate_message("null");
    assert!(result.is_err(), "null should fail");
}

#[test]
fn validate_message_array_input() {
    let result = validate_message("[]");
    assert!(result.is_err(), "array should fail for single message");
}

// ── validate_tool() ─────────────────────────────────────────────────────

#[test]
fn validate_tool_valid_minimal() {
    // Tool fields all have #[serde(default)], so an empty object should work
    let json = r#"{}"#;
    let result = validate_tool(json);
    assert!(
        result.is_ok(),
        "empty object (all defaults) should succeed: {result:?}"
    );
}

#[test]
fn validate_tool_valid_full() {
    let json = r#"{
        "name": "calculator",
        "description": "Performs arithmetic",
        "input_schema": {"type": "object", "properties": {"expr": {"type": "string"}}},
        "requires_approval": false,
        "defer_loading": false
    }"#;
    let result = validate_tool(json);
    assert!(
        result.is_ok(),
        "full tool definition should succeed: {result:?}"
    );
    let out = result.unwrap();
    assert!(
        out.contains("calculator"),
        "output should contain tool name"
    );
    assert!(
        out.contains("Performs arithmetic"),
        "output should contain description"
    );
}

#[test]
fn validate_tool_with_name_only() {
    let json = r#"{"name": "my_tool"}"#;
    let result = validate_tool(json);
    assert!(
        result.is_ok(),
        "tool with only name should succeed: {result:?}"
    );
}

#[test]
fn validate_tool_malformed_json() {
    let result = validate_tool("{broken");
    assert!(result.is_err(), "malformed JSON should fail");
}

#[test]
fn validate_tool_empty_string() {
    let result = validate_tool("");
    assert!(result.is_err(), "empty string should fail");
}

#[test]
fn validate_tool_null_input() {
    let result = validate_tool("null");
    assert!(result.is_err(), "null should fail");
}

#[test]
fn validate_tool_number_input() {
    let result = validate_tool("42");
    assert!(result.is_err(), "number literal should fail for tool");
}

#[test]
fn validate_tool_roundtrip_preserves_fields() {
    let json = r#"{"name":"read_file","description":"Reads a file","input_schema":{"type":"object"},"requires_approval":true}"#;
    let result = validate_tool(json).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["name"], "read_file");
    assert_eq!(parsed["description"], "Reads a file");
    assert_eq!(parsed["requires_approval"], true);
}

// ── serialize_history() ─────────────────────────────────────────────────

#[test]
fn serialize_history_valid_array() {
    let json = r#"[
        {"role":"user","content":"What is 2+2?"},
        {"role":"assistant","content":"4"}
    ]"#;
    let result = serialize_history(json);
    assert!(
        result.is_ok(),
        "valid message array should succeed: {result:?}"
    );
    let out = result.unwrap();
    // Output should be valid JSON
    let _parsed: serde_json::Value =
        serde_json::from_str(&out).expect("serialize_history output should be valid JSON");
}

#[test]
fn serialize_history_empty_array() {
    let result = serialize_history("[]");
    assert!(result.is_ok(), "empty array should succeed: {result:?}");
    let out = result.unwrap();
    let _parsed: serde_json::Value =
        serde_json::from_str(&out).expect("output should be valid JSON");
}

#[test]
fn serialize_history_single_message() {
    let json = r#"[{"role":"user","content":"Hello"}]"#;
    let result = serialize_history(json);
    assert!(
        result.is_ok(),
        "single message array should succeed: {result:?}"
    );
}

#[test]
fn serialize_history_multiple_roles() {
    let json = r#"[
        {"role":"system","content":"You are helpful."},
        {"role":"user","content":"Hi"},
        {"role":"assistant","content":"Hello!"},
        {"role":"user","content":"How are you?"}
    ]"#;
    let result = serialize_history(json);
    assert!(
        result.is_ok(),
        "multi-role conversation should succeed: {result:?}"
    );
}

#[test]
fn serialize_history_malformed_json() {
    let result = serialize_history("{not an array}");
    assert!(result.is_err(), "malformed JSON should fail");
}

#[test]
fn serialize_history_not_array() {
    let result = serialize_history(r#"{"role":"user","content":"Hi"}"#);
    assert!(result.is_err(), "non-array input should fail");
}

#[test]
fn serialize_history_empty_string() {
    let result = serialize_history("");
    assert!(result.is_err(), "empty string should fail");
}

#[test]
fn serialize_history_invalid_message_in_array() {
    let json = r#"[{"role":"user","content":"Hi"},{"invalid":true}]"#;
    let result = serialize_history(json);
    assert!(result.is_err(), "array with invalid message should fail");
}
