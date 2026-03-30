//! Secret Redaction Utilities
//!
//! Provides utilities to redact sensitive information from logs, error messages,
//! and other output. This helps prevent accidental exposure of secrets.

use std::borrow::Cow;
use lazy_static::lazy_static;
use regex::Regex;

/// A compiled pattern for matching and redacting secrets
struct SecretPattern {
    #[allow(dead_code)]
    name: &'static str,
    regex: Regex,
    capture_group: usize,
    replacement: &'static str,
}

impl SecretPattern {
    fn new(name: &'static str, pattern: &str, capture_group: usize, replacement: &'static str) -> Self {
        Self {
            name,
            regex: Regex::new(pattern).expect("Invalid regex pattern"),
            capture_group,
            replacement,
        }
    }

    /// Redact matches from the input string
    fn redact<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if !self.regex.is_match(input) {
            return Cow::Borrowed(input);
        }

        let result = self.regex.replace_all(input, |caps: &regex::Captures| {
            // Build the replacement, preserving prefix and suffix
            let mut result = String::new();
            let full_match = caps.get(0).unwrap().as_str();

            if let Some(secret) = caps.get(self.capture_group) {
                // Find the prefix (everything before the secret in the match)
                let prefix_end = secret.start() - caps.get(0).unwrap().start();
                let prefix = &full_match[..prefix_end];

                // Find the suffix (everything after the secret in the match)
                let suffix_start = secret.end() - caps.get(0).unwrap().start();
                let suffix = &full_match[suffix_start..];

                result.push_str(prefix);
                result.push_str(self.replacement);
                result.push_str(suffix);
            } else {
                result.push_str(self.replacement);
            }
            result
        });

        Cow::Owned(result.into_owned())
    }
}

