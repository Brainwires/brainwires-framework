//! Python executor - CPython 3.12 compatible via RustPython
//!
//! RustPython is a Python interpreter written in Rust.
//! It aims for CPython 3.12 compatibility with growing stdlib support.
//!
//! ## Features
//! - Large standard library
//! - Familiar syntax for most developers
//! - Good for data processing and scripting
//!
//! ## Limitations
//! - Slower than CPython (no JIT)
//! - Some stdlib modules not yet implemented
//! - C extension modules not supported

use rustpython_vm::{
    AsObject, Interpreter, PyObjectRef, PyRef, PyResult, Settings, VirtualMachine,
    builtins::{PyBaseException, PyDict, PyList, PyNamespace},
    function::FuncArgs,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::{LanguageExecutor, get_limits, truncate_output};
use crate::types::{ExecutionLimits, ExecutionRequest, ExecutionResult};

/// Python code executor using RustPython
pub struct PythonExecutor {
    _limits: ExecutionLimits,
}

impl PythonExecutor {
    /// Create a new Python executor with default limits
    pub fn new() -> Self {
        Self {
            _limits: ExecutionLimits::default(),
        }
    }

    /// Create a new Python executor with custom limits
    pub fn with_limits(limits: ExecutionLimits) -> Self {
        Self { _limits: limits }
    }

    /// Execute Python code
    pub fn execute_code(&self, request: &ExecutionRequest) -> ExecutionResult {
        let limits = get_limits(request);
        let start = Instant::now();

        // Capture stdout/stderr
        let stdout_capture = Arc::new(Mutex::new(Vec::<String>::new()));
        let stderr_capture = Arc::new(Mutex::new(Vec::<String>::new()));

        // Create interpreter with stdlib
        let interp = Interpreter::with_init(Settings::default(), |vm| {
            vm.add_native_modules(rustpython_stdlib::get_module_inits());
        });

        let result = interp.enter(|vm| {
            // Redirect stdout/stderr
            self.setup_io(vm, stdout_capture.clone(), stderr_capture.clone())?;

            // Inject context as globals
            let scope = vm.new_scope_with_builtins();
            if let Some(context) = &request.context {
                self.inject_context(vm, &scope, context)?;
            }

            // Compile and execute
            let code_obj = vm
                .compile(
                    &request.code,
                    rustpython_vm::compiler::Mode::Exec,
                    "<script>".to_owned(),
                )
                .map_err(|e| vm.new_syntax_error(&e, Some(&request.code)))?;

            vm.run_code_obj(code_obj, scope)
        });

        let timing_ms = start.elapsed().as_millis() as u64;

        // Get captured output
        let stdout = stdout_capture
            .lock()
            .map(|out| out.join(""))
            .unwrap_or_default();
        let stdout = truncate_output(&stdout, limits.max_output_bytes);

        let stderr = stderr_capture
            .lock()
            .map(|err| err.join(""))
            .unwrap_or_default();

        match result {
            Ok(value) => {
                // Convert result to JSON
                let result_value = interp.enter(|vm| py_to_json(vm, &value));

                ExecutionResult {
                    success: true,
                    stdout,
                    stderr,
                    result: result_value,
                    error: None,
                    timing_ms,
                    memory_used_bytes: None,
                    operations_count: None,
                }
            }
            Err(exc) => {
                let error_message = interp.enter(|vm| format_python_error(vm, &exc));

                ExecutionResult {
                    success: false,
                    stdout,
                    stderr: if stderr.is_empty() {
                        error_message.clone()
                    } else {
                        format!("{}\n{}", stderr, error_message)
                    },
                    result: None,
                    error: Some(error_message),
                    timing_ms,
                    memory_used_bytes: None,
                    operations_count: None,
                }
            }
        }
    }

    /// Setup stdout/stderr redirection
    fn setup_io(
        &self,
        vm: &VirtualMachine,
        stdout: Arc<Mutex<Vec<String>>>,
        stderr: Arc<Mutex<Vec<String>>>,
    ) -> PyResult<()> {
        // Create stdout namespace object
        let stdout_obj = PyNamespace::new_ref(&vm.ctx);

        // Create stdout write function
        let stdout_clone = stdout.clone();
        let stdout_writer = vm.new_function(
            "write",
            move |args: FuncArgs, vm: &VirtualMachine| -> PyResult<PyObjectRef> {
                if let Some(arg) = args.args.first() {
                    if let Ok(s) = arg.str(vm) {
                        if let Ok(mut out) = stdout_clone.lock() {
                            out.push(s.as_str().to_string());
                        }
                    }
                }
                Ok(vm.ctx.none())
            },
        );

        // Create stdout flush function
        let stdout_flush = vm.new_function(
            "flush",
            |_: FuncArgs, vm: &VirtualMachine| -> PyResult<PyObjectRef> { Ok(vm.ctx.none()) },
        );

        // Set attributes on stdout object
        stdout_obj
            .as_object()
            .set_attr("write", stdout_writer, vm)?;
        stdout_obj.as_object().set_attr("flush", stdout_flush, vm)?;

        // Create stderr namespace object
        let stderr_obj = PyNamespace::new_ref(&vm.ctx);

        // Create stderr write function
        let stderr_clone = stderr.clone();
        let stderr_writer = vm.new_function(
            "write",
            move |args: FuncArgs, vm: &VirtualMachine| -> PyResult<PyObjectRef> {
                if let Some(arg) = args.args.first() {
                    if let Ok(s) = arg.str(vm) {
                        if let Ok(mut err) = stderr_clone.lock() {
                            err.push(s.as_str().to_string());
                        }
                    }
                }
                Ok(vm.ctx.none())
            },
        );

        // Create stderr flush function
        let stderr_flush = vm.new_function(
            "flush",
            |_: FuncArgs, vm: &VirtualMachine| -> PyResult<PyObjectRef> { Ok(vm.ctx.none()) },
        );

        // Set attributes on stderr object
        stderr_obj
            .as_object()
            .set_attr("write", stderr_writer, vm)?;
        stderr_obj.as_object().set_attr("flush", stderr_flush, vm)?;

        // Set sys.stdout and sys.stderr
        let sys = vm.import("sys", 0)?;
        sys.set_attr("stdout", stdout_obj, vm)?;
        sys.set_attr("stderr", stderr_obj, vm)?;

        Ok(())
    }

    /// Inject context variables into the scope
    fn inject_context(
        &self,
        vm: &VirtualMachine,
        scope: &rustpython_vm::scope::Scope,
        context: &serde_json::Value,
    ) -> PyResult<()> {
        if let serde_json::Value::Object(map) = context {
            for (key, value) in map {
                let py_value = json_to_py(vm, value)?;
                scope.globals.set_item(key.as_str(), py_value, vm)?;
            }
        }
        Ok(())
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
        "3.12 (RustPython 0.4)".to_string()
    }
}

/// Convert JSON to Python object
fn json_to_py(vm: &VirtualMachine, value: &serde_json::Value) -> PyResult<PyObjectRef> {
    match value {
        serde_json::Value::Null => Ok(vm.ctx.none()),
        serde_json::Value::Bool(b) => Ok(vm.ctx.new_bool(*b).into()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(vm.ctx.new_int(i).into())
            } else if let Some(f) = n.as_f64() {
                Ok(vm.ctx.new_float(f).into())
            } else {
                Ok(vm.ctx.none())
            }
        }
        serde_json::Value::String(s) => Ok(vm.ctx.new_str(s.clone()).into()),
        serde_json::Value::Array(arr) => {
            let mut py_list = Vec::new();
            for v in arr {
                py_list.push(json_to_py(vm, v)?);
            }
            Ok(vm.ctx.new_list(py_list).into())
        }
        serde_json::Value::Object(obj) => {
            let py_dict = vm.ctx.new_dict();
            for (k, v) in obj {
                let py_key = vm.ctx.new_str(k.clone());
                let py_value = json_to_py(vm, v)?;
                py_dict.set_item(py_key.as_str(), py_value, vm)?;
            }
            Ok(py_dict.into())
        }
    }
}

