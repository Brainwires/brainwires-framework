// Integration tests for real authentication flow
// These tests require network access and a valid test API key
// Run with: cargo test --test auth_integration_test -- --ignored
mod common;

use brainwires_cli::auth::{AuthClient, SessionManager};
use std::sync::Mutex;
use tempfile::TempDir;

const TEST_API_KEY: &str = "bw_dev_d8c0dd65a0d84d4b10f297fe792a4ee5";
const DEV_BACKEND: &str = "https://dev.brainwires.net";

// Mutex to prevent parallel tests from interfering with each other's env vars
static TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Helper struct that sets up isolated session storage for the test duration
struct TestEnv {
    _temp_dir: TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
    original_xdg: Option<String>,
}

impl TestEnv {
    fn new() -> Self {
        let guard = TEST_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = temp_dir.path().join(".local/share");
        std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");

        // Save original env var
        let original_xdg = std::env::var("XDG_DATA_HOME").ok();

        // Set temp data home
        // SAFETY: We hold a mutex to prevent concurrent env var modifications in tests
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &data_dir);
        }

        Self {
            _temp_dir: temp_dir,
            _guard: guard,
            original_xdg,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original env var
        // SAFETY: We hold a mutex to prevent concurrent env var modifications in tests
        unsafe {
            match &self.original_xdg {
                Some(val) => std::env::set_var("XDG_DATA_HOME", val),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test --test auth_integration_test -- --ignored
async fn test_real_authentication() {
    let _env = TestEnv::new();
    let client = AuthClient::new(DEV_BACKEND.to_string());

    let result = client.authenticate(TEST_API_KEY).await;

    match result {
        Ok(session) => {
            println!("✅ Authentication successful!");
            println!("User: {}", session.user.username);
            println!("Display name: {}", session.user.display_name);
            println!("API key name: {}", session.key_name);
            println!("Backend: {}", session.backend);
            assert!(!session.user.user_id.is_empty());
            assert!(!session.is_expired());
        }
        Err(e) => {
            eprintln!("❌ Authentication failed: {}", e);
            panic!("Authentication should succeed with valid API key");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_authentication_invalid_key() {
    let _env = TestEnv::new();
    let client = AuthClient::new(DEV_BACKEND.to_string());

    let result = client
        .authenticate("bw_dev_00000000000000000000000000000000")
        .await;

    assert!(
        result.is_err(),
        "Authentication should fail with invalid key"
    );
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Authentication failed")
            || error.contains("401")
            || error.contains("Unauthorized"),
        "Error should indicate auth failure: {}",
        error
    );
}

#[tokio::test]
#[ignore]
async fn test_authentication_malformed_key() {
    let _env = TestEnv::new();
    let client = AuthClient::new(DEV_BACKEND.to_string());

    let result = client.authenticate("not-a-valid-api-key").await;

    assert!(
        result.is_err(),
        "Authentication should fail with malformed key"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid API key format")
    );
}

#[tokio::test]
#[ignore]
async fn test_session_persistence() {
    let _env = TestEnv::new();
    let client = AuthClient::new(DEV_BACKEND.to_string());

    // Authenticate
    let session = client
        .authenticate(TEST_API_KEY)
        .await
        .expect("Authentication should succeed");

    // Session should be saved
    let loaded_session = SessionManager::get_session()
        .expect("Should load session")
        .expect("Session should exist");

    assert_eq!(loaded_session.user.user_id, session.user.user_id);
    assert_eq!(loaded_session.backend, session.backend);

    // Check authentication status
    let is_authed = SessionManager::is_authenticated().expect("Should check auth status");
    assert!(is_authed, "Should be authenticated after successful login");

    // Cleanup is automatic when TestEnv drops
}

#[tokio::test]
#[ignore]
async fn test_session_fields() {
    let _env = TestEnv::new();
    let client = AuthClient::new(DEV_BACKEND.to_string());

    let session = client
        .authenticate(TEST_API_KEY)
        .await
        .expect("Authentication should succeed");

    // Check session fields
    println!("Session fields:");
    println!("  User ID: {}", session.user.user_id);
    println!("  Username: {}", session.user.username);
    println!("  API key name: {}", session.key_name);
    println!("  Backend: {}", session.backend);

    // Verify all required fields are populated
    assert!(
        !session.user.user_id.is_empty(),
        "User ID should not be empty"
    );
    assert!(
        !session.user.username.is_empty(),
        "Username should not be empty"
    );
    assert!(!session.key_name.is_empty(), "Key name should not be empty");
    assert!(!session.api_key.is_empty(), "API key should not be empty");
    assert!(
        !session.backend.is_empty(),
        "Backend URL should not be empty"
    );

    // Cleanup is automatic when TestEnv drops
}
