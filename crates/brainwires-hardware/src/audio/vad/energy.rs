use crate::audio::types::{AudioBuffer, SampleFormat};
use crate::audio::vad::{rms_db, SpeechSegment, VoiceActivityDetector};

/// A pure-Rust energy-based Voice Activity Detector.
///
/// Computes the RMS energy of each frame and compares it against a dBFS
/// threshold. Simple and dependency-free, though less accurate than the
/// WebRTC VAD algorithm on noisy signals.
///
/// # Example
/// ```rust,no_run
/// use brainwires_hardware::audio::vad::{EnergyVad, VoiceActivityDetector};
/// let vad = EnergyVad::default();  // -40 dB threshold
/// ```
pub struct EnergyVad {
    /// Energy threshold in dBFS. Frames above this level are classified as
    /// speech. Typical values: -40 dB (quiet room) to -20 dB (noisy).
    pub threshold_db: f32,
}

impl Default for EnergyVad {
    fn default() -> Self {
        Self { threshold_db: -40.0 }
    }
}

impl EnergyVad {
    /// Create a detector with a custom threshold.
    pub fn new(threshold_db: f32) -> Self {
        Self { threshold_db }
    }
}

impl VoiceActivityDetector for EnergyVad {
    fn is_speech(&self, audio: &AudioBuffer) -> bool {
        rms_db(audio) > self.threshold_db
    }

    fn detect_segments(&self, audio: &AudioBuffer, frame_ms: u32) -> Vec<SpeechSegment> {
        let sr = audio.config.sample_rate;
        let channels = audio.config.channels as usize;
        let bytes_per_sample = match audio.config.sample_format {
            SampleFormat::I16 => 2,
            SampleFormat::F32 => 4,
        };
        let frame_samples = (sr * frame_ms / 1000) as usize * channels;
        let frame_bytes = frame_samples * bytes_per_sample;

        let total_frames = audio.data.len() / frame_bytes.max(1);
        let mut segments: Vec<SpeechSegment> = Vec::new();

        for i in 0..total_frames {
            let start = i * frame_bytes;
            let end = (start + frame_bytes).min(audio.data.len());
            let frame_data = audio.data[start..end].to_vec();
            let frame_buf = AudioBuffer {
                data: frame_data,
                config: audio.config.clone(),
            };
            let is_speech = rms_db(&frame_buf) > self.threshold_db;
            let sample_start = i * frame_samples;
            let sample_end = sample_start + frame_samples;

            match segments.last_mut() {
                Some(last) if last.is_speech == is_speech => {
                    last.end_sample = sample_end;
                }
                _ => segments.push(SpeechSegment {
                    is_speech,
                    start_sample: sample_start,
                    end_sample: sample_end,
                }),
            }
        }

        segments
    }
}
