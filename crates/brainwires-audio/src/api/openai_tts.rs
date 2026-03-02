use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::error::{AudioError, AudioResult};
use crate::tts::TextToSpeech;
use crate::types::{AudioBuffer, OutputFormat, TtsOptions, Voice};
use crate::wav::decode_wav;

/// OpenAI TTS API text-to-speech implementation.
pub struct OpenAiTts {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiTts {
    /// Create a new OpenAI TTS client.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "tts-1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set a custom base URL (for compatible APIs).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the model name (e.g., "tts-1", "tts-1-hd").
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

fn format_to_string(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Wav => "wav",
        OutputFormat::Mp3 => "mp3",
        OutputFormat::Pcm => "pcm",
        OutputFormat::Opus => "opus",
    }
}

#[async_trait]
impl TextToSpeech for OpenAiTts {
    fn name(&self) -> &str {
        "openai-tts"
    }

    async fn list_voices(&self) -> AudioResult<Vec<Voice>> {
        // OpenAI has fixed voices; return them statically.
        Ok(vec![
            Voice {
                id: "alloy".to_string(),
                name: Some("Alloy".to_string()),
                language: None,
            },
            Voice {
                id: "echo".to_string(),
                name: Some("Echo".to_string()),
                language: None,
            },
            Voice {
                id: "fable".to_string(),
                name: Some("Fable".to_string()),
                language: None,
            },
            Voice {
                id: "onyx".to_string(),
                name: Some("Onyx".to_string()),
                language: None,
            },
            Voice {
                id: "nova".to_string(),
                name: Some("Nova".to_string()),
                language: None,
            },
            Voice {
                id: "shimmer".to_string(),
                name: Some("Shimmer".to_string()),
                language: None,
            },
        ])
    }

    async fn synthesize(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> AudioResult<AudioBuffer> {
        let mut body = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": options.voice.id,
            "response_format": format_to_string(options.output_format),
        });

        if let Some(speed) = options.speed {
            body["speed"] = serde_json::json!(speed);
        }

        let response = self
            .client
            .post(format!("{}/audio/speech", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AudioError::Api(format!("request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(AudioError::Api(format!("API error {status}: {body}")));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AudioError::Api(format!("failed to read response body: {e}")))?;

        match options.output_format {
            OutputFormat::Wav => decode_wav(&bytes),
            OutputFormat::Pcm => {
                // OpenAI PCM output is 24kHz mono 16-bit
                let config = crate::types::AudioConfig {
                    sample_rate: 24000,
                    channels: 1,
                    sample_format: crate::types::SampleFormat::I16,
                };
                Ok(AudioBuffer::from_pcm(bytes.to_vec(), config))
            }
            _ => {
                // For mp3/opus, return raw bytes with a default config.
                // The caller would need to decode these formats externally.
                Err(AudioError::Unsupported(format!(
                    "direct decoding of {:?} not supported; use Wav or Pcm format",
                    options.output_format
                )))
            }
        }
    }

    fn synthesize_stream(
        &self,
        text: &str,
        options: &TtsOptions,
    ) -> BoxStream<'static, AudioResult<AudioBuffer>> {
        // For streaming TTS, we request the full audio and yield it as one chunk.
        // True streaming would require chunking the response bytes, but the OpenAI
        // TTS API returns the complete audio in one response.
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();
        let text = text.to_string();
        let options = options.clone();

        let stream = async_stream::stream! {
            let tts = OpenAiTts {
                api_key,
                base_url,
                model,
                client: reqwest::Client::new(),
            };
            yield tts.synthesize(&text, &options).await;
        };

        Box::pin(stream)
    }
}
