use thiserror::Error;

/// Errors that can occur during audio operations.
#[derive(Debug, Error)]
pub enum AudioError {
    /// Audio device not found or unavailable.
    #[error("device error: {0}")]
    Device(String),

    /// Audio capture failed.
    #[error("capture error: {0}")]
    Capture(String),

    /// Audio playback failed.
    #[error("playback error: {0}")]
    Playback(String),

    /// Speech-to-text transcription failed.
    #[error("transcription error: {0}")]
    Transcription(String),

    /// Text-to-speech synthesis failed.
    #[error("synthesis error: {0}")]
    Synthesis(String),

    /// Audio format conversion failed.
    #[error("format error: {0}")]
    Format(String),

    /// API communication error.
    #[error("api error: {0}")]
    Api(String),

    /// Audio stream was interrupted or closed.
    #[error("stream closed: {0}")]
    StreamClosed(String),

    /// Unsupported configuration requested.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// IO error.
    #[error("io error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

/// Result alias for audio operations.
pub type AudioResult<T> = Result<T, AudioError>;
