//! Chat Stream Processing
//!
//! Handles streaming chat responses with tool execution support.

use anyhow::Result;
use futures::StreamExt;
use indicatif::ProgressBar;
use std::sync::Arc;

use super::continuation::{default_logger, send_continuation_request};
use crate::providers::Provider;
use crate::tools::ToolExecutor;
use crate::types::agent::{AgentContext, PermissionMode};
use crate::types::message::StreamChunk;
use crate::types::provider::ChatOptions;
use crate::types::tool::{ToolContext, ToolContextExt, ToolUse};

/// Process chat stream with tool execution support
pub async fn process_chat_stream(
    provider: &Arc<dyn Provider>,
    context: &AgentContext,
    spinner: &Option<ProgressBar>,
    model: &str,
    chat_id: Option<String>,
) -> Result<String> {
    use crate::types::message::Role;

    let mut full_text = String::new();
    let tool_executor = ToolExecutor::new(PermissionMode::Auto);

    // Extract system prompt from conversation history
    let system_prompt = context
        .conversation_history
        .iter()
        .find(|m| m.role == Role::System)
        .and_then(|m| m.text().map(|s| s.to_string()));

    let options = ChatOptions {
        temperature: Some(0.7),
        max_tokens: Some(4096),
        top_p: None,
        stop: None,
        system: system_prompt,
        model: None,
        cache_strategy: Default::default(),
    };

    let mut stream = provider.stream_chat(
        &context.conversation_history,
        Some(&context.tools),
        &options,
    );

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        match chunk {
            StreamChunk::Text(text) => {
                full_text.push_str(&text);
            }
            StreamChunk::ToolCall {
                call_id,
                response_id,
                chat_id: tool_chat_id,
                tool_name,
                server,
                parameters,
            } => {
                // Tool call received from backend
                if let Some(s) = spinner {
                    s.set_message("Executing tool...");
                }

                eprintln!(
                    "\n🔧 Tool requested: {} (server: {})",
                    console::style(&tool_name).cyan().bold(),
                    console::style(&server).dim()
                );

                // Only execute if it's a cli-local tool
                if server == "cli-local" {
                    // Create ToolUse from the call
                    let tool_use = ToolUse {
                        id: call_id.clone(),
                        name: tool_name.clone(),
                        input: parameters.clone(),
                    };

                    // Execute tool locally
                    let tool_context = ToolContext::from_agent_context(context);

                    let result = tool_executor.execute(&tool_use, &tool_context).await?;

                    // Limit tool output to prevent context window overflow
                    const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;
                    let truncated_output = if result.content.len() > MAX_TOOL_OUTPUT_CHARS {
                        let truncated = &result.content[..MAX_TOOL_OUTPUT_CHARS];
                        let lines_count = result.content.lines().count();
                        let truncated_lines = truncated.lines().count();
                        format!(
                            "{}\n\n[Output truncated: showing first {} of {} lines ({} of {} characters)]",
                            truncated,
                            truncated_lines,
                            lines_count,
                            MAX_TOOL_OUTPUT_CHARS,
                            result.content.len()
                        )
                    } else {
                        result.content.clone()
                    };

                    if result.is_error {
                        eprintln!(
                            "❌ Tool {} failed: {}\n",
                            console::style(&tool_name).red(),
                            console::style(&result.content).dim()
                        );
                    } else {
                        let preview = if truncated_output.len() > 200 {
                            format!("{}...", &truncated_output[..200])
                        } else {
                            truncated_output.clone()
                        };
                        eprintln!(
                            "✅ Tool {} completed: {}\n",
                            console::style(&tool_name).green(),
                            console::style(preview).dim()
                        );
                    }

                    // Send continuation request to backend with tool result
                    if let Some(s) = spinner {
                        s.set_message("Processing tool result...");
                    }

                    let continuation_text = send_continuation_request(
                        provider,
                        context,
                        model,
                        tool_chat_id.or_else(|| chat_id.clone()),
                        &response_id,
                        &call_id,
                        &tool_name,
                        &parameters,
                        &truncated_output,
                        &[], // Empty accumulated history for first tool call
                        default_logger(),
                    )
                    .await?;

                    full_text.push_str(&continuation_text);

                    // Tool execution complete - stop reading from original stream
                    break;
                } else {
                    eprintln!("⚠️  Ignoring tool from unknown server: {}\n", server);
                }
            }
            StreamChunk::Usage(_usage) => {
                // Ignore usage for now
            }
            StreamChunk::Done => {
                break;
            }
            StreamChunk::ToolUse { .. } | StreamChunk::ToolInputDelta { .. } => {
                // These are for other tool formats, ignore
            }
            StreamChunk::ContextCompacted { .. } => {
                // Context compaction is handled by the agent layer
            }
        }
    }

    Ok(full_text)
}
