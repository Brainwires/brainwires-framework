//! # Code Interpreters
//!
//! Sandboxed code execution for multiple programming languages.
//! Designed to work both natively and compiled to WASM for browser execution.
//!
//! ## Supported Languages
//!
//! | Language | Feature | Speed | Power | Notes |
//! |----------|---------|-------|-------|-------|
//! | Rhai | `rhai` | ⚡⚡⚡⚡ | ⭐⭐ | Native Rust, fastest startup |
//! | Lua | `lua` | ⚡⚡⚡ | ⭐⭐⭐ | Small runtime, good stdlib |
//! | JavaScript | `javascript` | ⚡⚡ | ⭐⭐⭐⭐ | ECMAScript compliant (Boa) |
//! | Python | `python` | ⚡ | ⭐⭐⭐⭐⭐ | CPython 3.12 compatible |
//!
//! ## Example
//!
//! ```rust
//! use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};
//!
//! let executor = Executor::new();
//! let result = executor.execute(ExecutionRequest {
//!     language: Language::Rhai,
//!     code: "let x = 1 + 2; x".to_string(),
//!     ..Default::default()
//! });
//!
//! assert!(result.success);
//! assert_eq!(result.stdout, "3");
//! ```

mod types;
mod executor;
mod languages;

#[cfg(feature = "wasm")]
mod wasm_bindings;

pub use types::*;
pub use executor::Executor;

// Re-export language-specific executors for advanced use
pub mod lang {
    #[cfg(feature = "rhai")]
    pub use crate::languages::rhai::RhaiExecutor;

    #[cfg(feature = "lua")]
    pub use crate::languages::lua::LuaExecutor;

    #[cfg(feature = "javascript")]
    pub use crate::languages::javascript::JavaScriptExecutor;

    #[cfg(feature = "python")]
    pub use crate::languages::python::PythonExecutor;
}

/// Get a list of supported languages based on enabled features
pub fn supported_languages() -> Vec<Language> {
    let mut languages = Vec::new();

    #[cfg(feature = "rhai")]
    languages.push(Language::Rhai);

    #[cfg(feature = "lua")]
    languages.push(Language::Lua);

    #[cfg(feature = "javascript")]
    languages.push(Language::JavaScript);

    #[cfg(feature = "python")]
    languages.push(Language::Python);

    languages
}

/// Check if a language is supported
pub fn is_language_supported(language: Language) -> bool {
    match language {
        #[cfg(feature = "rhai")]
        Language::Rhai => true,

        #[cfg(feature = "lua")]
        Language::Lua => true,

        #[cfg(feature = "javascript")]
        Language::JavaScript => true,

        #[cfg(feature = "python")]
        Language::Python => true,

        #[allow(unreachable_patterns)]
        _ => false,
    }
}
