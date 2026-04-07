//! Main executor that routes to appropriate language implementation

use crate::types::{ExecutionError, ExecutionLimits, ExecutionRequest, ExecutionResult, Language};

/// Main code executor that dispatches to language-specific implementations
pub struct Executor {
    limits: ExecutionLimits,
}

impl Executor {
    /// Create a new executor with default limits
    pub fn new() -> Self {
        Self {
            limits: ExecutionLimits::default(),
        }
    }

    /// Create a new executor with custom limits
    pub fn with_limits(limits: ExecutionLimits) -> Self {
        Self { limits }
    }

    /// Execute code in the specified language
    pub fn execute(&self, request: ExecutionRequest) -> ExecutionResult {
        // Merge limits
        let request = if request.limits.is_none() {
            ExecutionRequest {
                limits: Some(self.limits.clone()),
                ..request
            }
        } else {
            request
        };

        // Dispatch to appropriate executor
        match request.language {
            #[cfg(feature = "rhai")]
            Language::Rhai => {
                use crate::languages::rhai::RhaiExecutor;
                let executor = RhaiExecutor::with_limits(
                    request
                        .limits
                        .clone()
                        .unwrap_or_else(ExecutionLimits::default),
                );
                executor.execute_code(&request)
            }

            #[cfg(feature = "lua")]
            Language::Lua => {
                use crate::languages::lua::LuaExecutor;
                let executor = LuaExecutor::with_limits(
                    request
                        .limits
                        .clone()
                        .unwrap_or_else(ExecutionLimits::default),
                );
                executor.execute_code(&request)
            }

            #[cfg(feature = "javascript")]
            Language::JavaScript => {
                use crate::languages::javascript::JavaScriptExecutor;
                let executor = JavaScriptExecutor::with_limits(
                    request
                        .limits
                        .clone()
                        .unwrap_or_else(ExecutionLimits::default),
                );
                executor.execute_code(&request)
            }

            #[cfg(feature = "python")]
            Language::Python => {
                use crate::languages::python::PythonExecutor;
                let executor = PythonExecutor::with_limits(
                    request
                        .limits
                        .clone()
                        .unwrap_or_else(ExecutionLimits::default),
                );
                executor.execute_code(&request)
            }

            #[allow(unreachable_patterns)]
            _ => ExecutionError::UnsupportedLanguage(request.language.to_string()).to_result(0),
        }
    }

    /// Execute code from a string, parsing the language
    pub fn execute_str(&self, language: &str, code: &str) -> ExecutionResult {
        match Language::parse(language) {
            Some(lang) => self.execute(ExecutionRequest {
                language: lang,
                code: code.to_string(),
                limits: Some(self.limits.clone()),
                ..Default::default()
            }),
            None => ExecutionError::UnsupportedLanguage(language.to_string()).to_result(0),
        }
    }

    /// Get list of supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        crate::supported_languages()
    }

    /// Check if a language is supported
    pub fn is_supported(&self, language: Language) -> bool {
        crate::is_language_supported(language)
    }

    /// Get the current limits
    pub fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = Executor::new();
        assert!(!executor.supported_languages().is_empty());
    }

    #[test]
    fn test_executor_with_limits() {
        let limits = ExecutionLimits::strict();
        let executor = Executor::with_limits(limits.clone());
        assert_eq!(executor.limits().max_timeout_ms, limits.max_timeout_ms);
    }

    #[test]
    #[cfg(feature = "rhai")]
    fn test_rhai_execution() {
        let executor = Executor::new();
        let result = executor.execute_str("rhai", "1 + 2");
        assert!(result.success);
        assert!(result.stdout.contains("3"));
    }

    #[test]
    #[cfg(feature = "lua")]
    fn test_lua_execution() {
        let executor = Executor::new();
        let result = executor.execute_str("lua", "return 1 + 2");
        assert!(result.success);
        assert!(result.stdout.contains("3"));
    }

    #[test]
    #[cfg(feature = "javascript")]
    fn test_javascript_execution() {
        let executor = Executor::new();
        let result = executor.execute_str("js", "1 + 2");
        assert!(result.success);
        assert!(result.stdout.contains("3"));
    }

    #[test]
    #[cfg(feature = "python")]
    fn test_python_execution() {
        let executor = Executor::new();
        let result = executor.execute_str("python", "print(1 + 2)");
        assert!(result.success);
        assert!(result.stdout.contains("3"));
    }

    #[test]
    fn test_unsupported_language() {
        let executor = Executor::new();
        let result = executor.execute_str("cobol", "DISPLAY 'HELLO'");
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not supported"));
    }

    #[test]
    fn test_language_aliases() {
        // Test that language parsing works for aliases
        // (whether the language is actually supported depends on features)
        assert!(Language::parse("python").is_some());
        assert!(Language::parse("py").is_some());
        assert!(Language::parse("javascript").is_some());
        assert!(Language::parse("js").is_some());
        assert!(Language::parse("lua").is_some());
        assert!(Language::parse("rhai").is_some());

        // Ensure both aliases map to the same variant
        assert_eq!(Language::parse("python"), Language::parse("py"));
        assert_eq!(Language::parse("javascript"), Language::parse("js"));
    }
}
