//! Auth module — thin adapter over brainwires-network auth
//!
//! Re-exports bridge types and provides CLI-specific wrappers that preserve
//! the static API used by 20+ call sites throughout the codebase.

use anyhow::{Result, anyhow};
use zeroize::Zeroizing;

use crate::config::constants;
use crate::utils::paths::PlatformPaths;

// ── Private imports from bridge ──────────────────────────────────────────

use brainwires::agent_network::auth::AuthClient as BridgeAuthClient;
use brainwires::agent_network::auth::SessionManager as BridgeSessionManager;
use brainwires::agent_network::auth::keyring::KeyringKeyStore;
use brainwires::agent_network::auth::types::*;
use brainwires::agent_network::traits::KeyStore;

// ── CLI AuthClient wrapper ──────────────────────────────────────────────

/// Authentication client for interacting with Brainwires Studio backend.
///
/// Wraps bridge `AuthClient` with CLI-specific defaults from `config::constants`.
pub struct AuthClient {
    bridge: BridgeAuthClient,
    pub backend_url: String,
}

impl AuthClient {
    /// Create a new authentication client.
    pub fn new(backend_url: String) -> Self {
        let bridge = BridgeAuthClient::new(
            backend_url.clone(),
            constants::API_CLI_AUTH_ENDPOINT.to_string(),
            constants::API_KEY_PATTERN,
        );
        Self {
            bridge,
            backend_url,
        }
    }

    /// Validate API key format (static — creates temp bridge client).
    pub fn validate_api_key_format(api_key: &str) -> Result<()> {
        let tmp = BridgeAuthClient::new(String::new(), String::new(), constants::API_KEY_PATTERN);
        tmp.validate_api_key_format(api_key)
    }

    /// Authenticate with API key — auto-saves session + keyring.
    pub async fn authenticate(&self, api_key: &str) -> Result<AuthSession> {
        let response = self.bridge.authenticate(api_key).await?;
        let session =
            SessionManager::create_session(response, self.backend_url.clone(), api_key.to_string());
        SessionManager::save(&session, Some(api_key))?;
        Ok(session)
    }

    /// Validate current session (local expiration check).
    pub async fn validate_session(&self) -> Result<bool> {
        let session = SessionManager::get_session()?.ok_or_else(|| anyhow!("No active session"))?;
        Ok(!session.is_expired())
    }

    /// Refresh provider keys (re-saves session to update timestamp).
    pub async fn refresh_provider_keys(&self) -> Result<()> {
        let session = SessionManager::get_session()?.ok_or_else(|| anyhow!("No active session"))?;
        SessionManager::save(&session, None)?;
        Ok(())
    }

    /// Logout (clear local session and keyring).
    pub fn logout() -> Result<()> {
        SessionManager::delete()
    }
}

// ── CLI SessionManager wrapper ──────────────────────────────────────────

/// Session manager — static wrapper preserving existing API.
///
/// Internally creates bridge `BridgeSessionManager` instances on each call,
/// injecting `PlatformPaths::session_file()` and `KeyringKeyStore`.
pub struct SessionManager;

impl SessionManager {
    fn bridge_manager() -> Result<BridgeSessionManager> {
        let session_file = PlatformPaths::session_file()?;
        let key_store: Option<Box<dyn KeyStore>> = Some(Box::new(KeyringKeyStore::new()));
        Ok(BridgeSessionManager::new(session_file, key_store))
    }

    pub fn load() -> Result<Option<AuthSession>> {
        Self::bridge_manager()?.load()
    }

    pub fn save(session: &AuthSession, api_key: Option<&str>) -> Result<()> {
        Self::bridge_manager()?.save(session, api_key)
    }

    pub fn delete() -> Result<()> {
        Self::bridge_manager()?.delete()
    }

    pub fn is_authenticated() -> Result<bool> {
        Self::bridge_manager()?.is_authenticated()
    }

    pub fn get_session() -> Result<Option<AuthSession>> {
        Self::bridge_manager()?.get_session()
    }

    pub fn get_api_key() -> Result<Option<Zeroizing<String>>> {
        Self::bridge_manager()?.get_api_key()
    }

