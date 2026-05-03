//! Native Gemma 4 forward-pass diagnostic rig.
//!
//! Reproduces the chat-pwa's wasm32+WebGPU step-0 forward pass against a
//! native candle WGPU device (or CPU). Used to verify candle-fork bug
//! fixes without spinning up the chat-pwa Docker stack — every iteration
//! of the bug-fix loop becomes `cargo run` instead of `./start.sh dev`
//! plus a browser test.
//!
//! Usage:
//!
//! ```text
//! cargo run --release --example gemma4_diag \
//!     --features native,local-llm-vision,candle-wgpu -- \
//!     --device wgpu --prompt "hello"
//! ```
//!
//! Output: `[gemma4/diag]` lines on stderr (same format the chat-pwa
//! emits to the worker DevTools console), and one `RESULT: PASS|FAIL`
//! line on stdout. Exit code mirrors the result (0 / 1).
//!
//! First run downloads ~10 GB of Gemma 4 E2B weights to
//! `~/.cache/huggingface/hub/`. Subsequent runs reuse the cache.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::var_builder::{SimpleBackend, VarBuilderArgs};
use candle_nn::VarBuilder;
use candle_transformers::models::gemma4::{Model as Gemma4Model, config::Gemma4Config};
use clap::{Parser, ValueEnum};
use hf_hub::api::tokio::Api;
use tokenizers::Tokenizer;

use brainwires_providers::local_llm::vision::{
    Gemma4MultiModal, gemma4_mm::nan_scan_count, gemma4_mm::nan_scan_first_label,
    gemma4_mm::nan_scan_reset,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DeviceArg {
    /// candle WGPU backend — exercises the same kernels the chat-pwa
    /// runs in the browser. Default.
    Wgpu,
    /// candle CPU backend — useful as a reference to confirm a model-math
    /// path is correct independent of WGSL kernels.
    Cpu,
}

#[derive(Parser, Debug)]
#[command(
    about = "Native Gemma 4 forward-pass diagnostic rig",
    long_about = "Reproduces the chat-pwa's step-0 forward pass natively. \
                  Reads model.safetensors + tokenizer.json + config.json from \
                  the HF Hub cache, builds the model on a candle device, runs \
                  one forward step, and exits 0/1 based on whether the diag \
                  scaffold observed any NaN/Inf cells."
)]
struct Args {
    /// Backend device for the forward pass.
    #[arg(long, value_enum, default_value_t = DeviceArg::Wgpu)]
    device: DeviceArg,

    /// HuggingFace model id.
    #[arg(long, default_value = "google/gemma-4-e2b")]
    model_id: String,

    /// HF revision (branch / tag / commit).
    #[arg(long, default_value = "main")]
    revision: String,

    /// Prompt text. Tokenized; one forward step is run per
    /// `--max-new-tokens`.
    #[arg(long, default_value = "hello")]
    prompt: String,

    /// Number of new tokens to generate. The diag scaffold only fires
    /// at step 0, so 1 is usually sufficient.
    #[arg(long, default_value_t = 1)]
    max_new_tokens: usize,

    /// Layer index to capture intra-layer checkpoints for. Set to "none"
    /// to disable. Read by `gemma4_mm` via the `BW_GEMMA4_DIAG_LAYER`
    /// env var; this flag plumbs to the env var.
    #[arg(long, default_value = "8")]
    target_layer: String,

    /// Override the local weights file path (skip HF Hub fetch).
    #[arg(long)]
    weights_file: Option<PathBuf>,

    /// Override the tokenizer file path.
    #[arg(long)]
    tokenizer_file: Option<PathBuf>,

    /// Override the config.json path.
    #[arg(long)]
    config_file: Option<PathBuf>,
}

