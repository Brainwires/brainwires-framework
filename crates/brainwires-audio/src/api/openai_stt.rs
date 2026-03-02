use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::error::{AudioError, AudioResult};
use crate::stt::SpeechToText;
use crate::types::{AudioBuffer, SttOptions, Transcript, TranscriptSegment};
use crate::wav::encode_wav;

/// OpenAI Whisper API speech-to-text implementation.
pub struct OpenAiStt {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiStt {
    /// Create a new OpenAI STT client.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "whisper-1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set a custom base URL (for compatible APIs).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl SpeechToText for OpenAiStt {
    fn name(&self) -> &str {
        "openai-whisper"
    }

    async fn transcribe(
        &self,
        audio: &AudioBuffer,
        options: &SttOptions,
    ) -> AudioResult<Transcript> {
        let wav_data = encode_wav(audio)?;

        let file_part = reqwest::multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AudioError::Api(format!("failed to create multipart: {e}")))?;

        let mut form = reqwest::multipart::Form::new()
            .text("model", self.model.clone())
            .part("file", file_part);

        if let Some(lang) = &options.language {
            form = form.text("language", lang.clone());
        }
        if let Some(prompt) = &options.prompt {
            form = form.text("prompt", prompt.clone());
        }
        if options.timestamps {
            form = form.text("response_format", "verbose_json");
            form = form.text("timestamp_granularities[]", "segment");
        } else {
            form = form.text("response_format", "verbose_json");
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
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

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AudioError::Api(format!("failed to parse response: {e}")))?;

        let text = body["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let language = body["language"].as_str().map(|s| s.to_string());
        let duration_secs = body["duration"].as_f64();

        let segments = if options.timestamps {
            body["segments"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|seg| {
                            Some(TranscriptSegment {
                                text: seg["text"].as_str()?.to_string(),
                                start: seg["start"].as_f64()?,
                                end: seg["end"].as_f64()?,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Transcript {
            text,
            language,
            duration_secs,
            segments,
        })
    }

    fn transcribe_stream(
        &self,
        audio_stream: BoxStream<'static, AudioResult<AudioBuffer>>,
        options: &SttOptions,
    ) -> BoxStream<'static, AudioResult<Transcript>> {
        // OpenAI Whisper API doesn't support streaming input natively.
        // Buffer all audio, then transcribe as a single request.
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();
        let options = options.clone();

        let stream = async_stream::stream! {
            use futures::StreamExt;

            let mut all_data = Vec::new();
            let mut config = None;
            let mut audio_stream = audio_stream;

            while let Some(result) = audio_stream.next().await {
                match result {
                    Ok(buffer) => {
                        if config.is_none() {
                            config = Some(buffer.config.clone());
                        }
                        all_data.extend_from_slice(&buffer.data);
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }

            if let Some(cfg) = config {
                let full_buffer = AudioBuffer::from_pcm(all_data, cfg);
                let stt = OpenAiStt {
                    api_key,
                    base_url,
                    model,
                    client: reqwest::Client::new(),
                };
                yield stt.transcribe(&full_buffer, &options).await;
            }
        };

        Box::pin(stream)
    }
}
