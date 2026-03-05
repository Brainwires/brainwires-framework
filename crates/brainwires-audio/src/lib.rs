#![warn(missing_docs)]
//! # brainwires-audio
//!
//! Audio capture, playback, speech-to-text, and text-to-speech for the
//! Brainwires Agent Framework.

/// Error types for audio operations.
pub mod error;
/// Core audio types, configs, and data structures.
pub mod types;
/// Audio capture trait and implementations.
pub mod capture;
/// Audio playback trait and implementations.
pub mod playback;
/// Speech-to-text trait and implementations.
pub mod stt;
/// Text-to-speech trait and implementations.
pub mod tts;
/// Audio device enumeration and selection.
pub mod device;
/// WAV encoding and decoding utilities.
pub mod wav;
/// Ring buffer for streaming audio data.
pub mod buffer;

/// Native hardware audio backends using cpal.
#[cfg(feature = "native")]
pub mod hardware;

/// Cloud API integrations (OpenAI STT/TTS).
#[cfg(feature = "native")]
pub mod api;

/// FLAC encoding utilities.
#[cfg(feature = "flac")]
pub mod flac;

#[cfg(feature = "local-stt")]
pub mod local;

// Re-exports
pub use error::{AudioError, AudioResult};
pub use types::{
    AudioBuffer, AudioConfig, OutputFormat, SampleFormat, SttOptions, Transcript,
    TranscriptSegment, TtsOptions, Voice,
};
pub use capture::AudioCapture;
pub use playback::AudioPlayback;
pub use stt::SpeechToText;
pub use tts::TextToSpeech;
pub use device::{AudioDevice, DeviceDirection};
pub use wav::{decode_wav, encode_wav};
pub use buffer::AudioRingBuffer;

#[cfg(feature = "native")]
pub use hardware::{CpalCapture, CpalPlayback};
#[cfg(feature = "native")]
pub use api::{
    OpenAiStt, OpenAiTts,
    ElevenLabsTts, ElevenLabsStt,
    DeepgramTts, DeepgramStt,
    GoogleTts,
    AzureTts, AzureStt,
    FishTts, FishStt,
    CartesiaTts,
    MurfTts,
};
#[cfg(feature = "flac")]
pub use flac::{decode_flac, encode_flac};
#[cfg(feature = "local-stt")]
pub use local::WhisperStt;
