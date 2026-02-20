//! WASM bindings for the tool orchestrator
//!
//! This module provides JavaScript-compatible bindings for the tool orchestrator,
//! allowing AI models to execute Rhai scripts that call registered tools from the browser.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::engine::dynamic_to_json;
use crate::sandbox::ExecutionLimits as CoreExecutionLimits;

// ============================================================================
// Engine Configuration Constants
// ============================================================================

/// Maximum expression nesting depth (prevents stack overflow from deeply nested expressions)
const MAX_EXPR_DEPTH: usize = 64;

/// Maximum function call nesting depth (prevents stack overflow from deep recursion)
const MAX_CALL_DEPTH: usize = 64;

use crate::types::{OrchestratorResult as CoreOrchestratorResult, ToolCall as CoreToolCall};

// ============================================================================
// WASM-compatible ExecutionLimits wrapper
// ============================================================================

/// Execution limits for safe script execution (WASM-compatible)
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    inner: CoreExecutionLimits,
}

#[wasm_bindgen]
impl ExecutionLimits {
    /// Create new execution limits with defaults.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: CoreExecutionLimits::default(),
        }
    }

    /// Create quick execution limits for simple scripts.
    #[wasm_bindgen]
    #[must_use]
    pub fn quick() -> Self {
        Self {
            inner: CoreExecutionLimits::quick(),
        }
    }

    /// Create extended limits for complex orchestration.
    #[wasm_bindgen]
    #[must_use]
    pub fn extended() -> Self {
        Self {
            inner: CoreExecutionLimits::extended(),
        }
    }

    /// Get max operations.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn max_operations(&self) -> u64 {
        self.inner.max_operations
    }

    /// Set max operations.
    #[wasm_bindgen(setter)]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn set_max_operations(&mut self, value: u64) {
        self.inner.max_operations = value;
    }

    /// Get max tool calls.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn max_tool_calls(&self) -> usize {
        self.inner.max_tool_calls
    }

    /// Set max tool calls.
    #[wasm_bindgen(setter)]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn set_max_tool_calls(&mut self, value: usize) {
        self.inner.max_tool_calls = value;
    }

    /// Get timeout in milliseconds.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn timeout_ms(&self) -> u64 {
        self.inner.timeout_ms
    }

    /// Set timeout in milliseconds.
    #[wasm_bindgen(setter)]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn set_timeout_ms(&mut self, value: u64) {
        self.inner.timeout_ms = value;
    }

    /// Get max string size.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn max_string_size(&self) -> usize {
        self.inner.max_string_size
    }

    /// Set max string size.
    #[wasm_bindgen(setter)]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn set_max_string_size(&mut self, value: usize) {
        self.inner.max_string_size = value;
    }

    /// Get max array size.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn max_array_size(&self) -> usize {
        self.inner.max_array_size
    }

    /// Set max array size.
    #[wasm_bindgen(setter)]
    #[allow(clippy::missing_const_for_fn)] // wasm_bindgen doesn't support const fn
    pub fn set_max_array_size(&mut self, value: usize) {
        self.inner.max_array_size = value;
    }
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WASM Orchestrator
// ============================================================================

/// Tool executor function type (JavaScript callback)
type JsToolExecutor = Rc<RefCell<js_sys::Function>>;

/// WASM-compatible tool orchestrator.
///
/// This wraps the core `ToolOrchestrator` and provides JavaScript-friendly bindings
/// for registering tools and executing scripts.
#[wasm_bindgen]
pub struct WasmOrchestrator {
    /// JavaScript tool executors (separate from core orchestrator)
    js_executors: HashMap<String, JsToolExecutor>,
}

