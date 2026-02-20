//! Rhai engine setup and tool orchestration.
//!
//! This module contains the core [`ToolOrchestrator`] struct that executes
//! Rhai scripts with access to registered tools. It implements Anthropic's
//! "Programmatic Tool Calling" pattern.
//!
//! # Architecture
//!
//! The orchestrator uses feature-gated thread-safety primitives:
//!
//! - **`native`** feature: Uses `Arc<Mutex<T>>` for thread-safe execution
//! - **`wasm`** feature: Uses `Rc<RefCell<T>>` for single-threaded WASM
//!
//! # Key Components
//!
//! - [`ToolOrchestrator`] - Main entry point for script execution
//! - [`ToolExecutor`] - Type alias for tool callback functions
//! - [`dynamic_to_json`] - Converts Rhai values to JSON for tool input
//!
//! # Example
//!
//! ```ignore
//! use tool_orchestrator::{ToolOrchestrator, ExecutionLimits};
//!
//! let mut orchestrator = ToolOrchestrator::new();
//!
//! // Register a tool
//! orchestrator.register_executor("greet", |input| {
//!     let name = input.as_str().unwrap_or("world");
//!     Ok(format!("Hello, {}!", name))
//! });
//!
//! // Execute a script that uses the tool
//! let result = orchestrator.execute(
//!     r#"greet("Claude")"#,
//!     ExecutionLimits::default()
//! )?;
//!
//! assert_eq!(result.output, "Hello, Claude!");
//! ```
//!
//! # Security
//!
//! The Rhai engine is sandboxed by default with no access to:
//! - File system
//! - Network
//! - Shell commands
//! - System time (except via provided primitives)
//!
//! All resource limits are enforced via [`ExecutionLimits`].

use std::collections::HashMap;

#[cfg(feature = "native")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use std::time::Instant;

#[cfg(feature = "wasm")]
use std::cell::RefCell;
#[cfg(feature = "wasm")]
use std::rc::Rc;
#[cfg(feature = "wasm")]
use web_time::Instant;

use rhai::{Engine, EvalAltResult, Scope};

use crate::sandbox::ExecutionLimits;
use crate::types::{OrchestratorError, OrchestratorResult, ToolCall};

// ============================================================================
// Engine Configuration Constants
// ============================================================================

/// Maximum expression nesting depth (prevents stack overflow from deeply nested expressions)
const MAX_EXPR_DEPTH: usize = 64;

/// Maximum function call nesting depth (prevents stack overflow from deep recursion)
const MAX_CALL_DEPTH: usize = 64;

// ============================================================================
// Type aliases for thread-safety primitives (feature-gated)
// ============================================================================

/// Thread-safe vector wrapper (native: `Arc<Mutex<Vec<T>>>`)
#[cfg(feature = "native")]
pub type SharedVec<T> = Arc<Mutex<Vec<T>>>;

/// Thread-safe counter wrapper (native: `Arc<Mutex<usize>>`)
#[cfg(feature = "native")]
pub type SharedCounter = Arc<Mutex<usize>>;

/// Tool executor function type (native: thread-safe `Arc<dyn Fn>`)
///
/// Tools receive JSON input and return either a success string or error string.
///
/// # Example
///
/// ```ignore
/// orchestrator.register_executor("my_tool", |input: serde_json::Value| {
///     // Process input and return result
///     Ok("result".to_string())
/// });
/// ```
#[cfg(feature = "native")]
pub type ToolExecutor = Arc<dyn Fn(serde_json::Value) -> Result<String, String> + Send + Sync>;

/// Single-threaded vector wrapper (WASM: `Rc<RefCell<Vec<T>>>`)
#[cfg(feature = "wasm")]
pub type SharedVec<T> = Rc<RefCell<Vec<T>>>;

/// Single-threaded counter wrapper (WASM: `Rc<RefCell<usize>>`)
#[cfg(feature = "wasm")]
pub type SharedCounter = Rc<RefCell<usize>>;