fn build_secret_patterns() -> Vec<SecretPattern> {
    vec![
        // API Keys with common prefixes
        SecretPattern::new("api_key", r#"(?i)(api[_-]?key|apikey)[=:\s]+['"]?([a-zA-Z0-9_-]{20,})['"]?"#, 2, "***API_KEY***"),
        SecretPattern::new("bearer_token", r"(?i)(bearer)\s+([a-zA-Z0-9_.-]{20,})", 2, "***TOKEN***"),
        SecretPattern::new("authorization", r#"(?i)(authorization)[=:\s]+['"]?([a-zA-Z0-9_.-]{20,})['"]?"#, 2, "***AUTH***"),

        // OpenAI/Anthropic/Other AI provider keys
        SecretPattern::new("openai_key", r"(sk-[a-zA-Z0-9]{48,})", 1, "***OPENAI_KEY***"),
        SecretPattern::new("anthropic_key", r"(sk-ant-[a-zA-Z0-9-]{40,})", 1, "***ANTHROPIC_KEY***"),
        SecretPattern::new("google_ai_key", r"(AIza[a-zA-Z0-9_-]{35})", 1, "***GOOGLE_KEY***"),

        // Supabase keys
        SecretPattern::new("supabase_anon", r"(eyJ[a-zA-Z0-9_-]{100,}\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+)", 1, "***SUPABASE_JWT***"),
        SecretPattern::new("supabase_service", r"(sbp_[a-zA-Z0-9]{40,})", 1, "***SUPABASE_KEY***"),

        // Session tokens (hex-encoded)
        SecretPattern::new("session_token", r#"(?i)(session[_-]?token|token)[=:\s]+['"]?([a-f0-9]{64,})['"]?"#, 2, "***SESSION***"),

        // Password patterns - match "password": "value" or password="value" etc.
        SecretPattern::new("password_json", r#"(?i)"(password|passwd|pwd)":\s*"([^"]+)""#, 2, "***PASSWORD***"),
        SecretPattern::new("password_quoted", r#"(?i)(password|passwd|pwd)[=:\s]+["']([^"']+)["']"#, 2, "***PASSWORD***"),
        SecretPattern::new("password_plain", r#"(?i)(password|passwd|pwd)[=:\s]+([^\s&,;"']+)"#, 2, "***PASSWORD***"),

        // Connection strings with embedded credentials
        SecretPattern::new("postgres_uri", r"(postgres(?:ql)?://[^:]+:)([^@]+)(@)", 2, "***PASSWORD***"),
        SecretPattern::new("redis_uri", r"(redis://[^:]*:)([^@]+)(@)", 2, "***PASSWORD***"),
        SecretPattern::new("mongodb_uri", r"(mongodb(?:\+srv)?://[^:]+:)([^@]+)(@)", 2, "***PASSWORD***"),

        // AWS credentials
        SecretPattern::new("aws_access_key", r"(AKIA[A-Z0-9]{16})", 1, "***AWS_KEY***"),
        SecretPattern::new("aws_secret_key", r#"(?i)(aws[_-]?secret[_-]?access[_-]?key)[=:\s]+['"]?([a-zA-Z0-9/+=]{40})['"]?"#, 2, "***AWS_SECRET***"),

        // GitHub tokens
        SecretPattern::new("github_token", r"(ghp_[a-zA-Z0-9]{36})", 1, "***GITHUB_TOKEN***"),
        SecretPattern::new("github_oauth", r"(gho_[a-zA-Z0-9]{36})", 1, "***GITHUB_OAUTH***"),
        SecretPattern::new("github_pat", r"(github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})", 1, "***GITHUB_PAT***"),

        // Private keys (PEM format headers)
        SecretPattern::new("private_key", r"(-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----)", 1, "***PRIVATE_KEY_REDACTED***"),

        // Generic secrets (environment variable format)
        SecretPattern::new("env_secret", r#"(?i)(secret|private[_-]?key|access[_-]?token)[=:\s]+['"]?([a-zA-Z0-9_/+=.-]{20,})['"]?"#, 2, "***SECRET***"),
    ]
}

lazy_static! {
    /// Pattern types for different secret formats
    static ref SECRET_PATTERNS: Vec<SecretPattern> = build_secret_patterns();
}

/// Redact all known secret patterns from a string
///
/// This function applies all registered secret patterns to redact
/// sensitive information from the input string.
///
/// # Examples
///
/// ```
/// use brainwires_cli::utils::secret_redaction::redact_secrets;
///
/// let log = "Connecting with api_key=sk-1234567890abcdef1234567890abcdef12345678901234567890";
/// let redacted = redact_secrets(log);
/// assert!(!redacted.contains("sk-"));
/// ```
pub fn redact_secrets(input: &str) -> Cow<'_, str> {
    let mut result: Cow<str> = Cow::Borrowed(input);

    for pattern in SECRET_PATTERNS.iter() {
        match result {
            Cow::Borrowed(s) => {
                result = pattern.redact(s);
            }
            Cow::Owned(ref s) => {
                let redacted = pattern.redact(s);
                if let Cow::Owned(new) = redacted {
                    result = Cow::Owned(new);
                }
            }
        }
    }

    result
}

/// Redact secrets from an error message, preserving the error type name
pub fn redact_error_message(error: &dyn std::error::Error) -> String {
    let message = error.to_string();
    redact_secrets(&message).into_owned()
}

/// A wrapper type that automatically redacts secrets when displayed
#[derive(Debug, Clone)]
pub struct RedactedString(String);

impl RedactedString {
    /// Create a new redacted string from a raw string
    pub fn new(raw: &str) -> Self {
        Self(redact_secrets(raw).into_owned())
    }

    /// Get the redacted content
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RedactedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RedactedString {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&str> for RedactedString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_key_redaction() {
        let input = "Using API key: sk-1234567890abcdef1234567890abcdef12345678901234567890";
        let result = redact_secrets(input);
        assert!(result.contains("***OPENAI_KEY***"));
        assert!(!result.contains("sk-1234567890"));
    }

    #[test]
    fn test_anthropic_key_redaction() {
        let input = "Anthropic key: sk-ant-api03-abc123def456ghi789jkl012mno345pqr678";
        let result = redact_secrets(input);
        assert!(result.contains("***ANTHROPIC_KEY***"));
        assert!(!result.contains("sk-ant-api03"));
    }

    #[test]
    fn test_bearer_token_redaction() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        let result = redact_secrets(input);
        assert!(result.contains("***TOKEN***") || result.contains("***SUPABASE_JWT***"));
        assert!(!result.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_api_key_in_url() {
        let input = "GET /api/data?api_key=abcdef1234567890abcdef1234567890";
        let result = redact_secrets(input);
        assert!(result.contains("***"));
        assert!(!result.contains("abcdef1234567890"));
    }

    #[test]
    fn test_password_redaction() {
        // Test with double quotes (common JSON format)
        let input = r#"Config: {"password": "super_secret_password123"}"#;
        let result = redact_secrets(input);
        assert!(result.contains("***PASSWORD***"), "Result was: {}", result);
        assert!(!result.contains("super_secret_password123"));

        // Test with equals and no quotes (env var format)
        let input2 = "password=mysecretpass123";
        let result2 = redact_secrets(input2);
        assert!(result2.contains("***PASSWORD***"), "Result was: {}", result2);
        assert!(!result2.contains("mysecretpass123"));
    }

    #[test]
    fn test_postgres_connection_string() {
        let input = "DATABASE_URL=postgres://user:password123@localhost:5432/db";
        let result = redact_secrets(input);
        assert!(result.contains("***PASSWORD***"));
        assert!(!result.contains("password123"));
        // Should still have the username and host
        assert!(result.contains("user:"));
        assert!(result.contains("@localhost"));
    }

    #[test]
    fn test_session_token_redaction() {
        let input = "session_token=a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4";
        let result = redact_secrets(input);
        assert!(result.contains("***SESSION***"));
        assert!(!result.contains("a1b2c3d4e5f6"));
    }

    #[test]
    fn test_aws_keys_redaction() {
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let result = redact_secrets(input);
        assert!(result.contains("***AWS_KEY***"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_github_token_redaction() {
        let input = "GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let result = redact_secrets(input);
        assert!(result.contains("***GITHUB_TOKEN***"));
        assert!(!result.contains("ghp_1234567890"));
    }

    #[test]
    fn test_private_key_header_redaction() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...";
        let result = redact_secrets(input);
        assert!(result.contains("***PRIVATE_KEY_REDACTED***"));
    }

    #[test]
    fn test_preserves_non_secret_content() {
        let input = "This is a normal log message with no secrets";
        let result = redact_secrets(input);
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn test_multiple_secrets() {
        // OpenAI key + password with double quotes
        let input = r#"api_key="sk-1234567890abcdef1234567890abcdef12345678901234567890" password="secret123""#;
        let result = redact_secrets(input);
        assert!(result.contains("***OPENAI_KEY***") || result.contains("***API_KEY***"), "Result was: {}", result);
        assert!(result.contains("***PASSWORD***"), "Result was: {}", result);
        assert!(!result.contains("sk-1234567890"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_redacted_string_wrapper() {
        let raw = "Using key: sk-1234567890abcdef1234567890abcdef12345678901234567890";
        let redacted = RedactedString::new(raw);
        assert!(redacted.as_str().contains("***OPENAI_KEY***"));
        assert!(!redacted.as_str().contains("sk-1234567890"));
    }
}
