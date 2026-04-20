//! Minimal demonstration of [`SandboxedToolExecutor`].
//!
//! Wraps a [`BuiltinToolExecutor`] in a [`SandboxedToolExecutor`] backed by a
//! canned in-process sandbox double and shows that `execute_command` calls
//! are routed through the sandbox while the inner executor is bypassed.
//!
//! Run:
//!   cargo run -p brainwires-tools --example sandboxed_executor --features sandbox

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use brainwires_core::{ToolContext, ToolUse};
use brainwires_sandbox::{
    ExecHandle, ExecOutput, ExecSpec, Sandbox, SandboxPolicy, SandboxRuntime,
};
use brainwires_tools::{BuiltinToolExecutor, SandboxedToolExecutor, ToolExecutor, ToolRegistry};

/// Canned Sandbox implementation used in place of a real Docker daemon.
struct MockSandbox;

#[async_trait]
impl Sandbox for MockSandbox {
    async fn spawn(&self, _spec: ExecSpec) -> brainwires_sandbox::Result<ExecHandle> {
        Ok(ExecHandle::new())
    }

    async fn wait(&self, _handle: ExecHandle) -> brainwires_sandbox::Result<ExecOutput> {
        Ok(ExecOutput {
            exit_code: 0,
            stdout: b"mock".to_vec(),
            stderr: vec![],
            wall_time: Duration::from_millis(1),
        })
    }

    async fn shutdown(&self) -> brainwires_sandbox::Result<()> {
        Ok(())
    }

    fn runtime(&self) -> SandboxRuntime {
        SandboxRuntime::Host
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = ToolRegistry::with_builtins();
    let builtin = BuiltinToolExecutor::new(registry, ToolContext::default());

    let exec = SandboxedToolExecutor::new(
        builtin,
        Arc::new(MockSandbox) as Arc<dyn Sandbox>,
        SandboxPolicy::default(),
    )
    .with_timeout(Duration::from_secs(10));

    let tool_use = ToolUse {
        id: "demo-1".to_string(),
        name: "execute_command".to_string(),
        input: json!({ "command": "echo hello from the sandbox" }),
    };

    let ctx = ToolContext::default();
    let result = exec.execute(&tool_use, &ctx).await?;

    println!("is_error: {}", result.is_error);
    println!("content:  {}", result.content);
    println!("policy runtime: {:?}", exec.policy().runtime);

    Ok(())
}