/// Tool executor function type (WASM: single-threaded `Rc<dyn Fn>`)
///
/// Tools receive JSON input and return either a success string or error string.
#[cfg(feature = "wasm")]
pub type ToolExecutor = Rc<dyn Fn(serde_json::Value) -> Result<String, String>>;

// ============================================================================
// Helper functions for shared state (feature-gated)
// ============================================================================
//
// These functions abstract over the difference between native (Arc/Mutex)
// and WASM (Rc/RefCell) shared state primitives, allowing the main code
// to be feature-agnostic.

#[cfg(feature = "native")]
fn new_shared_vec<T>() -> SharedVec<T> {
    Arc::new(Mutex::new(Vec::new()))
}

#[cfg(feature = "wasm")]
fn new_shared_vec<T>() -> SharedVec<T> {
    Rc::new(RefCell::new(Vec::new()))
}

#[cfg(feature = "native")]
fn new_shared_counter() -> SharedCounter {
    Arc::new(Mutex::new(0))
}

#[cfg(feature = "wasm")]
fn new_shared_counter() -> SharedCounter {
    Rc::new(RefCell::new(0))
}

#[cfg(feature = "native")]
fn clone_shared<T: ?Sized>(shared: &Arc<T>) -> Arc<T> {
    Arc::clone(shared)
}

#[cfg(feature = "wasm")]
fn clone_shared<T: ?Sized>(shared: &Rc<T>) -> Rc<T> {
    Rc::clone(shared)
}

#[cfg(feature = "native")]
fn lock_vec<T: Clone>(shared: &SharedVec<T>) -> Vec<T> {
    shared.lock().unwrap().clone()
}

#[cfg(feature = "wasm")]
fn lock_vec<T: Clone>(shared: &SharedVec<T>) -> Vec<T> {
    shared.borrow().clone()
}

#[cfg(feature = "native")]
fn push_to_vec<T>(shared: &SharedVec<T>, item: T) {
    shared.lock().unwrap().push(item);
}

#[cfg(feature = "wasm")]
fn push_to_vec<T>(shared: &SharedVec<T>, item: T) {
    shared.borrow_mut().push(item);
}

#[cfg(feature = "native")]
fn increment_counter(shared: &SharedCounter, max: usize) -> Result<(), ()> {
    let mut c = shared.lock().unwrap();
    if *c >= max {
        return Err(());
    }
    *c += 1;
    drop(c); // Release lock early to avoid unnecessary contention
    Ok(())
}

#[cfg(feature = "wasm")]
fn increment_counter(shared: &SharedCounter, max: usize) -> Result<(), ()> {
    let mut c = shared.borrow_mut();
    if *c >= max {
        return Err(());
    }
    *c += 1;
    Ok(())
}

// ============================================================================
// ToolOrchestrator
// ============================================================================

/// Tool orchestrator - executes Rhai scripts with registered tool access.
///
/// The `ToolOrchestrator` is the main entry point for programmatic tool calling.
/// It manages tool registration and script execution within a sandboxed Rhai
/// environment.
///
/// # Features
///
/// - **Tool Registration**: Register Rust functions as callable tools
/// - **Script Execution**: Run Rhai scripts that can invoke registered tools
/// - **Resource Limits**: Configurable limits prevent runaway execution
/// - **Audit Trail**: All tool calls are logged with timing information
///
/// # Thread Safety
///
/// - With the `native` feature, the orchestrator is thread-safe
/// - With the `wasm` feature, it's single-threaded for WASM compatibility
///
/// # Example
///
/// ```ignore
/// use tool_orchestrator::{ToolOrchestrator, ExecutionLimits};
///
/// let mut orchestrator = ToolOrchestrator::new();
///
/// // Register tools
/// orchestrator.register_executor("add", |input| {
///     let arr = input.as_array().unwrap();
///     let sum: i64 = arr.iter().filter_map(|v| v.as_i64()).sum();
///     Ok(sum.to_string())
/// });
///
/// // Execute script
/// let result = orchestrator.execute(
///     r#"
///     let a = add([1, 2, 3]);
///     let b = add([4, 5, 6]);
///     `Sum: ${a} + ${b}`
///     "#,
///     ExecutionLimits::default()
/// )?;
///
/// println!("{}", result.output);  // "Sum: 6 + 15"
/// println!("Tool calls: {}", result.tool_calls.len());  // 2
/// ```
pub struct ToolOrchestrator {
    #[allow(dead_code)]
    engine: Engine,
    executors: HashMap<String, ToolExecutor>,
}