/// Convert Python object to JSON
fn py_to_json(vm: &VirtualMachine, value: &PyObjectRef) -> Option<serde_json::Value> {
    // Check for None
    if vm.is_none(value) {
        return None;
    }

    // Check for bool (must come before int check since bool is subclass of int in Python)
    if let Ok(b) = value.clone().try_to_bool(vm) {
        return Some(serde_json::Value::Bool(b));
    }

    // Check for int
    if let Ok(i) = value.clone().try_int(vm) {
        if let Ok(n) = i.try_to_primitive::<i64>(vm) {
            return Some(serde_json::Value::Number(serde_json::Number::from(n)));
        }
    }

    // Check for float
    if let Ok(f) = value.clone().try_float(vm) {
        let f_val = f.to_f64();
        if let Some(n) = serde_json::Number::from_f64(f_val) {
            return Some(serde_json::Value::Number(n));
        }
    }

    // Check for string
    if let Ok(s) = value.str(vm) {
        return Some(serde_json::Value::String(s.as_str().to_string()));
    }

    // Check for list
    if let Ok(list) = value.clone().downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.borrow_vec().iter() {
            arr.push(py_to_json(vm, item).unwrap_or(serde_json::Value::Null));
        }
        return Some(serde_json::Value::Array(arr));
    }

    // Check for dict
    if let Ok(dict) = value.clone().downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.into_iter() {
            if let Ok(key_str) = k.str(vm) {
                let json_v = py_to_json(vm, &v).unwrap_or(serde_json::Value::Null);
                map.insert(key_str.as_str().to_string(), json_v);
            }
        }
        return Some(serde_json::Value::Object(map));
    }

    // Default: convert to string representation
    if let Ok(s) = value.repr(vm) {
        return Some(serde_json::Value::String(s.as_str().to_string()));
    }

    None
}

