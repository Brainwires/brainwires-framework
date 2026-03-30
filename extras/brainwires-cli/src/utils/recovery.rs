//! Recovery and retry system for handling transient failures
//!
//! Provides exponential backoff retry logic for operations that may fail
//! due to transient issues (network timeouts, rate limits, etc.)

use anyhow::{anyhow, Result};
use std::future::Future;
use std::time::Duration;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to add jitter to delay (prevents thundering herd)
    pub add_jitter: bool,
    /// Error patterns that should trigger retries
    pub retryable_patterns: Vec<String>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
            add_jitter: true,
            retryable_patterns: vec![
                "connection refused".into(),
                "connection reset".into(),
                "connection closed".into(),
                "timeout".into(),
                "timed out".into(),
                "rate limit".into(),
                "429".into(),
                "503".into(),
                "502".into(),
                "504".into(),
                "temporary".into(),
                "unavailable".into(),
                "ECONNRESET".into(),
                "ETIMEDOUT".into(),
                "ECONNREFUSED".into(),
            ],
        }
    }
}

impl RecoveryConfig {
    /// Create a configuration for quick retries (short delays)
    pub fn quick() -> Self {
        Self {
            max_retries: 2,
            initial_delay_ms: 50,
            max_delay_ms: 500,
            backoff_multiplier: 2.0,
            add_jitter: true,
            retryable_patterns: Self::default().retryable_patterns,
        }
    }

    /// Create a configuration for patient retries (longer delays)
    pub fn patient() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            add_jitter: true,
            retryable_patterns: Self::default().retryable_patterns,
        }
    }

    /// Create a configuration for aggressive retries (many attempts)
    pub fn aggressive() -> Self {
        Self {
            max_retries: 10,
            initial_delay_ms: 100,
            max_delay_ms: 60000,
            backoff_multiplier: 1.5,
            add_jitter: true,
            retryable_patterns: Self::default().retryable_patterns,
        }
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the initial delay
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Add a retryable error pattern
    pub fn with_retryable_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.retryable_patterns.push(pattern.into());
        self
    }
}

/// Manages retry logic with exponential backoff
pub struct RecoveryManager {
    config: RecoveryConfig,
}

impl RecoveryManager {
    /// Create a new recovery manager with default configuration
    pub fn new() -> Self {
        Self {
            config: RecoveryConfig::default(),
        }
    }

    /// Create a recovery manager with custom configuration
    pub fn with_config(config: RecoveryConfig) -> Self {
        Self { config }
    }

    /// Check if an error message matches any retryable pattern
    fn is_retryable(&self, error: &str) -> bool {
        let error_lower = error.to_lowercase();
        self.config
            .retryable_patterns
            .iter()
            .any(|pattern| error_lower.contains(&pattern.to_lowercase()))
    }

    /// Calculate delay with optional jitter
    fn calculate_delay(&self, base_delay: u64) -> Duration {
        let delay = base_delay.min(self.config.max_delay_ms);

        if self.config.add_jitter {
            // Add ±25% jitter
            let jitter_range = delay / 4;
            let jitter = if jitter_range > 0 {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as u64;
                (seed % (jitter_range * 2)).saturating_sub(jitter_range)
            } else {
                0
            };
            Duration::from_millis(delay.saturating_add_signed(jitter as i64))
        } else {
            Duration::from_millis(delay)
        }
    }

    /// Execute an async function with retry logic
    ///
    /// Returns the result on success, or the last error after all retries exhausted.
    pub async fn with_retry<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut delay_ms = self.config.initial_delay_ms;
        let mut attempts = 0;
        let mut last_error: Option<String> = None;

        loop {
            match operation().await {
                Ok(result) => {
                    if attempts > 0 {
                        debug!("Retry succeeded after {} attempt(s)", attempts);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    if self.is_retryable(&error_msg) && attempts < self.config.max_retries {
                        attempts += 1;
                        warn!(
                            "Retryable error (attempt {}/{}): {}",
                            attempts,
                            self.config.max_retries,
                            error_msg
                        );

                        let delay = self.calculate_delay(delay_ms);
                        debug!("Waiting {:?} before retry", delay);
                        tokio::time::sleep(delay).await;

                        delay_ms = ((delay_ms as f64) * self.config.backoff_multiplier) as u64;
                        last_error = Some(error_msg);
                    } else {
                        // Non-retryable error or max retries reached
                        if attempts > 0 {
                            return Err(anyhow!(
                                "Failed after {} retry attempt(s). Last error: {}",
                                attempts,
                                error_msg
                            ));
                        } else {
                            return Err(anyhow!("{}", error_msg));
                        }
                    }
                }
            }
        }
    }

