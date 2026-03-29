//! # brainwires-hardware
//!
//! Hardware I/O for the Brainwires Agent Framework.
//!
//! Provides a unified hardware abstraction layer covering:
//!
//! | Module | Feature flag | Description |
//! |--------|-------------|-------------|
//! | [`audio`] | `audio` | Audio capture/playback, STT, TTS (16 cloud providers + local Whisper) |
//! | [`gpio`] | `gpio` | GPIO pin management with safety allow-lists and PWM (Linux) |
//! | [`bluetooth`] | `bluetooth` | BLE advertisement scanning and adapter enumeration |
//! | [`network`] | `network` | NIC enumeration, IP config, ARP discovery, port scanning |
//!
//! ## Feature flags
//!
//! ```toml
//! [dependencies]
//! brainwires-hardware = { version = "0.6", features = ["audio", "gpio", "bluetooth", "network"] }
//! # or enable everything:
//! brainwires-hardware = { version = "0.6", features = ["full"] }
//! ```
//!
//! ### Audio
//! The `audio` feature enables hardware audio capture/playback via CPAL and
//! 16 cloud STT/TTS provider integrations. Add `local-stt` for offline Whisper
//! inference and `flac` for FLAC codec support.
//!
//! ### GPIO (Linux)
//! The `gpio` feature exposes safe GPIO pin access using the Linux character
//! device API (`gpio-cdev`) with an explicit allow-list safety policy.
//!
//! ### Bluetooth
//! The `bluetooth` feature uses [`btleplug`](https://crates.io/crates/btleplug)
//! for cross-platform BLE scanning (Linux/BlueZ, macOS CoreBluetooth, Windows WinRT).
//!
//! ### Network
//! The `network` feature provides interface enumeration, IP configuration
//! parsing, ARP-based host discovery, and async TCP port scanning.

/// Audio capture, playback, STT, and TTS.
#[cfg(feature = "audio")]
pub mod audio;

/// GPIO hardware access (Linux).
#[cfg(feature = "gpio")]
pub mod gpio;

/// Bluetooth discovery and scanning.
#[cfg(feature = "bluetooth")]
pub mod bluetooth;

/// Network interface enumeration, discovery, and port scanning.
#[cfg(feature = "network")]
pub mod network;

// ── Convenience re-exports: mirrors the old brainwires-audio public API ──────

#[cfg(feature = "audio")]
pub use audio::{
    AudioBuffer, AudioCapture, AudioConfig, AudioDevice, AudioError, AudioPlayback, AudioResult,
    AudioRingBuffer, DeviceDirection, OutputFormat, SampleFormat, SpeechToText, SttOptions,
    TextToSpeech, Transcript, TranscriptSegment, TtsOptions, Voice,
};

#[cfg(feature = "audio")]
pub use audio::{decode_wav, encode_wav};

#[cfg(feature = "audio")]
pub use audio::{
    AzureStt, AzureTts, CartesiaTts, DeepgramStt, DeepgramTts, ElevenLabsStt, ElevenLabsTts,
    FishStt, FishTts, GoogleTts, MurfTts, OpenAiResponsesStt, OpenAiResponsesTts, OpenAiStt,
    OpenAiTts,
};

#[cfg(feature = "audio")]
pub use audio::{CpalCapture, CpalPlayback};

#[cfg(all(feature = "audio", feature = "flac"))]
pub use audio::{decode_flac, encode_flac};

#[cfg(all(feature = "audio", feature = "local-stt"))]
pub use audio::WhisperStt;

// ── GPIO re-exports ───────────────────────────────────────────────────────────

#[cfg(feature = "gpio")]
pub use gpio::{GpioChipInfo, GpioLineInfo, GpioPin, GpioPinManager, GpioSafetyPolicy};
