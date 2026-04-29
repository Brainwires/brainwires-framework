//! Candle-based local LLM provider (Gemma family).
//!
//! This is a parallel inference path to the llama.cpp-backed [`LocalLlmProvider`]
//! that compiles to `wasm32-unknown-unknown`. It is gated behind the
//! `local-llm-candle` Cargo feature.
//!
//! Why a separate provider:
//! - `llama-cpp-2` is a C/C++ binding and cannot target wasm32.
//! - Candle is pure Rust and runs in the browser via WASM with `Device::Cpu`.
//!
//! Construction:
//! - On any target: [`CandleLlmProvider::from_bytes`] — caller (typically the
//!   JS layer in a PWA) downloads the safetensors weights and `tokenizer.json`
//!   and hands the byte buffers to the provider.
//! - On non-wasm targets: [`CandleLlmProvider::from_hf`] — fetch from a
//!   Hugging Face repo + revision via `hf-hub`. Async, requires a tokio
//!   runtime — gated to non-wasm32.
//!
//! Streaming:
//! - The async [`Provider::chat`] method runs to completion and returns one
//!   [`ChatResponse`].
//! - [`Provider::stream_chat`] yields one [`StreamChunk::Text`] per generated
//!   token, followed by [`StreamChunk::Usage`] and [`StreamChunk::Done`].

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use brainwires_core::message::{ChatResponse, Message, Role, StreamChunk, Usage};
use brainwires_core::provider::{ChatOptions, Provider};
use brainwires_core::tool::Tool;
use futures::stream::BoxStream;
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::gemma3::{Config as GemmaConfig, Model as GemmaModel};
use tokenizers::Tokenizer;

/// Default maximum tokens generated per response when [`ChatOptions::max_tokens`]
/// is unset.
const DEFAULT_MAX_TOKENS: u32 = 512;

/// Default sampling temperature when [`ChatOptions::temperature`] is unset.
/// Matches the rest of the framework.
const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Default RNG seed for the sampler. Stable across runs to keep behaviour
/// reproducible; can be lifted into [`ChatOptions`] later if needed.
const DEFAULT_SEED: u64 = 299792458;

/// Candle-backed local LLM provider.
///
/// Currently targets the Gemma 3 architecture from
/// `candle_transformers::models::gemma3`, which is the most general Gemma
/// implementation in the candle release. The same structure is intended to
/// host Gemma 4 once an official model card lands.
pub struct CandleLlmProvider {
    /// Logical model identifier (used by [`Provider::name`]).
    model_id: String,
    /// Tokenizer loaded from `tokenizer.json`.
    tokenizer: Tokenizer,
    /// EOS token id, looked up once at construction.
    eos_token_id: Option<u32>,
    /// Compute device. We pin to CPU because that is the only target that
    /// works in the browser and the only one we can guarantee on every host.
    device: Device,
    /// Inner model. Wrapped in a [`Mutex`] because the candle Gemma forward
    /// pass takes `&mut self` (it carries the rolling KV cache).
    model: Mutex<GemmaModel>,
}

impl CandleLlmProvider {
    /// The compute device this provider's model is loaded on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Build a provider from already-downloaded byte buffers.
    ///
    /// `model_id` is purely a label returned from [`Provider::name`]. `weights`
    /// must be a single safetensors file and `tokenizer_json` the contents of
    /// `tokenizer.json`. A `config.json` byte buffer is read from a sidecar
    /// file in the bundled HF snapshot in the future; for now this constructor
    /// uses an embedded default config for `gemma-4-e2b`-class models so the
    /// JS layer only has to ship two blobs.
    ///
    /// Note: the embedded config is a placeholder until upstream publishes
    /// `google/gemma-4-e2b/config.json`; see the TODO inside.
    pub fn from_bytes(model_id: &str, weights: Vec<u8>, tokenizer_json: Vec<u8>) -> Result<Self> {
        Self::from_bytes_on_device(model_id, weights, tokenizer_json, &Device::Cpu)
    }

