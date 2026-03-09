//! FFI-safe error types.

/// Error type exposed across the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiAudioError {
    /// Provider API error (network, auth, rate limit, etc.).
    #[error("provider error: {message}")]
    Provider { message: String },

    /// Invalid provider handle.
    #[error("invalid handle: {message}")]
    InvalidHandle { message: String },

    /// Unsupported operation for this provider.
    #[error("unsupported: {message}")]
    Unsupported { message: String },

    /// Hardware audio error (device, capture, playback).
    #[error("hardware error: {message}")]
    Hardware { message: String },

    /// Unknown provider name.
    #[error("unknown provider: {message}")]
    UnknownProvider { message: String },
}
