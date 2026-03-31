//! A simple chat agent that processes messages through an LLM provider with tool support.
//!
//! [`ChatAgent`] is the framework's ready-to-use agent for text message to response
//! flows, including automatic tool call dispatch via [`BuiltinToolExecutor`].

use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;

use brainwires_core::{
    ChatOptions, ContentBlock, Message, MessageContent, Provider, Role, StreamChunk, Tool,
    ToolContext, ToolUse, Usage,
};
use brainwires_tool_system::{BuiltinToolExecutor, PreHookDecision, ToolPreHook};

/// A simple chat agent that processes messages through an LLM provider with tool support.
///
/// This is the framework's ready-to-use agent for text message -> response flows.
/// It manages conversation history, streams responses from the provider, and
/// automatically dispatches tool calls through a [`BuiltinToolExecutor`].
///
/// # Example
///
/// ```rust,ignore
/// use brainwires_agents::ChatAgent;
/// use brainwires_tool_system::{BuiltinToolExecutor, ToolRegistry};
/// use brainwires_core::{ChatOptions, ToolContext};
/// use std::sync::Arc;
///
/// let provider = /* create a provider */;
/// let registry = ToolRegistry::with_builtins();
/// let context = ToolContext::default();
/// let executor = Arc::new(BuiltinToolExecutor::new(registry, context));
/// let options = ChatOptions::default();
///
/// let mut agent = ChatAgent::new(provider, executor, options)
///     .with_system_prompt("You are a helpful assistant.")
///     .with_max_tool_rounds(5);
///
/// let response = agent.process_message("Hello!").await?;
/// println!("{}", response);
/// ```
pub struct ChatAgent {
    provider: Arc<dyn Provider>,
    executor: Arc<BuiltinToolExecutor>,
    messages: Vec<Message>,
    options: ChatOptions,
    max_tool_rounds: usize,
    pre_execute_hook: Option<Arc<dyn ToolPreHook>>,
    /// Accumulated token usage across all completions in this session.
    cumulative_usage: Usage,
}

impl ChatAgent {
    /// Create a new `ChatAgent`.
    ///
    /// Defaults `max_tool_rounds` to 10.
    pub fn new(
        provider: Arc<dyn Provider>,
        executor: Arc<BuiltinToolExecutor>,
        options: ChatOptions,
    ) -> Self {
        Self {
            provider,
            executor,
            messages: Vec::new(),
            options,
            max_tool_rounds: 10,
            pre_execute_hook: None,
            cumulative_usage: Usage::default(),
        }
    }

    /// Set the maximum number of tool-call rounds before the agent stops.
    pub fn with_max_tool_rounds(mut self, rounds: usize) -> Self {
        self.max_tool_rounds = rounds;
        self
    }

    /// Attach a pre-execution hook that can allow or reject tool calls before they run.
    pub fn with_pre_execute_hook(mut self, hook: Arc<dyn ToolPreHook>) -> Self {
        self.pre_execute_hook = Some(hook);
        self
    }