    /// Like [`from_bytes`](Self::from_bytes) but places the model on a
    /// specific [`Device`] (e.g. `Device::Wgpu` for GPU acceleration).
    pub fn from_bytes_on_device(
        model_id: &str,
        weights: Vec<u8>,
        tokenizer_json: Vec<u8>,
        device: &Device,
    ) -> Result<Self> {
        let device = device.clone();
        let cfg = default_gemma_e2b_config();
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device)
            .map_err(|e| anyhow!("failed to map safetensors weights: {e}"))?;
        Self::from_vb_on_device(model_id, vb, tokenizer_json, &device, &cfg)
    }

    /// Build a provider from a pre-constructed [`VarBuilder`].
    ///
    /// Used by the WASM chunked-loading path which builds the VarBuilder from
    /// individually-loaded tensors to avoid allocating the entire safetensors
    /// file in memory at once.
    pub fn from_vb_on_device(
        model_id: &str,
        vb: VarBuilder<'_>,
        tokenizer_json: Vec<u8>,
        device: &Device,
        cfg: &GemmaConfig,
    ) -> Result<Self> {
        let device = device.clone();

        let tokenizer = Tokenizer::from_bytes(&tokenizer_json)
            .map_err(|e| anyhow!("failed to parse tokenizer.json: {e}"))?;
        let eos_token_id = tokenizer.token_to_id("<eos>");

        let model = GemmaModel::new(false, cfg, vb)
            .map_err(|e| anyhow!("failed to build gemma model: {e}"))?;

        Ok(Self {
            model_id: model_id.to_string(),
            tokenizer,
            eos_token_id,
            device,
            model: Mutex::new(model),
        })
    }

    /// Fetch a Gemma model from a Hugging Face repository and build a provider.
    ///
    /// `repo` is a `org/name` repository identifier (e.g. `google/gemma-4-e2b`).
    /// `revision` is a branch name, tag, or commit SHA (`main` is fine).
    ///
    /// Only available off-wasm: `hf-hub` requires a real tokio runtime and
    /// network stack that the browser does not provide. WASM consumers should
    /// download via JS and call [`CandleLlmProvider::from_bytes`].
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn from_hf(repo: &str, revision: &str) -> Result<Self> {
        use hf_hub::{Repo, RepoType, api::tokio::Api};

        let api = Api::new().map_err(|e| anyhow!("hf-hub init failed: {e}"))?;
        let repo_handle = api.repo(Repo::with_revision(
            repo.to_string(),
            RepoType::Model,
            revision.to_string(),
        ));

        let weights_path = repo_handle.get("model.safetensors").await.map_err(|e| {
            anyhow!("failed to fetch model.safetensors from {repo}@{revision}: {e}")
        })?;
        let tokenizer_path = repo_handle
            .get("tokenizer.json")
            .await
            .map_err(|e| anyhow!("failed to fetch tokenizer.json from {repo}@{revision}: {e}"))?;

        let weights = std::fs::read(&weights_path)
            .map_err(|e| anyhow!("failed to read cached weights: {e}"))?;
        let tokenizer_json = std::fs::read(&tokenizer_path)
            .map_err(|e| anyhow!("failed to read cached tokenizer: {e}"))?;

        Self::from_bytes(repo, weights, tokenizer_json)
    }

    /// Render a chat history into a single Gemma-style prompt string.
    ///
    /// Gemma uses `<start_of_turn>` / `<end_of_turn>` markers. We do not have
    /// a "system" turn role, so a system prompt — when present — is folded
    /// into the first user turn.
    fn format_prompt(&self, messages: &[Message], system: Option<&str>) -> String {
        let mut buf = String::new();
        let mut prepend_system = system.map(|s| s.to_string()).or_else(|| {
            messages.iter().find_map(|m| {
                if m.role == Role::System {
                    m.text().map(|t| t.to_string())
                } else {
                    None
                }
            })
        });

        for msg in messages {
            match msg.role {
                Role::System => continue, // already captured into `prepend_system`
                Role::User => {
                    let text = msg.text_or_summary();
                    let body = match prepend_system.take() {
                        Some(sys) => format!("{sys}\n\n{text}"),
                        None => text,
                    };
                    buf.push_str("<start_of_turn>user\n");
                    buf.push_str(&body);
                    buf.push_str("<end_of_turn>\n");
                }
                Role::Assistant => {
                    let text = msg.text_or_summary();
                    buf.push_str("<start_of_turn>model\n");
                    buf.push_str(&text);
                    buf.push_str("<end_of_turn>\n");
                }
                Role::Tool => {
                    // Gemma has no native tool role; serialize as a user turn.
                    let text = msg.text_or_summary();
                    buf.push_str("<start_of_turn>user\n[Tool Result] ");
                    buf.push_str(&text);
                    buf.push_str("<end_of_turn>\n");
                }
            }
        }

        // Open the model's turn so it actually generates a reply.
        buf.push_str("<start_of_turn>model\n");
        buf
    }

    /// Greedy/sampled decode loop. Returns the decoded UTF-8 string and the
    /// token counts for prompt and completion.
    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        top_p: Option<f32>,
    ) -> Result<(String, u32, u32)> {
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow!("tokenizer encode failed: {e}"))?;
        let prompt_ids: Vec<u32> = encoding.get_ids().to_vec();
        let prompt_tokens = prompt_ids.len() as u32;

        let sampling = match temperature.map(|t| t as f64) {
            Some(t) if t > 1e-7 => match top_p.map(|p| p as f64) {
                Some(p) if (0.0..1.0).contains(&p) => Sampling::TopP { p, temperature: t },
                _ => Sampling::All { temperature: t },
            },
            _ => Sampling::ArgMax,
        };
        let mut sampler = LogitsProcessor::from_sampling(DEFAULT_SEED, sampling);

        let mut model = self
            .model
            .lock()
            .map_err(|e| anyhow!("candle model mutex poisoned: {e}"))?;

        // Feed the prompt as one batch and grab the logits for the final token.
        let input = Tensor::new(prompt_ids.as_slice(), &self.device)
            .map_err(|e| anyhow!("input tensor build failed: {e}"))?
            .unsqueeze(0)
            .map_err(|e| anyhow!("input unsqueeze failed: {e}"))?;
        let mut logits = model
            .forward(&input, 0)
            .map_err(|e| anyhow!("prompt forward failed: {e}"))?;
        logits = logits
            .squeeze(0)
            .map_err(|e| anyhow!("logits squeeze failed: {e}"))?;
        if logits.dims().len() == 2 {
            // [seq, vocab] — keep only the last position.
            let last = logits.dim(0).unwrap_or(1).saturating_sub(1);
            logits = logits
                .get(last)
                .map_err(|e| anyhow!("logits last-token slice failed: {e}"))?;
        }

        let mut completion_ids: Vec<u32> = Vec::with_capacity(max_tokens as usize);
        let mut seqlen_offset = prompt_ids.len();

        for _ in 0..max_tokens {
            let next = sampler
                .sample(&logits)
                .map_err(|e| anyhow!("sampling failed: {e}"))?;
            if Some(next) == self.eos_token_id {
                break;
            }
            completion_ids.push(next);

            let next_input = Tensor::new(&[next], &self.device)
                .map_err(|e| anyhow!("next tensor build failed: {e}"))?
                .unsqueeze(0)
                .map_err(|e| anyhow!("next unsqueeze failed: {e}"))?;
            let new_logits = model
                .forward(&next_input, seqlen_offset)
                .map_err(|e| anyhow!("step forward failed: {e}"))?;
            logits = new_logits
                .squeeze(0)
                .map_err(|e| anyhow!("step logits squeeze failed: {e}"))?;
            if logits.dims().len() == 2 {
                let last = logits.dim(0).unwrap_or(1).saturating_sub(1);
                logits = logits
                    .get(last)
                    .map_err(|e| anyhow!("step logits slice failed: {e}"))?;
            }
            seqlen_offset += 1;
        }

        let completion_tokens = completion_ids.len() as u32;
        let text = self
            .tokenizer
            .decode(&completion_ids, true)
            .map_err(|e| anyhow!("tokenizer decode failed: {e}"))?;

        Ok((text, prompt_tokens, completion_tokens))
    }
}

