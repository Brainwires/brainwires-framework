use serde::{Deserialize, Serialize};

/// Represents an audio device (input or output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    /// Unique device identifier (platform-specific).
    pub id: String,
    /// Human-readable device name.
    pub name: String,
    /// Whether this is the system default device.
    pub is_default: bool,
    /// Device direction.
    pub direction: DeviceDirection,
}

/// Whether a device is for input (capture) or output (playback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceDirection {
    /// Capture / microphone input.
    Input,
    /// Playback / speaker output.
    Output,
}
