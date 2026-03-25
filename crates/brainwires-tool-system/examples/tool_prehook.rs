//! Tool PreHook demonstration.
//!
//! Implements the `ToolPreHook` trait to validate or reject tool calls before
//! execution. Shows how `PreHookDecision::Allow` and `PreHookDecision::Reject`
//! control tool execution flow.
//!
//! Run:
//!   cargo run -p brainwires-tool-system --example tool_prehook

use async_trait::async_trait;
use brainwires_tool_system::{PreHookDecision, ToolContext, ToolPreHook};
use brainwires_core::ToolUse;
use serde_json::json;

// ── 1. Define a safety-check hook ───────────────────────────────────────────
// This hook blocks destructive tools (delete, deploy) and rejects calls with
// suspicious input patterns.

/// A pre-execution hook that enforces safety policies.
struct SafetyGuardHook {
    /// Tool names that are always blocked.
    blocked_tools: Vec<String>,
    /// Substring patterns that, if found in input, cause rejection.
    blocked_patterns: Vec<String>,
}

impl SafetyGuardHook {
    fn new() -> Self {
        Self {
            blocked_tools: vec![
                "delete_file".to_string(),
                "deploy_production".to_string(),
            ],
            blocked_patterns: vec![
                "rm -rf".to_string(),
                "DROP TABLE".to_string(),
            ],
        }
    }
}

#[async_trait]
impl ToolPreHook for SafetyGuardHook {
    async fn before_execute(
        &self,
        tool_use: &ToolUse,
        _context: &ToolContext,
    ) -> anyhow::Result<PreHookDecision> {
        // Check if the tool itself is blocked.
        if self.blocked_tools.contains(&tool_use.name) {
            return Ok(PreHookDecision::Reject(format!(
                "Tool '{}' is blocked by safety policy.",
                tool_use.name
            )));
        }

        // Check if the input contains any blocked patterns.
        let input_str = tool_use.input.to_string();
        for pattern in &self.blocked_patterns {
            if input_str.contains(pattern) {
                return Ok(PreHookDecision::Reject(format!(
                    "Input contains blocked pattern: '{pattern}'"
                )));
            }
        }

        // Otherwise, allow the call to proceed.
        Ok(PreHookDecision::Allow)
    }
}

// ── 2. Define a logging/audit hook ──────────────────────────────────────────

/// A hook that logs every tool call for auditing and always allows execution.
struct AuditLogHook;

#[async_trait]
impl ToolPreHook for AuditLogHook {
    async fn before_execute(
        &self,
        tool_use: &ToolUse,
        context: &ToolContext,
    ) -> anyhow::Result<PreHookDecision> {
        println!(
            "[AUDIT] Tool='{}' id='{}' cwd='{}' input={}",
            tool_use.name,
            tool_use.id,
            context.working_directory,
            tool_use.input,
        );
        Ok(PreHookDecision::Allow)
    }
}

// ── 3. Helper to build a mock ToolUse ───────────────────────────────────────

fn mock_tool_use(name: &str, input: serde_json::Value) -> ToolUse {
    ToolUse {
        id: format!("call_{name}"),
        name: name.to_string(),
        input,
    }
}

fn mock_context() -> ToolContext {
    ToolContext {
        working_directory: "/home/user/project".to_string(),
        user_id: Some("demo-user".to_string()),
        metadata: Default::default(),
        capabilities: None,
        idempotency_registry: None,
        staging_backend: None,
    }
}

// ── 4. Run the demonstration ────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Tool PreHook Example ===\n");

    let safety = SafetyGuardHook::new();
    let audit = AuditLogHook;
    let ctx = mock_context();

    // Scenario A: A safe tool call — should be allowed.
    let safe_call = mock_tool_use(
        "read_file",
        json!({ "path": "/home/user/project/README.md" }),
    );
    let decision = safety.before_execute(&safe_call, &ctx).await?;
    println!("Scenario A (read_file):");
    println!("  Decision: {decision:?}");
    assert_eq!(decision, PreHookDecision::Allow);

    // Also run the audit hook on the same call.
    let _ = audit.before_execute(&safe_call, &ctx).await?;

    // Scenario B: A blocked tool — should be rejected.
    let blocked_call = mock_tool_use(
        "delete_file",
        json!({ "path": "/etc/important.conf" }),
    );
    let decision = safety.before_execute(&blocked_call, &ctx).await?;
    println!("\nScenario B (delete_file):");
    println!("  Decision: {decision:?}");
    assert_eq!(
        decision,
        PreHookDecision::Reject("Tool 'delete_file' is blocked by safety policy.".to_string())
    );

    // Scenario C: Input contains a dangerous pattern — should be rejected.
    let dangerous_input = mock_tool_use(
        "execute_command",
        json!({ "command": "rm -rf /important/data" }),
    );
    let decision = safety.before_execute(&dangerous_input, &ctx).await?;
    println!("\nScenario C (execute_command with 'rm -rf'):");
    println!("  Decision: {decision:?}");
    assert_eq!(
        decision,
        PreHookDecision::Reject("Input contains blocked pattern: 'rm -rf'".to_string())
    );

    // Scenario D: A safe command — should be allowed.
    let safe_cmd = mock_tool_use(
        "execute_command",
        json!({ "command": "ls -la" }),
    );
    let decision = safety.before_execute(&safe_cmd, &ctx).await?;
    println!("\nScenario D (execute_command with 'ls -la'):");
    println!("  Decision: {decision:?}");
    assert_eq!(decision, PreHookDecision::Allow);

    println!("\nAll scenarios passed.");
    Ok(())
}