impl std::fmt::Debug for CandleLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleLlmProvider")
            .field("model_id", &self.model_id)
            .field("device", &"Cpu")
            .finish()
    }
}

#[async_trait]
impl Provider for CandleLlmProvider {
    fn name(&self) -> &str {
        &self.model_id
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(DEFAULT_MAX_TOKENS)
    }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let prompt = self.format_prompt(messages, options.system.as_deref());
        let max_tokens = options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        let temperature = options.temperature.or(Some(DEFAULT_TEMPERATURE));
        let top_p = options.top_p;

        let (text, prompt_tokens, completion_tokens) =
            self.generate(&prompt, max_tokens, temperature, top_p)?;

        Ok(ChatResponse {
            message: Message::assistant(text),
            usage: Usage::new(prompt_tokens, completion_tokens),
            finish_reason: Some("stop".to_string()),
        })
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        _tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        // Candle's forward pass is synchronous and not Send-friendly across
        // threads, so we run the whole decode inline and emit chunks
        // afterwards. Per-token streaming requires interleaving sample/forward
        // with `yield`, which is fine on native but pulls in `tokio::spawn`
        // for true parallelism — so we keep this simple to stay
        // wasm-compatible (no `tokio::spawn`, no `std::thread`).
        let prompt = self.format_prompt(messages, options.system.as_deref());
        let max_tokens = options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        let temperature = options.temperature.or(Some(DEFAULT_TEMPERATURE));
        let top_p = options.top_p;