#[wasm_bindgen]
impl WasmOrchestrator {
    /// Create a new WASM orchestrator.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        Self {
            js_executors: HashMap::new(),
        }
    }

    /// Register a tool executor function
    ///
    /// The function should accept a JSON string and return a string result.
    #[wasm_bindgen]
    pub fn register_tool(&mut self, name: &str, callback: js_sys::Function) {
        self.js_executors
            .insert(name.to_string(), Rc::new(RefCell::new(callback)));
    }

    /// Get list of registered tool names.
    #[wasm_bindgen]
    #[must_use]
    pub fn registered_tools(&self) -> Vec<String> {
        self.js_executors.keys().cloned().collect()
    }

    /// Execute a Rhai script with the registered tools.
    ///
    /// Returns a `JsValue` containing the `OrchestratorResult`.
    ///
    /// # Errors
    ///
    /// Returns `JsValue` error if serialization fails.
    #[wasm_bindgen]
    #[allow(clippy::too_many_lines)] // Execute function is inherently complex
    pub fn execute(&self, script: &str, limits: &ExecutionLimits) -> Result<JsValue, JsValue> {
        use web_time::Instant;

        let start_time = Instant::now();
        let tool_calls: Rc<RefCell<Vec<CoreToolCall>>> = Rc::new(RefCell::new(Vec::new()));
        let call_count: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

        // Create a new Rhai engine with limits
        let mut engine = rhai::Engine::new();

        // Apply resource limits from ExecutionLimits
        engine.set_max_operations(limits.inner.max_operations);
        engine.set_max_string_size(limits.inner.max_string_size);
        engine.set_max_array_size(limits.inner.max_array_size);
        engine.set_max_map_size(limits.inner.max_map_size);
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_CALL_DEPTH);

        // Set up real-time timeout via on_progress callback
        let timeout_ms = limits.inner.timeout_ms;
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

        // Register each JS tool as a Rhai function
        for (name, executor) in &self.js_executors {
            let exec = Rc::clone(executor);
            let calls = Rc::clone(&tool_calls);
            let count = Rc::clone(&call_count);
            let max_calls = limits.inner.max_tool_calls;
            let tool_name = name.clone();

            engine.register_fn(name.as_str(), move |input: rhai::Dynamic| -> String {
                let call_start = Instant::now();

                // Check call limit
                {
                    let mut c = count.borrow_mut();
                    if *c >= max_calls {
                        return format!("ERROR: Maximum tool calls ({max_calls}) exceeded");
                    }
                    *c += 1;
                }

                // Convert Dynamic to JSON
                let json_input = dynamic_to_json(&input);
                let json_str = serde_json::to_string(&json_input).unwrap_or_default();

                // Call the JavaScript function
                let callback = exec.borrow();
                let js_input = JsValue::from_str(&json_str);

                let (output, success) = match callback.call1(&JsValue::NULL, &js_input) {
                    Ok(result) => result.as_string().map_or_else(
                        || ("Tool returned non-string result".to_string(), false),
                        |s| (s, true),
                    ),
                    Err(e) => {
                        let err_msg = e
                            .as_string()
                            .map_or_else(|| "Tool execution failed".to_string(), |s| format!("Tool error: {s}"));
                        (err_msg, false)
                    }
                };

                // Record the call (saturate to u64::MAX for extremely long-running calls)
                {
                    let duration_ms = u64::try_from(call_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                    let call = CoreToolCall::new(
                        tool_name.clone(),
                        json_input,
                        output.clone(),
                        success,
                        duration_ms,
                    );
                    calls.borrow_mut().push(call);
                }

                output
            });
        }

        // Compile the script
        let ast = match engine.compile(script) {
            Ok(ast) => ast,
            Err(e) => {
                let result = CoreOrchestratorResult::error(
                    format!("Compilation error: {e}"),
                    tool_calls.borrow().clone(),
                    u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
                );
                return serde_wasm_bindgen::to_value(&result)
                    .map_err(|e| JsValue::from_str(&e.to_string()));
            }
        };

        // Execute the script
        let mut scope = rhai::Scope::new();
        let eval_result = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast);

        let execution_time_ms = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
        let calls = tool_calls.borrow().clone();

        match eval_result {
            Ok(result) => {
                let output = if result.is_string() {
                    result.into_string().unwrap_or_default()
                } else if result.is_unit() {
                    String::new()
                } else {
                    format!("{result:?}")
                };

                let result = CoreOrchestratorResult::success(output, calls, execution_time_ms);
                serde_wasm_bindgen::to_value(&result)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(e) => {
                let error_msg = match *e {
                    rhai::EvalAltResult::ErrorTooManyOperations(_) => {
                        format!(
                            "Script exceeded maximum operations ({})",
                            limits.inner.max_operations
                        )
                    }
                    rhai::EvalAltResult::ErrorTerminated(_, _) => {
                        format!(
                            "Script execution timed out after {}ms",
                            limits.inner.timeout_ms
                        )
                    }
                    _ => format!("Execution error: {e}"),
                };

                let result = CoreOrchestratorResult::error(error_msg, calls, execution_time_ms);
                serde_wasm_bindgen::to_value(&result)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
        }
    }
}

impl Default for WasmOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_limits_default() {
        let limits = ExecutionLimits::default();
        assert_eq!(limits.max_operations(), 100_000);
        assert_eq!(limits.max_tool_calls(), 50);
    }

    #[test]
    fn test_execution_limits_quick() {
        let limits = ExecutionLimits::quick();
        assert_eq!(limits.max_operations(), 10_000);
        assert_eq!(limits.max_tool_calls(), 10);
    }

    #[test]
    fn test_wasm_orchestrator_creation() {
        let orchestrator = WasmOrchestrator::new();
        assert!(orchestrator.registered_tools().is_empty());
    }
}
