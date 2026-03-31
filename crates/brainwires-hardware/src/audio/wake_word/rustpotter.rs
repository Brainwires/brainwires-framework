use std::path::Path;
use std::time::Instant;

use rustpotter::{Rustpotter, RustpotterBuilder};
use tracing::debug;

use crate::audio::error::{AudioError, AudioResult};
use crate::audio::wake_word::{WakeWordDetection, WakeWordDetector};

/// Wake word detector backed by [`rustpotter`] — pure Rust, Apache 2.0.
///
/// Requires one or more `.rpw` keyword model files (record samples with
/// `rustpotter-cli` to create them). Supports both DTW and ONNX neural models.
///
/// Feature: `wake-word`
///
/// # Example
/// ```rust,no_run
/// use brainwires_hardware::audio::wake_word::{RustpotterDetector, WakeWordDetector};
/// let mut d = RustpotterDetector::from_model_file("hey_assistant.rpw", 0.5).unwrap();
/// // Feed detector.frame_size() i16 samples per call:
/// // if let Some(det) = d.process_frame(&samples) { println!("Wake word: {}", det.keyword); }
/// ```
pub struct RustpotterDetector {
    inner: Rustpotter,
    frame_size: usize,
    start: Instant,
}

impl RustpotterDetector {
    /// Load a single `.rpw` model file.
    pub fn from_model_file(path: impl AsRef<Path>, threshold: f32) -> AudioResult<Self> {
        Self::from_model_files(&[path.as_ref()], threshold)
    }

    /// Load multiple `.rpw` model files.
    pub fn from_model_files(paths: &[impl AsRef<Path>], threshold: f32) -> AudioResult<Self> {
        let mut inner = RustpotterBuilder::new()
            .set_threshold(threshold)
            .build()
            .map_err(|e| AudioError::Device(format!("rustpotter init failed: {e}")))?;

        for path in paths {
            let p = path.as_ref();
            inner
                .add_wakeword_from_file(p.to_str().unwrap_or_default())
                .map_err(|e| {
                    AudioError::Device(format!(
                        "failed to load wake word model {}: {e}",
                        p.display()
                    ))
                })?;
        }

        let frame_size = inner.get_samples_per_frame();
        debug!("RustpotterDetector ready — frame_size={frame_size}");

        Ok(Self {
            inner,
            frame_size,
            start: Instant::now(),
        })
    }
}

impl WakeWordDetector for RustpotterDetector {
    fn sample_rate(&self) -> u32 {
        16_000
    }

    fn frame_size(&self) -> usize {
        self.frame_size
    }

    fn process_frame(&mut self, samples: &[i16]) -> Option<WakeWordDetection> {
        let result = self.inner.process_pcm_signed(samples)?;
        let timestamp_ms = self.start.elapsed().as_millis() as u64;
        debug!(
            keyword = %result.name,
            score = result.score,
            "Wake word detected"
        );
        Some(WakeWordDetection {
            keyword: result.name,
            score: result.score,
            timestamp_ms,
        })
    }
}