    /// Execute an async function with retry, using anyhow::Result
    pub async fn retry_anyhow<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        self.with_retry(operation).await
    }

    /// Execute an async function with retry, providing attempt number to the operation
    pub async fn with_retry_context<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn(u32) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut delay_ms = self.config.initial_delay_ms;
        let mut attempts = 0;

        loop {
            match operation(attempts).await {
                Ok(result) => {
                    if attempts > 0 {
                        debug!("Retry succeeded after {} attempt(s)", attempts);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    if self.is_retryable(&error_msg) && attempts < self.config.max_retries {
                        attempts += 1;
                        warn!(
                            "Retryable error (attempt {}/{}): {}",
                            attempts,
                            self.config.max_retries,
                            error_msg
                        );

                        let delay = self.calculate_delay(delay_ms);
                        tokio::time::sleep(delay).await;
                        delay_ms = ((delay_ms as f64) * self.config.backoff_multiplier) as u64;
                    } else {
                        if attempts > 0 {
                            return Err(anyhow!(
                                "Failed after {} retry attempt(s). Last error: {}",
                                attempts,
                                error_msg
                            ));
                        } else {
                            return Err(anyhow!("{}", error_msg));
                        }
                    }
                }
            }
        }
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for one-off retries with default config
pub async fn with_retry<F, Fut, T, E>(operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    RecoveryManager::new().with_retry(operation).await
}

/// Convenience function for one-off retries with custom config
pub async fn with_retry_config<F, Fut, T, E>(config: RecoveryConfig, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    RecoveryManager::with_config(config).with_retry(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_recovery_config_defaults() {
        let config = RecoveryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert!(config.retryable_patterns.contains(&"timeout".to_string()));
    }

    #[test]
    fn test_recovery_config_presets() {
        let quick = RecoveryConfig::quick();
        assert_eq!(quick.max_retries, 2);
        assert!(quick.initial_delay_ms < RecoveryConfig::default().initial_delay_ms);

        let patient = RecoveryConfig::patient();
        assert_eq!(patient.max_retries, 5);
        assert!(patient.initial_delay_ms > RecoveryConfig::default().initial_delay_ms);

        let aggressive = RecoveryConfig::aggressive();
        assert_eq!(aggressive.max_retries, 10);
    }

    #[test]
    fn test_is_retryable() {
        let manager = RecoveryManager::new();

        assert!(manager.is_retryable("connection refused"));
        assert!(manager.is_retryable("Connection Refused")); // case insensitive
        assert!(manager.is_retryable("request timeout"));
        assert!(manager.is_retryable("rate limit exceeded"));
        assert!(manager.is_retryable("HTTP 429 Too Many Requests"));
        assert!(manager.is_retryable("service unavailable"));

        assert!(!manager.is_retryable("invalid argument"));
        assert!(!manager.is_retryable("permission denied"));
        assert!(!manager.is_retryable("not found"));
    }

    #[tokio::test]
    async fn test_retry_immediate_success() {
        let manager = RecoveryManager::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32> = manager.with_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, anyhow::Error>(42)
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let config = RecoveryConfig {
            max_retries: 3,
            initial_delay_ms: 1, // Very short for testing
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
            add_jitter: false,
            retryable_patterns: vec!["timeout".to_string()],
        };
        let manager = RecoveryManager::with_config(config);
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32> = manager.with_retry(|| {
            let c = count.clone();
            async move {
                let attempt = c.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(anyhow!("timeout error"))
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let config = RecoveryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
            add_jitter: false,
            retryable_patterns: vec!["timeout".to_string()],
        };
        let manager = RecoveryManager::with_config(config);
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32> = manager.with_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(anyhow!("permission denied"))
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // No retries
    }

    #[tokio::test]
    async fn test_retry_max_retries_exhausted() {
        let config = RecoveryConfig {
            max_retries: 2,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
            add_jitter: false,
            retryable_patterns: vec!["timeout".to_string()],
        };
        let manager = RecoveryManager::with_config(config);
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32> = manager.with_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(anyhow!("timeout error"))
            }
        }).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed after 2 retry attempt(s)"));
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
    }

    #[tokio::test]
    async fn test_retry_with_context() {
        let config = RecoveryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
            add_jitter: false,
            retryable_patterns: vec!["timeout".to_string()],
        };
        let manager = RecoveryManager::with_config(config);
        let attempts_seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let attempts = attempts_seen.clone();

        let result: Result<i32> = manager.with_retry_context(|attempt| {
            let a = attempts.clone();
            async move {
                a.lock().unwrap().push(attempt);
                if attempt < 2 {
                    Err(anyhow!("timeout error"))
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempts_seen.lock().unwrap(), vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_convenience_function() {
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32> = with_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, anyhow::Error>(42)
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
