//! FFI-safe mirror types for brainwires-hardware.
//!
//! These types are annotated with UniFFI derives so they can cross the Rust ↔ C#
//! (or Kotlin/Swift/Python) boundary. Each has `From` conversions to/from the
//! native brainwires-hardware equivalents.

use brainwires_hardware::{
    AudioBuffer, AudioConfig, AudioDevice, DeviceDirection, OutputFormat, SampleFormat, SttOptions,
    Transcript, TranscriptSegment, TtsOptions, Voice,
};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Audio sample format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiSampleFormat {
    /// 16-bit signed integer.
    I16,
    /// 32-bit floating point.
    F32,
}

impl From<SampleFormat> for FfiSampleFormat {
    fn from(f: SampleFormat) -> Self {
        match f {
            SampleFormat::I16 => Self::I16,
            SampleFormat::F32 => Self::F32,
        }
    }
}

impl From<FfiSampleFormat> for SampleFormat {
    fn from(f: FfiSampleFormat) -> Self {
        match f {
            FfiSampleFormat::I16 => Self::I16,
            FfiSampleFormat::F32 => Self::F32,
        }
    }
}

/// Audio output format for TTS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiOutputFormat {
    /// WAV container.
    Wav,
    /// MP3 compressed.
    Mp3,
    /// Raw PCM bytes.
    Pcm,
    /// Opus compressed.
    Opus,
    /// FLAC lossless.
    Flac,
}

impl From<OutputFormat> for FfiOutputFormat {
    fn from(f: OutputFormat) -> Self {
        match f {
            OutputFormat::Wav => Self::Wav,
            OutputFormat::Mp3 => Self::Mp3,
            OutputFormat::Pcm => Self::Pcm,
            OutputFormat::Opus => Self::Opus,
            OutputFormat::Flac => Self::Flac,
        }
    }
}

impl From<FfiOutputFormat> for OutputFormat {
    fn from(f: FfiOutputFormat) -> Self {
        match f {
            FfiOutputFormat::Wav => Self::Wav,
            FfiOutputFormat::Mp3 => Self::Mp3,
            FfiOutputFormat::Pcm => Self::Pcm,
            FfiOutputFormat::Opus => Self::Opus,
            FfiOutputFormat::Flac => Self::Flac,
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Audio buffer — raw PCM data with metadata (flattened from AudioBuffer + AudioConfig).
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiAudioBuffer {
    /// Raw audio bytes (PCM, little-endian).
    pub data: Vec<u8>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: u16,
    /// Sample format.
    pub sample_format: FfiSampleFormat,
}

impl From<AudioBuffer> for FfiAudioBuffer {
    fn from(b: AudioBuffer) -> Self {
        Self {
            data: b.data,
            sample_rate: b.config.sample_rate,
            channels: b.config.channels,
            sample_format: b.config.sample_format.into(),
        }
    }
}

impl From<FfiAudioBuffer> for AudioBuffer {
    fn from(b: FfiAudioBuffer) -> Self {
        Self {
            data: b.data,
            config: AudioConfig {
                sample_rate: b.sample_rate,
                channels: b.channels,
                sample_format: b.sample_format.into(),
            },
        }
    }
}

/// Voice identifier.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiVoice {
    /// Provider-specific voice ID.
    pub id: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// ISO-639-1 language code.
    pub language: Option<String>,
}

impl From<Voice> for FfiVoice {
    fn from(v: Voice) -> Self {
        Self {
            id: v.id,
            name: v.name,
            language: v.language,
        }
    }
}

impl From<FfiVoice> for Voice {
    fn from(v: FfiVoice) -> Self {
        Self {
            id: v.id,
            name: v.name,
            language: v.language,
        }
    }
}

/// TTS synthesis options.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiTtsOptions {
    /// Voice ID to use.
    pub voice_id: String,
    /// Speech speed (0.25–4.0).
    pub speed: Option<f32>,
    /// Output audio format.
    pub output_format: FfiOutputFormat,
}

impl FfiTtsOptions {
    /// Convert to native TtsOptions.
    pub fn to_native(&self) -> TtsOptions {
        TtsOptions {
            voice: Voice {
                id: self.voice_id.clone(),
                name: None,
                language: None,
            },
            speed: self.speed,
            output_format: self.output_format.into(),
        }
    }
}

/// STT transcription options.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiSttOptions {
    /// Language hint (ISO-639-1).
    pub language: Option<String>,
    /// Whether to include word-level timestamps.
    pub timestamps: bool,
    /// Prompt hint for the model.
    pub prompt: Option<String>,
}

impl From<FfiSttOptions> for SttOptions {
    fn from(o: FfiSttOptions) -> Self {
        Self {
            language: o.language,
            timestamps: o.timestamps,
            prompt: o.prompt,
        }
    }
}

/// Transcription segment with timestamps.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiTranscriptSegment {
    /// Segment text.
    pub text: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

impl From<TranscriptSegment> for FfiTranscriptSegment {
    fn from(s: TranscriptSegment) -> Self {
        Self {
            text: s.text,
            start: s.start,
            end: s.end,
        }
    }
}

/// Transcription result.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiTranscript {
    /// Full transcription text.
    pub text: String,
    /// Detected language.
    pub language: Option<String>,
    /// Audio duration in seconds.
    pub duration_secs: Option<f64>,
    /// Word-level segments (if timestamps requested).
    pub segments: Vec<FfiTranscriptSegment>,
}

impl From<Transcript> for FfiTranscript {
    fn from(t: Transcript) -> Self {
        Self {
            text: t.text,
            language: t.language,
            duration_secs: t.duration_secs,
            segments: t.segments.into_iter().map(Into::into).collect(),
        }
    }
}

/// Audio device descriptor.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiAudioDevice {
    /// Platform-specific device ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether this is the default device.
    pub is_default: bool,
    /// Whether this is an input (capture) device.
    pub is_input: bool,
}

impl From<AudioDevice> for FfiAudioDevice {
    fn from(d: AudioDevice) -> Self {
        Self {
            id: d.id,
            name: d.name,
            is_default: d.is_default,
            is_input: matches!(d.direction, DeviceDirection::Input),
        }
    }
}

/// Provider info returned by `list_providers`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiProviderInfo {
    /// Provider identifier (e.g. "openai", "elevenlabs").
    pub name: String,
    /// Display name.
    pub display_name: String,
    /// Whether this provider supports TTS.
    pub has_tts: bool,
    /// Whether this provider supports STT.
    pub has_stt: bool,
    /// Whether this provider requires a `region` parameter.
    pub requires_region: bool,
}
