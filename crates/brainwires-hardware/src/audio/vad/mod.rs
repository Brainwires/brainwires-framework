//! Voice Activity Detection (VAD) for the voice assistant pipeline.
//!
//! Two implementations are provided:
//!
//! - [`EnergyVad`] — pure-Rust RMS energy threshold. Zero extra dependencies.
//!   Always available when the `audio` feature is enabled.
//! - [`WebRtcVad`] — wraps the WebRTC VAD algorithm (three aggressiveness modes).
//!   Enabled by the `vad` feature flag.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use brainwires_hardware::audio::vad::{EnergyVad, VoiceActivityDetector};
//! // ... create an AudioBuffer from mic capture, then:
//! let vad = EnergyVad::default();
//! // if vad.is_speech(&buffer) { /* speech detected */ }
//! ```

/// Energy-based VAD implementation.
pub mod energy;
/// WebRTC-based VAD implementation.
#[cfg(feature = "vad")]
pub mod webrtc;

pub use energy::EnergyVad;
#[cfg(feature = "vad")]
pub use webrtc::{VadMode, WebRtcVad};

use crate::audio::types::{AudioBuffer, SampleFormat};

/// A span within an audio buffer classified as speech or silence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechSegment {
    /// Whether this segment contains speech.
    pub is_speech: bool,
    /// Start sample index within the source buffer.
    pub start_sample: usize,
    /// Exclusive end sample index within the source buffer.
    pub end_sample: usize,
}

impl SpeechSegment {
    /// Number of samples in this segment.
    pub fn len(&self) -> usize {
        self.end_sample.saturating_sub(self.start_sample)
    }

    /// Returns `true` if this segment contains no samples.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A voice activity detector that classifies audio frames as speech or silence.
pub trait VoiceActivityDetector: Send + Sync {
    /// Returns `true` if `audio` contains any speech.
    fn is_speech(&self, audio: &AudioBuffer) -> bool;

    /// Segment `audio` into alternating speech / silence spans.
    ///
    /// `frame_ms` controls the granularity of analysis (10, 20, or 30 ms).
    fn detect_segments(&self, audio: &AudioBuffer, frame_ms: u32) -> Vec<SpeechSegment>;
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Compute the RMS energy of a PCM buffer in decibels (dBFS).
/// Returns `f32::NEG_INFINITY` for a silent buffer.
pub(crate) fn rms_db(audio: &AudioBuffer) -> f32 {
    let samples = pcm_to_f32(audio);
    if samples.is_empty() {
        return f32::NEG_INFINITY;
    }
    let mean_sq = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    if mean_sq == 0.0 {
        return f32::NEG_INFINITY;
    }
    10.0 * mean_sq.log10()
}

/// Convert a raw PCM `AudioBuffer` to `Vec<f32>` normalised to [-1, 1].
pub(crate) fn pcm_to_f32(audio: &AudioBuffer) -> Vec<f32> {
    match audio.config.sample_format {
        SampleFormat::I16 => audio
            .data
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect(),
        SampleFormat::F32 => audio
            .data
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
    }
}

/// Convert a raw PCM `AudioBuffer` to mono `Vec<i16>` (mix down if stereo).
pub(crate) fn pcm_to_i16_mono(audio: &AudioBuffer) -> Vec<i16> {
    let channels = audio.config.channels as usize;
    match audio.config.sample_format {
        SampleFormat::I16 => {
            let raw: Vec<i16> = audio
                .data
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();
            if channels <= 1 {
                raw
            } else {
                raw.chunks(channels)
                    .map(|ch| {
                        let sum: i32 = ch.iter().map(|&s| s as i32).sum();
                        (sum / channels as i32) as i16
                    })
                    .collect()
            }
        }
        SampleFormat::F32 => {
            let raw: Vec<f32> = audio
                .data
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            let mono: Vec<f32> = if channels <= 1 {
                raw
            } else {
                raw.chunks(channels)
                    .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                    .collect()
            };
            mono.iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect()
        }
    }
}
