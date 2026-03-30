use std::path::Path;
use std::time::Instant;

use pv_porcupine::{BuiltinKeywords, Porcupine, PorcupineBuilder};
use tracing::debug;

use crate::audio::error::{AudioError, AudioResult};
use crate::audio::wake_word::{WakeWordDetection, WakeWordDetector};

pub use pv_porcupine::BuiltinKeywords;

/// Wake word detector backed by [Picovoice Porcupine].
///
/// Provides excellent accuracy across challenging acoustic environments.
/// Requires:
/// 1. A free Picovoice AccessKey from <https://console.picovoice.ai/>
/// 2. The Porcupine native shared library (`.so` / `.dylib` / `.dll`)
///    must be on the library path or vendored.
///
/// Feature: `wake-word-porcupine`
///
/// **Note:** The Rust Porcupine SDK is maintained by Picovoice until July 2025.
/// For long-term deployments prefer [`RustpotterDetector`].
///
/// [Picovoice Porcupine]: https://picovoice.ai/platform/porcupine/
pub struct PorcupineDetector {
    inner: Porcupine,
    keywords: Vec<String>,
    frame_size: usize,
    start: Instant,
}

impl PorcupineDetector {
    /// Create a detector using one of Porcupine's built-in keywords.
    pub fn from_builtin(
        access_key: &str,
        keyword: BuiltinKeywords,
    ) -> AudioResult<Self> {
        let inner = PorcupineBuilder::new_with_keyword_paths(access_key, &[])
            .keywords(&[keyword])
            .init()
            .map_err(|e| AudioError::Device(format!("Porcupine init failed: {e}")))?;

        let frame_size = inner.frame_length() as usize;
        let keywords = vec![format!("{keyword:?}").to_lowercase()];

        Ok(Self { inner, keywords, frame_size, start: Instant::now() })
    }

    /// Create a detector from one or more custom `.ppn` keyword files.
    pub fn from_keyword_files(
        access_key: &str,
        keyword_paths: &[impl AsRef<Path>],
        sensitivities: &[f32],
    ) -> AudioResult<Self> {
        let paths: Vec<&str> = keyword_paths
            .iter()
            .map(|p| p.as_ref().to_str().unwrap_or_default())
            .collect();

        let mut builder = PorcupineBuilder::new_with_keyword_paths(access_key, &paths);
        if !sensitivities.is_empty() {
            builder = builder.sensitivities(sensitivities);
        }
        let inner = builder
            .init()
            .map_err(|e| AudioError::Device(format!("Porcupine init failed: {e}")))?;

        let frame_size = inner.frame_length() as usize;
        let keywords: Vec<String> = keyword_paths
            .iter()
            .map(|p| {
                p.as_ref()
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "keyword".to_string())
            })
            .collect();

        Ok(Self { inner, keywords, frame_size, start: Instant::now() })
    }
}

impl WakeWordDetector for PorcupineDetector {
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn frame_size(&self) -> usize {
        self.frame_size
    }

    fn process_frame(&mut self, samples: &[i16]) -> Option<WakeWordDetection> {
        let idx = self.inner.process(samples).ok()?;
        if idx < 0 {
            return None;
        }
        let keyword = self
            .keywords
            .get(idx as usize)
            .cloned()
            .unwrap_or_else(|| format!("keyword_{idx}"));

        let timestamp_ms = self.start.elapsed().as_millis() as u64;
        debug!(keyword = %keyword, index = idx, "Wake word detected (Porcupine)");

        Some(WakeWordDetection {
            keyword,
            score: 1.0, // Porcupine doesn't expose a float score
            timestamp_ms,
        })
    }
}
