//! Python executor — stub implementation.
//!
//! The full implementation embeds [RustPython](https://rustpython.github.io/),
//! but pulling RustPython into the workspace currently triggers a `links =
//! "lzma"` collision with the `xz2` crate already present via `datafusion`/
//! `lancedb`. Until we decide on the right way to resolve that (vendored xz,
//! upstream feature flag, or a separate crate boundary), this module is a
//! compile-only stub: every call returns a runtime error explaining how to
//! enable the real backend.
//!
//! The `interpreters-python` feature still toggles the module (and its
//! re-exports) so downstream code that conditionally compiles against
//! `PythonExecutor` keeps working.

use std::time::Instant;

use super::super::types::{ExecutionLimits, ExecutionRequest, ExecutionResult};
use super::{LanguageExecutor, get_limits};

/// Python code executor — stub. See module docs for status.
pub struct PythonExecutor {
    _limits: ExecutionLimits,
}

impl PythonExecutor {
    /// Create a new Python executor with default limits.
    pub fn new() -> Self {
        Self {
            _limits: ExecutionLimits::default(),
        }
    }

    /// Create a new Python executor with the supplied limits.
    pub fn with_limits(limits: ExecutionLimits) -> Self {
        Self { _limits: limits }
    }

    /// Execute Python code.
    ///
    /// The stub always reports the interpreter as unavailable. Wire RustPython
    /// (or another backend) in here once the link-collision blocker is sorted.
    pub fn execute_code(&self, request: &ExecutionRequest) -> ExecutionResult {
        let _ = get_limits(request);
        let start = Instant::now();
        ExecutionResult {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            result: None,
            error: Some(
                "Python interpreter not yet wired into this build. \
                 The `interpreters-python` feature is currently a stub — \
                 see crates/brainwires-tools/src/interpreters/languages/python.rs."
                    .to_string(),
            ),
            timing_ms: start.elapsed().as_millis() as u64,
            memory_used_bytes: None,
            operations_count: None,
        }
    }
}

impl Default for PythonExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageExecutor for PythonExecutor {
    fn execute(&self, request: &ExecutionRequest) -> ExecutionResult {
        self.execute_code(request)
    }

    fn language_name(&self) -> &'static str {
        "python"
    }

    fn language_version(&self) -> String {
        // Once the real interpreter lands, surface its semantic version here.
        "stub".to_string()
    }
}
