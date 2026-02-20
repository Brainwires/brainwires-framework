//! Language-specific executors

#[cfg(feature = "rhai")]
pub mod rhai;

#[cfg(feature = "lua")]
pub mod lua;

#[cfg(feature = "javascript")]
pub mod javascript;

#[cfg(feature = "python")]
pub mod python;

use crate::types::{ExecutionLimits, ExecutionRequest, ExecutionResult};

/// Trait for language executors
pub trait LanguageExecutor {
    /// Execute code and return the result
    fn execute(&self, request: &ExecutionRequest) -> ExecutionResult;

    /// Get the language name
    fn language_name(&self) -> &'static str;

    /// Get the language version
    fn language_version(&self) -> String;
}

/// Helper to create execution limits from request
pub(crate) fn get_limits(request: &ExecutionRequest) -> ExecutionLimits {
    request.limits.clone().unwrap_or_else(|| {
        let mut limits = ExecutionLimits::default();
        limits.max_timeout_ms = request.timeout_ms;
        limits.max_memory_mb = request.memory_limit_mb;
        limits
    })
}

/// Helper to truncate output if too large
pub(crate) fn truncate_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        output.to_string()
    } else {
        let truncated = &output[..max_bytes];
        format!("{}...\n[Output truncated at {} bytes]", truncated, max_bytes)
    }
}
