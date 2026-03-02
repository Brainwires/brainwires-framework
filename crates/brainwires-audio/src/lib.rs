pub mod error;
pub mod types;
pub mod capture;
pub mod playback;
pub mod stt;
pub mod tts;
pub mod device;
pub mod wav;
pub mod buffer;

#[cfg(feature = "native")]
pub mod hardware;

#[cfg(feature = "native")]
pub mod api;

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
pub use api::{OpenAiStt, OpenAiTts};
#[cfg(feature = "local-stt")]
pub use local::WhisperStt;
