// Common utilities for integration tests
use brainwires_cli::types::agent::PermissionMode;
use brainwires_cli::types::message::{Message, MessageContent, Role};
use brainwires_cli::types::provider::ProviderType;
use std::env;
use tempfile::TempDir;

/// Get test API key or panic with helpful message
pub fn get_test_api_key(_provider: ProviderType) -> String {
    env::var("TEST_BRAINWIRES_API_KEY").unwrap_or_else(|_| {
        eprintln!("Skipping test - TEST_BRAINWIRES_API_KEY not set");
        "test-key-placeholder".to_string()
    })
}

/// Create a temporary directory for tests
pub fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

/// Create a test message
pub fn create_test_message(role: Role, content: &str) -> Message {
    Message {
        role,
        content: MessageContent::Text(content.to_string()),
        name: None,
        metadata: None,
    }
}

/// Check if we should skip integration tests (no API keys set)
pub fn should_skip_integration_tests() -> bool {
    env::var("TEST_BRAINWIRES_API_KEY").is_err()
}

/// Get default permission mode for tests
pub fn default_permission_mode() -> PermissionMode {
    PermissionMode::ReadOnly
}