    /// Add a system prompt as the first message in the conversation.
    ///
    /// If messages already exist, the system message is inserted at position 0.
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        // Remove any existing system message at position 0
        if let Some(first) = self.messages.first()
            && first.role == Role::System
        {
            self.messages.remove(0);
        }
        self.messages.insert(0, Message::system(prompt));
        self
    }

    /// Process a user message and return the final assistant text response.
    ///
    /// This is the core completion loop:
    /// 1. Adds the user message to history
    /// 2. Streams the provider response, collecting text and tool calls
    /// 3. If tool calls are present, executes them and loops
    /// 4. Returns the final accumulated text once no more tool calls remain
    ///    (or `max_tool_rounds` is reached)
    pub async fn process_message(&mut self, input: &str) -> Result<String> {
        self.messages.push(Message::user(input));
        self.run_completion(None::<fn(&str)>).await
    }

    /// Process a user message with streaming — calls `on_chunk` for each text
    /// fragment as it arrives from the provider.
    ///
    /// Returns the full accumulated text once the completion loop finishes.
    pub async fn process_message_streaming<F>(&mut self, input: &str, on_chunk: F) -> Result<String>
    where
        F: Fn(&str) + Send + Sync,
    {
        self.messages.push(Message::user(input));
        self.run_completion(Some(on_chunk)).await
    }

    /// Access the conversation history.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Replace the entire message history with the provided messages.
    ///
    /// This is used by session persistence to restore a previously saved
    /// conversation when an agent session is recreated.
    pub fn restore_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    /// Clear all messages (including any system prompt).
    pub fn clear_history(&mut self) {
        self.messages.clear();
    }

    /// Keep only the last `max_messages` messages, preserving the system prompt
    /// at position 0 if one exists.
    pub fn trim_history(&mut self, max_messages: usize) {
        if self.messages.len() <= max_messages {
            return;
        }

        let has_system = self
            .messages
            .first()
            .map(|m| m.role == Role::System)
            .unwrap_or(false);

        if has_system && max_messages > 0 {
            let system = self.messages.remove(0);
            let keep = max_messages.saturating_sub(1);
            let start = self.messages.len().saturating_sub(keep);
            self.messages = std::iter::once(system)
                .chain(self.messages.drain(start..))
                .collect();
        } else {
            let start = self.messages.len().saturating_sub(max_messages);
            self.messages = self.messages.drain(start..).collect();
        }
    }

    /// Return the number of messages in the conversation.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Return the accumulated token usage for this agent session.
    ///
    /// Counts prompt + completion tokens across all completions. Updated
    /// whenever the provider emits a `StreamChunk::Usage` event.
    pub fn cumulative_usage(&self) -> &Usage {
        &self.cumulative_usage
    }

    /// Reset the cumulative token usage counter.
    pub fn reset_usage(&mut self) {
        self.cumulative_usage = Usage::default();
    }

    /// Compact conversation history by trimming older messages.
    ///
    /// This is a simple, LLM-free compaction that keeps the system prompt
    /// (if any) and the most recent `keep` messages. For LLM-powered
    /// summarisation, use the `DreamSummarizer` from `brainwires-autonomy`.
    pub async fn compact_history(&mut self) -> Result<()> {
        // Default: keep system prompt + last 20 messages
        self.trim_history(20);
        Ok(())
    }

    // ── Internal completion loop ─────────────────────────────────────────

    async fn run_completion<F>(&mut self, on_chunk: Option<F>) -> Result<String>
    where
        F: Fn(&str) + Send + Sync,
    {
        let mut final_text = String::new();

        for _ in 0..self.max_tool_rounds {
            let tool_defs: Vec<Tool> = self.executor.tools();
            let tools_opt = if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs.as_slice())
            };

            let (text_buf, tool_uses, response_id) =
                self.collect_stream(tools_opt, &on_chunk).await?;

            if tool_uses.is_empty() {
                // No tool calls — this is the final response
                self.messages.push(Message::assistant(&text_buf));
                final_text = text_buf;
                break;
            }

            // Build assistant message with text + tool use blocks
            let mut blocks = Vec::new();
            if !text_buf.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: text_buf.clone(),
                });
            }
            for tu in &tool_uses {
                blocks.push(ContentBlock::ToolUse {
                    id: tu.id.clone(),
                    name: tu.name.clone(),
                    input: tu.input.clone(),
                });
            }
            let metadata = response_id.map(|rid| serde_json::json!({"response_id": rid}));
            self.messages.push(Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(blocks),
                name: None,
                metadata,
            });

            // Execute each tool call and add results as a user message
            let mut result_blocks = Vec::new();
            for tu in &tool_uses {
                // Run pre-execute hook if configured
                if let Some(ref hook) = self.pre_execute_hook {
                    let ctx = ToolContext::default();
                    match hook.before_execute(tu, &ctx).await {
                        Ok(PreHookDecision::Allow) => {}
                        Ok(PreHookDecision::Reject(reason)) => {
                            result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: tu.id.clone(),
                                content: reason,
                                is_error: Some(true),
                            });
                            continue;
                        }
                        Err(e) => {
                            tracing::warn!(tool = %tu.name, error = %e, "Pre-execute hook error");
                        }
                    }
                }

                let result = self
                    .executor
                    .execute_tool(&tu.name, &tu.id, &tu.input)
                    .await;
                result_blocks.push(ContentBlock::ToolResult {
                    tool_use_id: tu.id.clone(),
                    content: result.content,
                    is_error: Some(result.is_error),
                });
            }

            self.messages.push(Message {
                role: Role::User,
                content: MessageContent::Blocks(result_blocks),
                name: None,
                metadata: None,
            });

            // Keep the last text in case we hit max rounds
            final_text = text_buf;
        }

        Ok(final_text)
    }

    /// Collect the stream into accumulated text + tool uses.
    async fn collect_stream<F>(
        &mut self,
        tools_opt: Option<&[Tool]>,
        on_chunk: &Option<F>,
    ) -> Result<(String, Vec<ToolUse>, Option<String>)>
    where
        F: Fn(&str) + Send + Sync,
    {
        let mut stream = self
            .provider
            .stream_chat(&self.messages, tools_opt, &self.options);

        let mut text_buf = String::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_input = String::new();
        let mut last_response_id: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            match chunk? {
                StreamChunk::Text(t) => {
                    if let Some(cb) = on_chunk {
                        cb(&t);
                    }
                    text_buf.push_str(&t);
                }
                StreamChunk::ToolUse { id, name } => {
                    // Flush previous tool if any
                    if !current_tool_id.is_empty() {
                        let input: serde_json::Value = serde_json::from_str(&current_tool_input)
                            .unwrap_or(serde_json::Value::Null);
                        tool_uses.push(ToolUse {
                            id: std::mem::take(&mut current_tool_id),
                            name: std::mem::take(&mut current_tool_name),
                            input,
                        });
                        current_tool_input.clear();
                    }
                    current_tool_id = id;
                    current_tool_name = name;
                }
                StreamChunk::ToolInputDelta { partial_json, .. } => {
                    current_tool_input.push_str(&partial_json);
                }
                StreamChunk::ToolCall {
                    call_id,
                    response_id,
                    tool_name,
                    parameters,
                    ..
                } => {
                    last_response_id = Some(response_id);
                    tool_uses.push(ToolUse {
                        id: call_id,
                        name: tool_name,
                        input: parameters,
                    });
                }
                StreamChunk::Usage(u) => {
                    self.cumulative_usage.prompt_tokens += u.prompt_tokens;
                    self.cumulative_usage.completion_tokens += u.completion_tokens;
                    self.cumulative_usage.total_tokens += u.total_tokens;
                }
                StreamChunk::Done => {}
            }
        }

        // Flush last tool if any
        if !current_tool_id.is_empty() {
            let input: serde_json::Value =
                serde_json::from_str(&current_tool_input).unwrap_or(serde_json::Value::Null);
            tool_uses.push(ToolUse {
                id: current_tool_id,
                name: current_tool_name,
                input,
            });
        }

        Ok((text_buf, tool_uses, last_response_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::{ToolContext, ToolInputSchema};
    use brainwires_tool_system::ToolRegistry;
    use futures::stream;
    use std::collections::HashMap;

    /// A mock provider that returns a simple text response.
    struct MockProvider {
        response_text: String,
    }

    impl MockProvider {
        fn new(text: &str) -> Self {
            Self {
                response_text: text.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: &[Message],
            _tools: Option<&[Tool]>,
            _options: &ChatOptions,
        ) -> Result<brainwires_core::ChatResponse> {
            Ok(brainwires_core::ChatResponse {
                message: Message::assistant(&self.response_text),
                usage: brainwires_core::Usage::new(10, 20),
                finish_reason: Some("stop".to_string()),
            })
        }

        fn stream_chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: Option<&'a [Tool]>,
            _options: &'a ChatOptions,
        ) -> futures::stream::BoxStream<'a, Result<StreamChunk>> {
            let text = self.response_text.clone();
            Box::pin(stream::iter(vec![
                Ok(StreamChunk::Text(text)),
                Ok(StreamChunk::Done),
            ]))
        }
    }

    fn make_executor() -> Arc<BuiltinToolExecutor> {
        let mut registry = ToolRegistry::new();
        registry.register(Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            ..Default::default()
        });
        let context = ToolContext::default();
        Arc::new(BuiltinToolExecutor::new(registry, context))
    }

    fn make_agent() -> ChatAgent {
        let provider = Arc::new(MockProvider::new("Hello from mock!"));
        let executor = make_executor();
        ChatAgent::new(provider, executor, ChatOptions::default())
    }

    #[test]
    fn test_new_creates_successfully() {
        let agent = make_agent();
        assert_eq!(agent.message_count(), 0);
        assert_eq!(agent.max_tool_rounds, 10);
    }

    #[test]
    fn test_with_system_prompt_adds_system_message() {
        let agent = make_agent().with_system_prompt("You are helpful.");
        assert_eq!(agent.message_count(), 1);
        assert_eq!(agent.messages()[0].role, Role::System);
        assert_eq!(agent.messages()[0].text(), Some("You are helpful."));
    }

    #[test]
    fn test_with_system_prompt_replaces_existing() {
        let agent = make_agent()
            .with_system_prompt("First prompt")
            .with_system_prompt("Second prompt");
        assert_eq!(agent.message_count(), 1);
        assert_eq!(agent.messages()[0].text(), Some("Second prompt"));
    }

    #[test]
    fn test_with_max_tool_rounds() {
        let agent = make_agent().with_max_tool_rounds(5);
        assert_eq!(agent.max_tool_rounds, 5);
    }

    #[test]
    fn test_messages_returns_history() {
        let mut agent = make_agent();
        assert!(agent.messages().is_empty());
        // Manually push to test accessor
        agent.messages.push(Message::user("test"));
        assert_eq!(agent.messages().len(), 1);
    }

    #[test]
    fn test_clear_history() {
        let mut agent = make_agent().with_system_prompt("sys");
        agent.messages.push(Message::user("hello"));
        assert_eq!(agent.message_count(), 2);
        agent.clear_history();
        assert_eq!(agent.message_count(), 0);
    }

    #[test]
    fn test_trim_history_no_system() {
        let mut agent = make_agent();
        for i in 0..10 {
            agent.messages.push(Message::user(format!("msg {}", i)));
        }
        assert_eq!(agent.message_count(), 10);
        agent.trim_history(3);
        assert_eq!(agent.message_count(), 3);
        // Should keep the last 3
        assert_eq!(agent.messages()[0].text(), Some("msg 7"));
        assert_eq!(agent.messages()[1].text(), Some("msg 8"));
        assert_eq!(agent.messages()[2].text(), Some("msg 9"));
    }

    #[test]
    fn test_trim_history_preserves_system() {
        let mut agent = make_agent().with_system_prompt("system prompt");
        for i in 0..10 {
            agent.messages.push(Message::user(format!("msg {}", i)));
        }
        assert_eq!(agent.message_count(), 11); // 1 system + 10 user
        agent.trim_history(4);
        assert_eq!(agent.message_count(), 4);
        assert_eq!(agent.messages()[0].role, Role::System);
        assert_eq!(agent.messages()[0].text(), Some("system prompt"));
        // Last 3 user messages
        assert_eq!(agent.messages()[1].text(), Some("msg 7"));
        assert_eq!(agent.messages()[2].text(), Some("msg 8"));
        assert_eq!(agent.messages()[3].text(), Some("msg 9"));
    }

    #[test]
    fn test_trim_history_under_limit_is_noop() {
        let mut agent = make_agent();
        agent.messages.push(Message::user("only one"));
        agent.trim_history(10);
        assert_eq!(agent.message_count(), 1);
    }

    #[test]
    fn test_message_count() {
        let mut agent = make_agent();
        assert_eq!(agent.message_count(), 0);
        agent.messages.push(Message::user("a"));
        assert_eq!(agent.message_count(), 1);
        agent.messages.push(Message::assistant("b"));
        assert_eq!(agent.message_count(), 2);
    }

    #[tokio::test]
    async fn test_process_message_returns_text() {
        let mut agent = make_agent();
        let result = agent.process_message("Hi").await.unwrap();
        assert_eq!(result, "Hello from mock!");
        // Should have user message + assistant response
        assert_eq!(agent.message_count(), 2);
        assert_eq!(agent.messages()[0].role, Role::User);
        assert_eq!(agent.messages()[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_process_message_streaming() {
        let mut agent = make_agent();
        let chunks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let chunks_clone = chunks.clone();

        let result = agent
            .process_message_streaming("Hi", move |chunk| {
                chunks_clone.lock().unwrap().push(chunk.to_string());
            })
            .await
            .unwrap();

        assert_eq!(result, "Hello from mock!");
        let received = chunks.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0], "Hello from mock!");
    }
}