    pub fn create_session(response: AuthResponse, backend: String, api_key: String) -> AuthSession {
        BridgeSessionManager::create_session(response, backend, api_key)
    }

    pub fn migrate_to_keyring() -> Result<bool> {
        Self::bridge_manager()?.migrate_to_key_store()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_session() -> AuthSession {
        AuthSession {
            user: UserProfile {
                user_id: "test-user-id".to_string(),
                username: "testuser".to_string(),
                display_name: "Test User".to_string(),
                role: "basic".to_string(),
            },
            supabase: SupabaseConfig {
                url: "https://test.supabase.co".to_string(),
                anon_key: "test-anon-key".to_string(),
            },
            key_name: "test-key".to_string(),
            api_key: "bw_dev_12345678901234567890123456789012".to_string(),
            backend: "https://brainwires.studio".to_string(),
            authenticated_at: Utc::now(),
        }
    }

    // ── API key validation ──────────────────────────────────────────

    #[test]
    fn test_validate_api_key_format() {
        assert!(
            AuthClient::validate_api_key_format("bw_dev_12345678901234567890123456789012").is_ok()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_prod_abcdefghijklmnopqrstuvwxyz123456").is_ok()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_test_00000000000000000000000000000000").is_ok()
        );

        assert!(AuthClient::validate_api_key_format("invalid").is_err());
        assert!(
            AuthClient::validate_api_key_format("bw_invalid_12345678901234567890123456789012")
                .is_err()
        );
        assert!(AuthClient::validate_api_key_format("bw_dev_short").is_err());
        assert!(
            AuthClient::validate_api_key_format("bw_dev_UPPERCASE0000000000000000000000").is_err()
        );
    }