/// Tensors the chat-pwa filters out of the safetensors load. Replicated
/// here so the native rig sees the same model as the chat-pwa.
///
/// Source: `extras/brainwires-chat-pwa/wasm/src/vision.rs::gemma4_skip_reason`.
fn gemma4_skip_reason(name: &str) -> Option<&'static str> {
    if name.contains(".audio_tower.") || name.contains(".embed_audio.") {
        return Some("audio");
    }
    // `embed_tokens_per_layer.weight` is ~4.7 GB BF16 on E2B and is
    // optional via candle-fork's projection-only PLE fallback. Skipping
    // it avoids GPU memory pressure on the rig (matches chat-pwa's
    // `ple-table-oversize` filter).
    if name.ends_with(".embed_tokens_per_layer.weight") {
        return Some("ple-table-oversize");
    }
    if name.ends_with(".input_min")
        || name.ends_with(".input_max")
        || name.ends_with(".output_min")
        || name.ends_with(".output_max")
    {
        return Some("qat-stat");
    }
    None
}

/// Wraps an inner `SimpleBackend` and reports any tensor matching
/// `gemma4_skip_reason` as absent. Forces candle's PLE construction to
/// fall back to the projection-only path and skips audio/QAT-stat
/// tensors so the model builds without those weights.
struct FilteredBackend {
    inner: Box<dyn SimpleBackend + 'static>,
}

impl SimpleBackend for FilteredBackend {
    fn get(
        &self,
        shape: candle_core::Shape,
        name: &str,
        h: candle_nn::Init,
        dtype: DType,
        dev: &Device,
    ) -> candle_core::Result<Tensor> {
        if let Some(reason) = gemma4_skip_reason(name) {
            return Err(candle_core::Error::Msg(format!(
                "tensor `{name}` filtered ({reason}) — caller should check \
                 contains_tensor first"
            )));
        }
        self.inner.get(shape, name, h, dtype, dev)
    }

    fn get_unchecked(
        &self,
        name: &str,
        dtype: DType,
        dev: &Device,
    ) -> candle_core::Result<Tensor> {
        if let Some(reason) = gemma4_skip_reason(name) {
            return Err(candle_core::Error::Msg(format!(
                "tensor `{name}` filtered ({reason})"
            )));
        }
        self.inner.get_unchecked(name, dtype, dev)
    }

    fn contains_tensor(&self, name: &str) -> bool {
        if gemma4_skip_reason(name).is_some() {
            return false;
        }
        self.inner.contains_tensor(name)
    }
}

fn build_device(arg: DeviceArg) -> Result<Device> {
    match arg {
        DeviceArg::Cpu => Ok(Device::Cpu),
        DeviceArg::Wgpu => Device::new_wgpu(0)
            .context("failed to construct candle WGPU device — ensure the candle-wgpu feature is enabled and a Vulkan/Metal/DX12 adapter is available"),
    }
}

async fn fetch_files(args: &Args) -> Result<(PathBuf, PathBuf, PathBuf)> {
    if let (Some(w), Some(t), Some(c)) = (
        args.weights_file.as_ref(),
        args.tokenizer_file.as_ref(),
        args.config_file.as_ref(),
    ) {
        return Ok((w.clone(), t.clone(), c.clone()));
    }

    let api = Api::new().context("failed to construct hf-hub Api")?;
    let repo = api.repo(hf_hub::Repo::with_revision(
        args.model_id.clone(),
        hf_hub::RepoType::Model,
        args.revision.clone(),
    ));

    let weights = match args.weights_file.clone() {
        Some(p) => p,
        None => repo
            .get("model.safetensors")
            .await
            .context("failed to fetch model.safetensors from HF Hub")?,
    };
    let tokenizer = match args.tokenizer_file.clone() {
        Some(p) => p,
        None => repo
            .get("tokenizer.json")
            .await
            .context("failed to fetch tokenizer.json from HF Hub")?,
    };
    let config = match args.config_file.clone() {
        Some(p) => p,
        None => repo
            .get("config.json")
            .await
            .context("failed to fetch config.json from HF Hub")?,
    };

    Ok((weights, tokenizer, config))
}

