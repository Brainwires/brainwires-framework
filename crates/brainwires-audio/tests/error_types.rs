//! Integration tests for AudioError variants and Display implementations.

use brainwires_audio::AudioError;

#[test]
fn device_error_display() {
    let err = AudioError::Device("no device".into());
    assert_eq!(err.to_string(), "device error: no device");
}

#[test]
fn capture_error_display() {
    let err = AudioError::Capture("overrun".into());
    assert_eq!(err.to_string(), "capture error: overrun");
}

#[test]
fn playback_error_display() {
    let err = AudioError::Playback("underrun".into());
    assert_eq!(err.to_string(), "playback error: underrun");
}

#[test]
fn transcription_error_display() {
    let err = AudioError::Transcription("model failed".into());
    assert_eq!(err.to_string(), "transcription error: model failed");
}

#[test]
fn synthesis_error_display() {
    let err = AudioError::Synthesis("voice not found".into());
    assert_eq!(err.to_string(), "synthesis error: voice not found");
}

#[test]
fn format_error_display() {
    let err = AudioError::Format("bad codec".into());
    assert_eq!(err.to_string(), "format error: bad codec");
}

#[test]
fn api_error_display() {
    let err = AudioError::Api("401 unauthorized".into());
    assert_eq!(err.to_string(), "api error: 401 unauthorized");
}

#[test]
fn stream_closed_error_display() {
    let err = AudioError::StreamClosed("eof".into());
    assert_eq!(err.to_string(), "stream closed: eof");
}

#[test]
fn unsupported_error_display() {
    let err = AudioError::Unsupported("24-bit PCM".into());
    assert_eq!(err.to_string(), "unsupported: 24-bit PCM");
}

#[test]
fn io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let audio_err: AudioError = io_err.into();
    let msg = audio_err.to_string();
    assert!(msg.contains("file missing"));
}

#[test]
fn error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    // AudioError contains std::io::Error which is Send+Sync
    // This ensures our error type composes well in async contexts.
    assert_send_sync::<AudioError>();
}