    #[test]
    fn test_validate_api_key_error_message() {
        let result = AuthClient::validate_api_key_format("invalid_key");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid API key format")
        );
    }

    #[test]
    fn test_validate_api_key_edge_cases() {
        assert!(AuthClient::validate_api_key_format("").is_err());
        assert!(AuthClient::validate_api_key_format("   ").is_err());
        assert!(AuthClient::validate_api_key_format("bw_dev_123").is_err());
        assert!(
            AuthClient::validate_api_key_format("bw_dev_123456789012345678901234567890123")
                .is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("dev_12345678901234567890123456789012").is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("bw__12345678901234567890123456789012").is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_dev_1234567890!@#$%^&*()1234567890").is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_dev_12345678901234567890 123456789012")
                .is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_dev_12345678901234567890\n123456789012")
                .is_err()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_Dev_12345678901234567890123456789012").is_err()
        );
    }

    #[test]
    fn test_validate_api_key_valid_variants() {
        assert!(
            AuthClient::validate_api_key_format("bw_dev_abcdef1234567890abcdef1234567890").is_ok()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_prod_12345678901234567890123456789012").is_ok()
        );
        assert!(
            AuthClient::validate_api_key_format("bw_test_abcdefghijklmnopqrstuvwxyzabcdef").is_ok()
        );
    }

    // ── AuthClient construction ─────────────────────────────────────

    #[test]
    fn test_auth_client_new() {
        let client = AuthClient::new("https://test.example.com".to_string());
        assert_eq!(client.backend_url, "https://test.example.com");
    }

    #[test]
    fn test_auth_client_new_empty_url() {
        let client = AuthClient::new("".to_string());
        assert_eq!(client.backend_url, "");
    }

    #[test]
    fn test_auth_client_new_with_path() {
        let client = AuthClient::new("https://api.example.com/v1".to_string());
        assert_eq!(client.backend_url, "https://api.example.com/v1");
    }

    // ── Session unit tests ──────────────────────────────────────────

    #[test]
    fn test_session_never_expires() {
        let session = create_test_session();
        assert!(!session.is_expired());
    }

    #[test]
    fn test_create_session() {
        let response = AuthResponse {
            user: UserProfile {
                user_id: "user123".to_string(),
                username: "john".to_string(),
                display_name: "John Doe".to_string(),
                role: "admin".to_string(),
            },
            supabase: SupabaseConfig {
                url: "https://test.supabase.co".to_string(),
                anon_key: "anon-test".to_string(),
            },
            key_name: "my_key".to_string(),
        };

        let session = SessionManager::create_session(
            response,
            "https://brainwires.studio".to_string(),
            "bw_dev_12345678901234567890123456789012".to_string(),
        );

        assert_eq!(session.user.user_id, "user123");
        assert_eq!(session.user.username, "john");
        assert_eq!(session.user.display_name, "John Doe");
        assert_eq!(session.key_name, "my_key");
        assert_eq!(session.backend, "https://brainwires.studio");
        assert!(!session.is_expired());
    }

    #[test]
    fn test_old_session_does_not_expire() {
        let mut session = create_test_session();
        session.authenticated_at = Utc::now() - chrono::Duration::days(365);
        assert!(!session.is_expired(), "Sessions should never expire");
    }

    #[test]
    fn test_session_serialization() {
        let session = create_test_session();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: AuthSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user.user_id, session.user.user_id);
        assert_eq!(deserialized.key_name, session.key_name);
        assert_eq!(deserialized.backend, session.backend);
    }

    // ── File I/O tests (using bridge SessionManager directly) ───────

    mod file_io {
        use super::*;
        use tempfile::TempDir;

        fn temp_bridge_manager() -> (TempDir, BridgeSessionManager) {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let session_file = dir.path().join("session.json");
            let mgr = BridgeSessionManager::new(session_file, None);
            (dir, mgr)
        }

        #[test]
        fn test_save_and_load_session() {
            let (_dir, mgr) = temp_bridge_manager();
            let session = create_test_session();
            mgr.save(&session, None).unwrap();

            let loaded = mgr.load().unwrap();
            assert!(loaded.is_some());
        }

        #[test]
        fn test_load_nonexistent_session() {
            let (_dir, mgr) = temp_bridge_manager();
            assert!(mgr.load().unwrap().is_none());
        }

        #[test]
        fn test_delete_session() {
            let (_dir, mgr) = temp_bridge_manager();
            let session = create_test_session();
            mgr.save(&session, None).unwrap();
            mgr.delete().unwrap();
            assert!(mgr.load().unwrap().is_none());
        }

        #[test]
        fn test_delete_nonexistent_session() {
            let (_dir, mgr) = temp_bridge_manager();
            assert!(mgr.delete().is_ok());
        }

        #[test]
        fn test_is_authenticated_with_valid_session() {
            let (_dir, mgr) = temp_bridge_manager();
            let session = create_test_session();
            mgr.save(&session, None).unwrap();
            assert!(mgr.is_authenticated().unwrap());
        }

        #[test]
        fn test_is_authenticated_without_session() {
            let (_dir, mgr) = temp_bridge_manager();
            assert!(!mgr.is_authenticated().unwrap());
        }

        #[test]
        fn test_is_authenticated_with_old_session() {
            let (_dir, mgr) = temp_bridge_manager();
            let mut session = create_test_session();
            session.authenticated_at = Utc::now() - chrono::Duration::days(365);
            mgr.save(&session, None).unwrap();
            assert!(mgr.is_authenticated().unwrap());
        }

        #[test]
        fn test_get_session_valid() {
            let (_dir, mgr) = temp_bridge_manager();
            let session = create_test_session();
            mgr.save(&session, None).unwrap();
            let retrieved = mgr.get_session().unwrap().unwrap();
            assert_eq!(retrieved.user.user_id, session.user.user_id);
        }

        #[test]
        fn test_get_session_none() {
            let (_dir, mgr) = temp_bridge_manager();
            assert!(mgr.get_session().unwrap().is_none());
        }

        #[test]
        fn test_get_session_old_still_valid() {
            let (_dir, mgr) = temp_bridge_manager();
            let mut session = create_test_session();
            session.authenticated_at = Utc::now() - chrono::Duration::days(365);
            mgr.save(&session, None).unwrap();
            assert!(mgr.get_session().unwrap().is_some());
        }
    }
}