impl ToolOrchestrator {
    /// Create a new tool orchestrator with default settings.
    ///
    /// Initializes a fresh Rhai engine with expression depth limits
    /// and an empty tool registry.
    #[must_use]
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Limit expression nesting depth to prevent stack overflow
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_CALL_DEPTH);

        Self {
            engine,
            executors: HashMap::new(),
        }
    }

    /// Register a tool executor function (native version - thread-safe).
    ///
    /// The executor function receives JSON input from the Rhai script and
    /// returns either a success string or an error string.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the tool will be callable as in Rhai scripts
    /// * `executor` - Function that processes tool calls
    ///
    /// # Example
    ///
    /// ```ignore
    /// orchestrator.register_executor("fetch_user", |input| {
    ///     let user_id = input.as_i64().ok_or("Expected user ID")?;
    ///     // Fetch user from database...
    ///     Ok(format!(r#"{{"id": {}, "name": "Alice"}}"#, user_id))
    /// });
    /// ```
    #[cfg(feature = "native")]
    pub fn register_executor<F>(&mut self, name: impl Into<String>, executor: F)
    where
        F: Fn(serde_json::Value) -> Result<String, String> + Send + Sync + 'static,
    {
        self.executors.insert(name.into(), Arc::new(executor));
    }

    /// Register a tool executor function (WASM version - single-threaded).
    ///
    /// See the native version for full documentation.
    #[cfg(feature = "wasm")]
    pub fn register_executor<F>(&mut self, name: impl Into<String>, executor: F)
    where
        F: Fn(serde_json::Value) -> Result<String, String> + 'static,
    {
        self.executors.insert(name.into(), Rc::new(executor));
    }

    /// Execute a Rhai script with access to registered tools.
    ///
    /// Compiles and runs the provided Rhai script, making all registered
    /// tools available as callable functions. Execution is bounded by the
    /// provided [`ExecutionLimits`].
    ///
    /// # Arguments
    ///
    /// * `script` - Rhai source code to execute
    /// * `limits` - Resource limits for this execution
    ///
    /// # Returns
    ///
    /// On success, returns [`OrchestratorResult`] containing:
    /// - The script's output (final expression value)
    /// - A log of all tool calls made
    /// - Execution timing information
    ///
    /// # Errors
    ///
    /// Returns [`OrchestratorError`] if:
    /// - Script fails to compile ([`CompilationError`])
    /// - Script throws a runtime error ([`ExecutionError`])
    /// - Operation limit exceeded ([`MaxOperationsExceeded`])
    /// - Time limit exceeded ([`Timeout`])
    ///
    /// [`CompilationError`]: OrchestratorError::CompilationError
    /// [`ExecutionError`]: OrchestratorError::ExecutionError
    /// [`MaxOperationsExceeded`]: OrchestratorError::MaxOperationsExceeded
    /// [`Timeout`]: OrchestratorError::Timeout
    pub fn execute(
        &self,
        script: &str,
        limits: ExecutionLimits,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        let start_time = Instant::now();
        let tool_calls: SharedVec<ToolCall> = new_shared_vec();
        let call_count: SharedCounter = new_shared_counter();

        // Create a new engine with limits for this execution
        let mut engine = Engine::new();

        // Apply resource limits from ExecutionLimits
        engine.set_max_operations(limits.max_operations);
        engine.set_max_string_size(limits.max_string_size);
        engine.set_max_array_size(limits.max_array_size);
        engine.set_max_map_size(limits.max_map_size);
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_CALL_DEPTH);

        // Set up real-time timeout via on_progress callback
        let timeout_ms = limits.timeout_ms;
        let progress_start = Instant::now();
        engine.on_progress(move |_ops| {
            // Use saturating conversion - elapsed time exceeding u64::MAX is always a timeout
            let elapsed = u64::try_from(progress_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            if elapsed > timeout_ms {
                Some(rhai::Dynamic::from("timeout"))
            } else {
                None
            }
        });

        // Register each tool as a Rhai function
        for (name, executor) in &self.executors {
            let exec = clone_shared(executor);
            let calls = clone_shared(&tool_calls);
            let count = clone_shared(&call_count);
            let max_calls = limits.max_tool_calls;
            let tool_name = name.clone();

            // Register as a function that takes a Dynamic and returns a String
            engine.register_fn(name.as_str(), move |input: rhai::Dynamic| -> String {
                let call_start = Instant::now();

                // Check call limit
                if increment_counter(&count, max_calls).is_err() {
                    return format!("ERROR: Maximum tool calls ({max_calls}) exceeded");
                }

                // Convert Dynamic to JSON
                let json_input = dynamic_to_json(&input);

                // Execute the tool
                let (output, success) = match exec(json_input.clone()) {
                    Ok(result) => (result, true),
                    Err(e) => (format!("Tool error: {e}"), false),
                };

                // Record the call (saturate to u64::MAX for extremely long-running calls)
                let duration_ms = u64::try_from(call_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                let call = ToolCall::new(
                    tool_name.clone(),
                    json_input,
                    output.clone(),
                    success,
                    duration_ms,
                );
                push_to_vec(&calls, call);

                output
            });
        }

        // Compile the script
        let ast = engine
            .compile(script)
            .map_err(|e| OrchestratorError::CompilationError(e.to_string()))?;

        // Execute with timeout handling
        let mut scope = Scope::new();
        let result = engine
            .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)
            .map_err(|e| match *e {
                EvalAltResult::ErrorTooManyOperations(_) => {
                    OrchestratorError::MaxOperationsExceeded(limits.max_operations)
                }
                EvalAltResult::ErrorTerminated(_, _) => {
                    OrchestratorError::Timeout(limits.timeout_ms)
                }
                _ => OrchestratorError::ExecutionError(e.to_string()),
            })?;

        let execution_time_ms = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);

        // Convert result to string
        let output = if result.is_string() {
            result.into_string().unwrap_or_default()
        } else if result.is_unit() {
            String::new()
        } else {
            format!("{result:?}")
        };

        let calls = lock_vec(&tool_calls);
        Ok(OrchestratorResult::success(output, calls, execution_time_ms))
    }

    /// Get list of registered tool names.
    ///
    /// Returns the names of all tools that have been registered with
    /// [`register_executor`]. These names are callable as functions
    /// in Rhai scripts.
    ///
    /// [`register_executor`]: Self::register_executor
    ///
    /// # Example
    ///
    /// ```ignore
    /// orchestrator.register_executor("tool_a", |_| Ok("a".into()));
    /// orchestrator.register_executor("tool_b", |_| Ok("b".into()));
    ///
    /// let tools = orchestrator.registered_tools();
    /// assert!(tools.contains(&"tool_a"));
    /// assert!(tools.contains(&"tool_b"));
    /// ```
    #[must_use]
    pub fn registered_tools(&self) -> Vec<&str> {
        self.executors.keys().map(String::as_str).collect()
    }
}