/// Format Python exception for display
fn format_python_error(vm: &VirtualMachine, exc: &PyRef<PyBaseException>) -> String {
    // Try to get exception type and message
    let exc_type = exc.class().name().to_string();

    // Try to get the exception message from args
    let args = exc.args();
    if !args.is_empty() {
        if let Some(first_arg) = args.first() {
            if let Ok(msg) = first_arg.str(vm) {
                return format!("{}: {}", exc_type, msg.as_str());
            }
        }
    }

    // Fallback to just the type
    exc_type
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Language;

    fn make_request(code: &str) -> ExecutionRequest {
        ExecutionRequest {
            language: Language::Python,
            code: code.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_simple_expression() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request("print(1 + 2)"));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("3"));
    }

    #[test]
    fn test_print() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(r#"print("Hello, World!")"#));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("Hello, World!"));
    }

    #[test]
    fn test_variables() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
x = 10
y = 20
print(x + y)
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("30"));
    }

    #[test]
    fn test_loop() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
total = 0
for i in range(10):
    total += i
print(total)
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("45")); // Sum of 0..9
    }

    #[test]
    fn test_list() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
arr = [1, 2, 3, 4, 5]
print(len(arr))
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("5"));
    }

    #[test]
    fn test_dict() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
d = {"name": "test", "value": 42}
print(d["value"])
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("42"));
    }

    #[test]
    fn test_function() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
def add(a, b):
    return a + b

print(add(3, 4))
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("7"));
    }

    #[test]
    fn test_syntax_error() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request("def incomplete("));
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_runtime_error() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request("undefined_variable"));
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("NameError"));
    }

    #[test]
    fn test_context_injection() {
        let executor = PythonExecutor::new();
        let mut request = make_request("print(x + y)");
        request.context = Some(serde_json::json!({
            "x": 10,
            "y": 20
        }));
        let result = executor.execute(&request);
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("30"));
    }

    #[test]
    fn test_list_comprehension() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
squares = [x**2 for x in range(5)]
print(squares)
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("[0, 1, 4, 9, 16]"));
    }

    #[test]
    fn test_string_methods() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
s = "hello world"
print(s.upper())
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("HELLO WORLD"));
    }

    #[test]
    fn test_math_import() {
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
import math
print(math.sqrt(16))
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        assert!(result.stdout.contains("4"));
    }

    #[test]
    fn test_repr_function() {
        // Test built-in repr function instead of json module
        // since json may not be available in all RustPython builds
        let executor = PythonExecutor::new();
        let result = executor.execute(&make_request(
            r#"
data = {"a": 1, "b": 2}
print(repr(data))
"#,
        ));
        assert!(result.success, "Error: {:?}", result.error);
        // Should contain dict representation
        assert!(result.stdout.contains("a") && result.stdout.contains("b"));
    }
}
