//! Tool Handler
//!
//! Handles tool execution and continuation requests.

use super::super::state::App;
use crate::cli::chat::continuation::{send_continuation_request, LogCallback};
use crate::types::agent::AgentContext;
use crate::types::message::Message;
use crate::types::tool::{ToolContext, ToolUse};
use std::sync::Arc;
use tokio::sync::mpsc;

impl App {
    /// Handle a tool call and return the continuation text
    pub(super) async fn handle_tool_call(
        &mut self,
        call_id: &str,
        response_id: &str,
        tool_chat_id: Option<String>,
        tool_name: &str,
        parameters: &serde_json::Value,
        conversation_clone: &[Message],
    ) -> Option<String> {
        // Execute the tool
        let tool_use = ToolUse {
            id: call_id.to_string(),
            name: tool_name.to_string(),
            input: parameters.clone(),
        };

        let tool_context = ToolContext {
            working_directory: self.working_directory.clone(),
            // Use full_access for TUI mode - users expect agents to have write access
            capabilities: serde_json::to_value(&brainwires::permissions::AgentCapabilities::full_access()).ok(),
            ..Default::default()
        };

        match self.tool_executor.execute(&tool_use, &tool_context).await {
            Ok(result) => {
                // Limit tool output to prevent context window overflow
                const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;
                let truncated_output = if result.content.len() > MAX_TOOL_OUTPUT_CHARS {
                    let truncated = &result.content[..MAX_TOOL_OUTPUT_CHARS];
                    format!(
                        "{}\n\n[Output truncated: {} of {} characters]",
                        truncated,
                        MAX_TOOL_OUTPUT_CHARS,
                        result.content.len()
                    )
                } else {
                    result.content.clone()
                };

                // PKS: Record tool usage for behavioral inference
                self.pks_integration.record_tool_usage(tool_name, !result.is_error);

                if result.is_error {
                    self.add_console_message(format!("❌ Tool {} failed: {}", tool_name, &result.content[..result.content.len().min(200)]));
                } else {
                    let preview = if truncated_output.len() > 200 {
                        format!("{}...", &truncated_output[..200])
                    } else {
                        truncated_output.clone()
                    };
                    self.add_console_message(format!("✅ Tool {} completed: {}", tool_name, preview));

                    // Update session task cache if task_list_write was executed
                    if tool_name == "task_list_write" {
                        self.update_session_task_cache().await;
                    }
                }

                // Build agent context for continuation
                let agent_context = AgentContext {
                    working_directory: self.working_directory.clone(),
                    user_id: None,
                    conversation_history: conversation_clone.to_vec(),
                    tools: self.tools.clone(),
                    metadata: std::collections::HashMap::new(),
                    working_set: crate::types::WorkingSet::new(),
                    // Use full_access for TUI mode
                    capabilities: brainwires::permissions::AgentCapabilities::full_access(),
                };

                // Create a channel-based logger for TUI console
                let (log_tx, mut log_rx) = mpsc::unbounded_channel::<String>();
                let tui_logger: LogCallback = Arc::new(move |msg: &str| {
                    let _ = log_tx.send(msg.to_string());
                });

                // Send continuation request
                self.status = "Processing tool result...".to_string();
                let continuation_result = send_continuation_request(
                    &self.provider,
                    &agent_context,
                    &self.model,
                    tool_chat_id,
                    response_id,
                    call_id,
                    tool_name,
                    parameters,
                    &truncated_output,  // Tool output (not parameters!)
                    &[],
                    tui_logger,
                ).await;

                // Collect all log messages from the channel
                while let Ok(log_msg) = log_rx.try_recv() {
                    self.add_console_message(log_msg);
                }

                match continuation_result {
                    Ok(continuation_text) => Some(continuation_text),
                    Err(e) => {
                        self.add_console_message(format!("❌ Continuation failed: {}", e));
                        self.status = format!("Error in tool continuation: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                self.add_console_message(format!("❌ Tool execution failed: {}", e));
                self.status = format!("Tool error: {}", e);
                None
            }
        }
    }
}
