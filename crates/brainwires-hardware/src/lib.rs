#![warn(missing_docs)]

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
//! | [`camera`] | `camera` | Webcam/camera frame capture (V4L2/AVFoundation/MSMF) |
//! | [`usb`] | `usb` | Raw USB device enumeration and bulk/control/interrupt transfers |
//! | [`homeauto`] | `homeauto` | Home automation: Zigbee (EZSP+ZNP), Z-Wave, Thread (OTBR), Matter |
//!
//! ## Feature flags
//!
//! ```toml
//! [dependencies]
//! brainwires-hardware = { version = "0.10", features = ["audio", "gpio", "bluetooth", "network"] }
//! # or enable everything:
//! brainwires-hardware = { version = "0.10", features = ["full"] }
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
//!
//! ### Camera
//! The `camera` feature enables video frame capture using [`nokhwa`](https://crates.io/crates/nokhwa):
//! V4L2 on Linux, AVFoundation on macOS, Media Foundation on Windows.
//!
//! ### USB
//! The `usb` feature provides raw USB device enumeration and transfers via
//! [`nusb`](https://crates.io/crates/nusb) — a pure-Rust async USB library
//! with no `libusb` system dependency.

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

/// Camera and webcam frame capture.
#[cfg(feature = "camera")]
pub mod camera;

/// Raw USB device access and transfers.
#[cfg(feature = "usb")]
pub mod usb;

/// Home automation protocols: Zigbee (EZSP + ZNP), Z-Wave (Serial API), Thread (OTBR), Matter.
#[cfg(any(
    feature = "homeauto",
    feature = "zigbee",
    feature = "zwave",
    feature = "thread",
    feature = "matter",
    feature = "matter-ble",
))]
pub mod homeauto;

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

// ── Camera re-exports ─────────────────────────────────────────────────────────

#[cfg(feature = "camera")]
pub use camera::{
    CameraCapture, CameraDevice, CameraError, CameraFormat, CameraFrame, FrameRate, NokhwaCapture,
    PixelFormat, Resolution,
};

// ── USB re-exports ────────────────────────────────────────────────────────────

#[cfg(feature = "usb")]
pub use usb::{UsbClass, UsbDevice, UsbError, UsbHandle, UsbSpeed};

// ── Home automation re-exports ────────────────────────────────────────────────

#[cfg(any(
    feature = "homeauto",
    feature = "zigbee",
    feature = "zwave",
    feature = "thread",
    feature = "matter",
    feature = "matter-ble",
))]
pub use homeauto::{HomeAutoError, HomeAutoEvent, HomeAutoResult, HomeDevice, Protocol};

#[cfg(feature = "zigbee")]
pub use homeauto::{EzspCoordinator, ZigbeeAddr, ZigbeeCoordinator, ZigbeeDevice, ZnpCoordinator};

#[cfg(feature = "zwave")]
pub use homeauto::{CommandClass, NodeId, ZWaveController, ZWaveNode, ZWaveSerialController};

#[cfg(feature = "thread")]
pub use homeauto::{ThreadBorderRouter, ThreadNeighbor, ThreadNodeInfo};

#[cfg(feature = "matter")]
pub use homeauto::{MatterController, MatterDevice, MatterDeviceConfig, MatterDeviceServer};

// ── VAD re-exports ────────────────────────────────────────────────────────────

#[cfg(feature = "audio")]
pub use audio::{EnergyVad, SpeechSegment, VoiceActivityDetector};
#[cfg(feature = "vad")]
pub use audio::{VadMode, WebRtcVad};

// ── Wake word re-exports ──────────────────────────────────────────────────────

#[cfg(feature = "wake-word-rustpotter")]
pub use audio::RustpotterDetector;
#[cfg(any(
    feature = "wake-word",
    feature = "wake-word-rustpotter"
))]
pub use audio::{EnergyTriggerDetector, WakeWordDetection, WakeWordDetector};

// ── Voice assistant re-exports ────────────────────────────────────────────────

#[cfg(feature = "voice-assistant")]
pub use audio::{
    AssistantState, VoiceAssistant, VoiceAssistantBuilder, VoiceAssistantConfig,
    VoiceAssistantHandler,
};