impl Default for ToolOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert Rhai [`Dynamic`] value to [`serde_json::Value`].
///
/// This function handles the conversion of Rhai's dynamic type system to
/// JSON for passing data to tool executors. Supports all common Rhai types:
///
/// - Strings → JSON strings
/// - Integers → JSON numbers
/// - Floats → JSON numbers
/// - Booleans → JSON booleans
/// - Arrays → JSON arrays (recursive)
/// - Maps → JSON objects (recursive)
/// - Unit → JSON null
/// - Other → Debug string representation
///
/// # Example
///
/// ```ignore
/// use rhai::Dynamic;
/// use tool_orchestrator::dynamic_to_json;
///
/// let d = Dynamic::from("hello");
/// let j = dynamic_to_json(&d);
/// assert_eq!(j, serde_json::json!("hello"));
/// ```
///
/// [`Dynamic`]: rhai::Dynamic
pub fn dynamic_to_json(value: &rhai::Dynamic) -> serde_json::Value {
    if value.is_string() {
        serde_json::Value::String(value.clone().into_string().unwrap_or_default())
    } else if value.is_int() {
        serde_json::Value::Number(serde_json::Number::from(value.clone().as_int().unwrap_or(0)))
    } else if value.is_float() {
        serde_json::json!(value.clone().as_float().unwrap_or(0.0))
    } else if value.is_bool() {
        serde_json::Value::Bool(value.clone().as_bool().unwrap_or(false))
    } else if value.is_array() {
        let arr: Vec<rhai::Dynamic> = value.clone().into_array().unwrap_or_default();
        serde_json::Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if value.is_map() {
        let map: rhai::Map = value.clone().cast();
        let mut json_map = serde_json::Map::new();
        for (k, v) in &map {
            json_map.insert(k.to_string(), dynamic_to_json(v));
        }
        serde_json::Value::Object(json_map)
    } else if value.is_unit() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(format!("{value:?}"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = ToolOrchestrator::new();
        assert!(orchestrator.registered_tools().is_empty());
    }

    #[test]
    fn test_register_executor() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("test_tool", |_| Ok("success".to_string()));
        assert!(orchestrator.registered_tools().contains(&"test_tool"));
    }

    #[test]
    fn test_simple_script() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator
            .execute("let x = 1 + 2; x", ExecutionLimits::default())
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "3");
    }

    #[test]
    fn test_string_interpolation() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator
            .execute(
                r#"let name = "world"; `Hello, ${name}!`"#,
                ExecutionLimits::default(),
            )
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "Hello, world!");
    }

    #[test]
    fn test_tool_execution() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("greet", |input| {
            let name = input.as_str().unwrap_or("stranger");
            Ok(format!("Hello, {}!", name))
        });

        let result = orchestrator
            .execute(r#"greet("Claude")"#, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Hello, Claude!");
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_name, "greet");
    }

    #[test]
    fn test_max_operations_limit() {
        let orchestrator = ToolOrchestrator::new();
        let limits = ExecutionLimits::default().with_max_operations(10);

        // This should exceed the operations limit
        let result = orchestrator.execute(
            "let sum = 0; for i in 0..1000 { sum += i; } sum",
            limits,
        );

        assert!(matches!(
            result,
            Err(OrchestratorError::MaxOperationsExceeded(_))
        ));
    }

    #[test]
    fn test_compilation_error() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator.execute(
            "this is not valid rhai syntax {{{{",
            ExecutionLimits::default(),
        );

        assert!(matches!(result, Err(OrchestratorError::CompilationError(_))));
    }

    #[test]
    fn test_multiple_tool_calls() {
        let mut orchestrator = ToolOrchestrator::new();

        orchestrator.register_executor("add", |input| {
            if let Some(arr) = input.as_array() {
                let sum: i64 = arr.iter().filter_map(|v| v.as_i64()).sum();
                Ok(sum.to_string())
            } else {
                Err("Expected array".to_string())
            }
        });

        let script = r#"
            let a = add([1, 2, 3]);
            let b = add([4, 5, 6]);
            `Sum1: ${a}, Sum2: ${b}`
        "#;

        let result = orchestrator
            .execute(script, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tool_calls.len(), 2);
        assert!(result.output.contains("Sum1: 6"));
        assert!(result.output.contains("Sum2: 15"));
    }

    #[test]
    fn test_tool_error_handling() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("fail_tool", |_| Err("Intentional failure".to_string()));

        let result = orchestrator
            .execute(r#"fail_tool("test")"#, ExecutionLimits::default())
            .unwrap();

        assert!(result.success); // Script completes, tool error is in output
        assert!(result.output.contains("Tool error"));
        assert_eq!(result.tool_calls.len(), 1);
        assert!(!result.tool_calls[0].success);
    }

    #[test]
    fn test_max_tool_calls_limit() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("count", |_| Ok("1".to_string()));

        let limits = ExecutionLimits::default().with_max_tool_calls(3);
        // Return the 4th call result directly so we can see the error
        let script = r#"
            let a = count("1");
            let b = count("2");
            let c = count("3");
            count("4")
        "#;

        let result = orchestrator.execute(script, limits).unwrap();

        // Fourth call should return error message instead of executing
        assert!(
            result.output.contains("Maximum tool calls"),
            "Expected error message about max tool calls, got: {}",
            result.output
        );
        // Only 3 calls should be recorded (the 4th was blocked)
        assert_eq!(result.tool_calls.len(), 3);
    }

    #[test]
    fn test_tool_with_map_input() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("get_value", |input| {
            if let Some(obj) = input.as_object() {
                if let Some(key) = obj.get("key").and_then(|v| v.as_str()) {
                    Ok(format!("Got key: {}", key))
                } else {
                    Err("Missing key field".to_string())
                }
            } else {
                Err("Expected object".to_string())
            }
        });

        let result = orchestrator
            .execute(r#"get_value(#{ key: "test_key" })"#, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Got key: test_key");
    }

    #[test]
    fn test_loop_with_tool_calls() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("double", |input| {
            let n = input.as_i64().unwrap_or(0);
            Ok((n * 2).to_string())
        });

        let script = r#"
            let results = [];
            for i in 1..4 {
                results.push(double(i));
            }
            results
        "#;

        let result = orchestrator
            .execute(script, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tool_calls.len(), 3);
    }

    #[test]
    fn test_conditional_tool_calls() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("check", |input| {
            let n = input.as_i64().unwrap_or(0);
            Ok(if n > 5 { "big" } else { "small" }.to_string())
        });

        let script = r#"
            let x = 10;
            if x > 5 {
                check(x)
            } else {
                "skipped"
            }
        "#;

        let result = orchestrator
            .execute(script, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "big");
        assert_eq!(result.tool_calls.len(), 1);
    }

    #[test]
    fn test_empty_script() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator
            .execute("", ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert!(result.output.is_empty());
    }

    #[test]
    fn test_unit_return() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator
            .execute("let x = 5;", ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert!(result.output.is_empty()); // Unit type returns empty string
    }

    #[test]
    fn test_dynamic_to_json_types() {
        // Test various Rhai Dynamic types convert to JSON correctly
        use rhai::Dynamic;

        // String
        let d = Dynamic::from("hello".to_string());
        let j = dynamic_to_json(&d);
        assert_eq!(j, serde_json::json!("hello"));

        // Integer
        let d = Dynamic::from(42_i64);
        let j = dynamic_to_json(&d);
        assert_eq!(j, serde_json::json!(42));

        // Float
        let d = Dynamic::from(3.14_f64);
        let j = dynamic_to_json(&d);
        assert!(j.as_f64().unwrap() - 3.14 < 0.001);

        // Boolean
        let d = Dynamic::from(true);
        let j = dynamic_to_json(&d);
        assert_eq!(j, serde_json::json!(true));

        // Unit (null)
        let d = Dynamic::UNIT;
        let j = dynamic_to_json(&d);
        assert_eq!(j, serde_json::Value::Null);
    }

    #[test]
    fn test_execution_time_recorded() {
        let orchestrator = ToolOrchestrator::new();
        let result = orchestrator
            .execute("let sum = 0; for i in 0..100 { sum += i; } sum", ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        // execution_time_ms is always recorded (u64 is always >= 0, but we verify a result exists)
        assert!(result.execution_time_ms < 10000); // Should complete in under 10 seconds
    }

    #[test]
    fn test_tool_call_duration_recorded() {
        let mut orchestrator = ToolOrchestrator::new();
        orchestrator.register_executor("slow_tool", |_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok("done".to_string())
        });

        let result = orchestrator
            .execute(r#"slow_tool("test")"#, ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tool_calls.len(), 1);
        assert!(result.tool_calls[0].duration_ms >= 10);
    }

    #[test]
    fn test_default_impl() {
        // Test that Default::default() works for ToolOrchestrator
        let orchestrator = ToolOrchestrator::default();
        assert!(orchestrator.registered_tools().is_empty());

        // Execute a simple script to verify it works
        let result = orchestrator
            .execute("1 + 1", ExecutionLimits::default())
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "2");
    }

    #[test]
    fn test_timeout_error() {
        let orchestrator = ToolOrchestrator::new();

        // Use a CPU-intensive loop that will trigger on_progress checks
        // Set timeout to 1ms - the loop will exceed this quickly
        let limits = ExecutionLimits::default()
            .with_timeout_ms(1)
            .with_max_operations(1_000_000); // Allow many ops so timeout triggers first

        // This loop will keep running until timeout kicks in via on_progress
        let result = orchestrator.execute(
            r#"
            let sum = 0;
            for i in 0..1000000 {
                sum += i;
            }
            sum
            "#,
            limits,
        );

        // Should return a timeout error (real-time via on_progress)
        assert!(result.is_err());
        match result {
            Err(OrchestratorError::Timeout(ms)) => assert_eq!(ms, 1),
            _ => panic!("Expected Timeout error, got: {:?}", result),
        }
    }

    #[test]
    fn test_runtime_error() {
        let orchestrator = ToolOrchestrator::new();

        // This should cause a runtime error (undefined variable)
        let result = orchestrator.execute("undefined_variable", ExecutionLimits::default());

        assert!(result.is_err());
        match result {
            Err(OrchestratorError::ExecutionError(msg)) => {
                assert!(msg.contains("undefined_variable") || msg.contains("not found"));
            }
            _ => panic!("Expected ExecutionError"),
        }
    }

    #[test]
    fn test_registered_tools() {
        let mut orchestrator = ToolOrchestrator::new();
        assert!(orchestrator.registered_tools().is_empty());

        orchestrator.register_executor("tool_a", |_| Ok("a".to_string()));
        orchestrator.register_executor("tool_b", |_| Ok("b".to_string()));

        let tools = orchestrator.registered_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"tool_a"));
        assert!(tools.contains(&"tool_b"));
    }

    #[test]
    fn test_dynamic_to_json_array() {
        use rhai::Dynamic;

        // Create an array
        let arr: Vec<Dynamic> = vec![
            Dynamic::from(1_i64),
            Dynamic::from(2_i64),
            Dynamic::from(3_i64),
        ];
        let d = Dynamic::from(arr);
        let j = dynamic_to_json(&d);

        assert_eq!(j, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_dynamic_to_json_map() {
        use rhai::{Dynamic, Map};

        // Create a map
        let mut map = Map::new();
        map.insert("key".into(), Dynamic::from("value".to_string()));
        map.insert("num".into(), Dynamic::from(42_i64));
        let d = Dynamic::from(map);
        let j = dynamic_to_json(&d);

        assert!(j.is_object());
        let obj = j.as_object().unwrap();
        assert_eq!(obj.get("key").unwrap(), &serde_json::json!("value"));
        assert_eq!(obj.get("num").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_non_string_result() {
        // Test that non-string results are formatted with Debug
        let orchestrator = ToolOrchestrator::new();

        // Return an integer (not a string)
        let result = orchestrator
            .execute("42", ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "42");
    }

    #[test]
    fn test_array_result() {
        // Test that array results are formatted
        let orchestrator = ToolOrchestrator::new();

        let result = orchestrator
            .execute("[1, 2, 3]", ExecutionLimits::default())
            .unwrap();

        assert!(result.success);
        // Arrays are formatted with Debug
        assert!(result.output.contains("1"));
        assert!(result.output.contains("2"));
        assert!(result.output.contains("3"));
    }

    #[test]
    fn test_dynamic_to_json_fallback() {
        use rhai::Dynamic;

        // Create a custom type that doesn't match standard types
        // Using a timestamp (FnPtr or similar) that falls through to the else branch
        #[derive(Clone)]
        struct CustomType {
            #[allow(dead_code)]
            value: i32,
        }

        let custom = CustomType { value: 42 };
        let d = Dynamic::from(custom);
        let j = dynamic_to_json(&d);

        // Should fall back to string representation via Debug
        assert!(j.is_string());
        // The string should contain some representation of the type
        let s = j.as_str().unwrap();
        assert!(!s.is_empty());
    }
}
