use async_trait::async_trait;
use brainwires_hardware::audio::{
    assistant::VoiceAssistantHandler,
    error::AudioError,
    types::Transcript,
};
use brainwires_providers::openai_chat::{
    OpenAiClient, OpenAIMessage, OpenAIContent, OpenAiRequestOptions,
};
use std::sync::Arc;
use tracing::{info, warn};

#[cfg(any(
    feature = "wake-word",
    feature = "wake-word-rustpotter",
    feature = "wake-word-porcupine"
))]
use brainwires_hardware::audio::wake_word::WakeWordDetection;

/// Handler that forwards transcripts to an LLM and returns the response text.
pub struct LlmHandler {
    client: Arc<OpenAiClient>,
    model: String,
    system_prompt: String,
}

impl LlmHandler {
    pub fn new(client: Arc<OpenAiClient>, model: String, system_prompt: String) -> Self {
        Self { client, model, system_prompt }
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

        let messages = vec![
            OpenAIMessage {
                role: "system".into(),
                content: OpenAIContent::Text(self.system_prompt.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            OpenAIMessage {
                role: "user".into(),
                content: OpenAIContent::Text(text.to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let opts = OpenAiRequestOptions::default();

        match self.client.chat_completions(&messages, &self.model, None, &opts).await {
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
