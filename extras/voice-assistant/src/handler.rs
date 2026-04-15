use async_trait::async_trait;
use brainwires_hardware::audio::{
    assistant::VoiceAssistantHandler, error::AudioError, types::Transcript,
};
use brainwires_providers::openai_chat::{
    OpenAIContent, OpenAIMessage, OpenAiClient, OpenAiRequestOptions,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[cfg(any(
    feature = "wake-word",
    feature = "wake-word-rustpotter",
    feature = "wake-word-porcupine"
))]
use brainwires_hardware::audio::wake_word::WakeWordDetection;

/// Handler that forwards transcripts to an LLM and returns the response text.
///
/// Maintains full multi-turn conversation history across calls so the assistant
/// can reference earlier messages in the session.
pub struct LlmHandler {
    client: Arc<OpenAiClient>,
    model: String,
    system_prompt: String,
    /// Accumulated conversation history (user + assistant turns only; system
    /// prompt is prepended fresh on every request so it is never stale).
    history: Mutex<Vec<OpenAIMessage>>,
}

impl LlmHandler {
    pub fn new(client: Arc<OpenAiClient>, model: String, system_prompt: String) -> Self {
        Self {
            client,
            model,
            system_prompt,
            history: Mutex::new(Vec::new()),
        }
    }

    /// Clear the conversation history (useful after a long pause or explicit reset).
    #[allow(dead_code)]
    pub async fn clear_history(&self) {
        self.history.lock().await.clear();
    }
}

#[async_trait]
impl VoiceAssistantHandler for LlmHandler {
    #[cfg(any(
        feature = "wake-word",
        feature = "wake-word-rustpotter",
        feature = "wake-word-porcupine"
    ))]
    async fn on_wake_word(&self, detection: &WakeWordDetection) {
        info!(
            keyword = %detection.keyword,
            score = detection.score,
            "Wake word detected — listening…"
        );
    }

    async fn on_speech(&self, transcript: &Transcript) -> Option<String> {
        let text = transcript.text.trim();
        if text.is_empty() {
            return None;
        }

        info!("You: {text}");

        let user_msg = OpenAIMessage {
            role: "user".into(),
            content: OpenAIContent::Text(text.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        // Build full message list: system + history + new user turn
        let messages = {
            let history = self.history.lock().await;
            let mut msgs = Vec::with_capacity(history.len() + 2);
            msgs.push(OpenAIMessage {
                role: "system".into(),
                content: OpenAIContent::Text(self.system_prompt.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
            msgs.extend(history.iter().cloned());
            msgs.push(user_msg.clone());
            msgs
        };

        let opts = OpenAiRequestOptions::default();

        match self
            .client
            .chat_completions(&messages, &self.model, None, &opts)
            .await
        {
            Ok(response) => {
                let reply = response
                    .choices
                    .into_iter()
                    .next()
                    .map(|c| match c.message.content {
                        OpenAIContent::Text(s) => s,
                        OpenAIContent::Array(_) => String::new(),
                    })
                    .unwrap_or_default();

                if !reply.is_empty() {
                    info!("Assistant: {reply}");

                    // Persist this turn to history
                    let mut history = self.history.lock().await;
                    history.push(user_msg);
                    history.push(OpenAIMessage {
                        role: "assistant".into(),
                        content: OpenAIContent::Text(reply.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });

                    Some(reply)
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("LLM error: {e}");
                None
            }
        }
    }

    async fn on_error(&self, error: &AudioError) {
        warn!("Pipeline error: {error}");
    }
}
