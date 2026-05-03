//! LLM-powered conversation summariser for the dream consolidation pipeline.

use anyhow::Result;

use brainwires_core::{ChatOptions, Message, Provider};

/// Stateless helper that calls an LLM to summarise a batch of messages.
pub struct DreamSummarizer;

impl DreamSummarizer {
    /// Summarise the given conversation messages into a concise text.
    ///
    /// The prompt instructs the LLM to:
    /// - Preserve key decisions, tool outcomes, and user preferences
    /// - Convert relative dates to absolute where possible
    /// - Keep the summary concise but complete
    pub async fn summarize_messages(
        messages: &[Message],
        provider: &dyn Provider,
    ) -> Result<String> {
        if messages.is_empty() {
            return Ok(String::new());
        }

        // Build a text representation of the messages
        let mut conversation_text = String::new();
        for msg in messages {
            let role = match msg.role {
                brainwires_core::Role::User => "User",
                brainwires_core::Role::Assistant => "Assistant",
                brainwires_core::Role::System => "System",
                brainwires_core::Role::Tool => "Tool",
            };
            let text = msg.text_or_summary();
            conversation_text.push_str(&format!("{role}: {text}\n\n"));
        }

        let prompt = format!(
            "Synthesize the following conversation into a concise summary. \
             Preserve key decisions, tool outcomes, and user preferences. \
             Convert relative dates to absolute where possible. \
             Focus on information that would be useful for future interactions.\n\n\
             Conversation:\n{conversation_text}\n\n\
             Summary:"
        );

        let llm_messages = vec![Message::user(&prompt)];
        let options = ChatOptions {
            temperature: Some(0.3),
            max_tokens: Some(1024),
            ..Default::default()
        };

        let response = provider.chat(&llm_messages, None, &options).await?;
        Ok(response.message.text_or_summary())
    }
}