fn load_config(path: &std::path::Path) -> Result<Gemma4Config> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let cfg: Gemma4Config = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {} as Gemma4Config", path.display()))?;
    Ok(cfg)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();

    // Plumb the target layer to gemma4_mm.rs via env var. "none" disables
    // intra-capture; numeric strings select a layer; "default" / unset
    // falls back to whatever gemma4_mm decides (currently 8).
    if !args.target_layer.eq_ignore_ascii_case("default") {
        // SAFETY: single-threaded current-thread runtime, set before
        // spawning any async work, so no other thread can be reading
        // env at the same time.
        unsafe {
            std::env::set_var("BW_GEMMA4_DIAG_LAYER", &args.target_layer);
        }
    }

    eprintln!(
        "[gemma4_diag] device={:?} model={} revision={} prompt={:?} target_layer={}",
        args.device, args.model_id, args.revision, args.prompt, args.target_layer
    );

    let result = run(args).await;
    match result {
        Ok(()) => {
            let nans = nan_scan_count();
            if nans > 0 {
                let label = nan_scan_first_label().unwrap_or_else(|| "<unknown>".to_string());
                println!(
                    "RESULT: FAIL  nan_scans={nans}  first_nan_at={label}"
                );
                ExitCode::from(1)
            } else {
                println!("RESULT: PASS  forward pass produced finite logits");
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("[gemma4_diag] error: {e:#}");
            println!("RESULT: ERROR  {e}");
            ExitCode::from(2)
        }
    }
}

async fn run(args: Args) -> Result<()> {
    let device = build_device(args.device)?;
    eprintln!("[gemma4_diag] device built: {:?}", device);

    let (weights, tokenizer_path, config_path) = fetch_files(&args).await?;
    eprintln!("[gemma4_diag] weights={}", weights.display());
    eprintln!("[gemma4_diag] tokenizer={}", tokenizer_path.display());
    eprintln!("[gemma4_diag] config={}", config_path.display());

    let cfg = load_config(&config_path)?;
    eprintln!(
        "[gemma4_diag] config: hidden={} layers={} heads={} head_dim={} global_head_dim={}",
        cfg.text_config.hidden_size,
        cfg.text_config.num_hidden_layers,
        cfg.text_config.num_attention_heads,
        cfg.text_config.head_dim,
        cfg.text_config.global_head_dim,
    );

    let dtype = DType::BF16;
    // SAFETY: from_mmaped_safetensors is unsafe because the underlying
    // mmap can be invalidated by external file writes. We hold the file
    // for the duration of the program; nothing modifies it.
    let inner = unsafe {
        candle_core::safetensors::MmapedSafetensors::multi(&[&weights])
            .context("mmap safetensors")?
    };
    let backend: Box<dyn SimpleBackend + 'static> = Box::new(FilteredBackend {
        inner: Box::new(inner),
    });
    let vb: VarBuilder = VarBuilderArgs::from_backend(backend, dtype, device.clone());

    eprintln!("[gemma4_diag] building Gemma 4 model (text-only, no vision/audio)...");
    let t0 = std::time::Instant::now();
    let model = Gemma4Model::new_partial(&cfg, vb, false, false)
        .context("Gemma4Model::new_partial")?;
    eprintln!("[gemma4_diag] model built in {:?}", t0.elapsed());

    let tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("tokenizer: {e}"))?;

    let pipeline = Gemma4MultiModal::from_components(
        model,
        tokenizer,
        device.clone(),
        cfg.clone(),
    );

    // Gemma 4 EOS = token id 1 (`<eos>` in the tokenizer). Hardcoded
    // because `Gemma4TextConfig` doesn't expose token-id fields. With
    // `max_new_tokens=1` the early-exit doesn't fire anyway, but pass
    // the right value for longer runs.
    let eos: Option<u32> = Some(1);

    nan_scan_reset();
    eprintln!("[gemma4_diag] generating {} token(s)...", args.max_new_tokens);
    let t0 = std::time::Instant::now();
    let output = pipeline
        .generate_greedy(&args.prompt, &[], args.max_new_tokens, eos)
        .await
        .map_err(|e| anyhow::anyhow!("generate_greedy: {e}"))?;
    eprintln!(
        "[gemma4_diag] generate_greedy returned in {:?}: {output:?}",
        t0.elapsed()
    );
    Ok(())
}