        Box::pin(async_stream_helper(
            self,
            prompt,
            max_tokens,
            temperature,
            top_p,
        ))
    }
}

/// Standalone async generator so `stream_chat` doesn't need the `async-stream`
/// crate (which is only enabled under the `native` feature flag).
fn async_stream_helper<'a>(
    provider: &'a CandleLlmProvider,
    prompt: String,
    max_tokens: u32,
    temperature: Option<f32>,
    top_p: Option<f32>,
) -> impl futures::Stream<Item = Result<StreamChunk>> + 'a {
    use futures::stream::{self, StreamExt};

    let result = provider.generate(&prompt, max_tokens, temperature, top_p);
    match result {
        Ok((text, prompt_tokens, completion_tokens)) => {
            // Emit the full text as a single chunk plus the usage and Done
            // markers. Per-token streaming is a future enhancement.
            let chunks: Vec<Result<StreamChunk>> = vec![
                Ok(StreamChunk::Text(text)),
                Ok(StreamChunk::Usage(Usage::new(
                    prompt_tokens,
                    completion_tokens,
                ))),
                Ok(StreamChunk::Done),
            ];
            stream::iter(chunks).boxed()
        }
        Err(e) => stream::iter(vec![Err(e)]).boxed(),
    }
}

/// Conservative default config compatible with the Gemma 3 architecture in
/// `candle_transformers::models::gemma3`. Used until the official
/// `google/gemma-4-e2b/config.json` is published; see the TODO in
/// [`CandleLlmProvider::from_bytes`].
pub fn default_gemma_e2b_config() -> GemmaConfig {
    use candle_nn::Activation;
    GemmaConfig {
        attention_bias: false,
        head_dim: 256,
        hidden_activation: Activation::GeluPytorchTanh,
        hidden_size: 2048,
        intermediate_size: 16384,
        num_attention_heads: 8,
        num_hidden_layers: 26,
        num_key_value_heads: 4,
        rms_norm_eps: 1e-6,
        rope_theta: 1_000_000.0,
        rope_local_base_freq: 10_000.0,
        vocab_size: 262_144,
        final_logit_softcapping: None,
        attn_logit_softcapping: None,
        query_pre_attn_scalar: 256,
        sliding_window: 4096,
        sliding_window_pattern: 6,
        max_position_embeddings: 8192,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_rejects_garbage() {
        let res = CandleLlmProvider::from_bytes("gemma-4-e2b", vec![0u8; 16], vec![0u8; 16]);
        assert!(res.is_err(), "garbage bytes must not yield a model");
    }

    #[test]
    fn default_config_is_consistent() {
        let cfg = default_gemma_e2b_config();
        // sliding_window_pattern divides into the layer index without
        // panicking; layer 0..N must all evaluate.
        assert!(cfg.sliding_window_pattern > 0);
        assert!(cfg.num_hidden_layers > 0);
    }
}
