//! Integration tests for AudioBuffer construction, sizing, and duration.

use brainwires_audio::{AudioBuffer, AudioConfig};

// ── Construction ─────────────────────────────────────────────────────

#[test]
fn new_buffer_is_empty() {
    let buf = AudioBuffer::new(AudioConfig::speech());
    assert!(buf.is_empty());
    assert_eq!(buf.num_frames(), 0);
    assert_eq!(buf.duration_secs(), 0.0);
}

#[test]
fn from_pcm_stores_data_and_config() {
    let data = vec![0u8; 128];
    let cfg = AudioConfig::cd_quality();
    let buf = AudioBuffer::from_pcm(data.clone(), cfg);
    assert_eq!(buf.data, data);
    assert_eq!(buf.config.sample_rate, 44_100);
    assert_eq!(buf.config.channels, 2);
    assert!(!buf.is_empty());
}

// ── Frame counting ───────────────────────────────────────────────────

#[test]
fn num_frames_mono_i16() {
    // 1 channel, I16 = 2 bytes/frame => 100 bytes = 50 frames
    let buf = AudioBuffer::from_pcm(vec![0u8; 100], AudioConfig::speech());
    assert_eq!(buf.num_frames(), 50);
}

#[test]
fn num_frames_stereo_i16() {
    // 2 channels, I16 = 4 bytes/frame => 100 bytes = 25 frames
    let buf = AudioBuffer::from_pcm(vec![0u8; 100], AudioConfig::cd_quality());
    assert_eq!(buf.num_frames(), 25);
}

#[test]
fn num_frames_stereo_f32() {
    // 2 channels, F32 = 8 bytes/frame => 80 bytes = 10 frames
    let buf = AudioBuffer::from_pcm(vec![0u8; 80], AudioConfig::high_quality());
    assert_eq!(buf.num_frames(), 10);
}

#[test]
fn num_frames_truncates_partial_frame() {
    // 2 bytes/frame, 5 bytes => 2 frames (truncated)
    let buf = AudioBuffer::from_pcm(vec![0u8; 5], AudioConfig::speech());
    assert_eq!(buf.num_frames(), 2);
}

// ── Duration ─────────────────────────────────────────────────────────

#[test]
fn duration_one_second_speech() {
    // 16kHz mono I16 => 2 bytes/frame => 32000 bytes = 16000 frames = 1.0s
    let buf = AudioBuffer::from_pcm(vec![0u8; 32_000], AudioConfig::speech());
    assert!((buf.duration_secs() - 1.0).abs() < 1e-9);
}

#[test]
fn duration_one_second_cd_quality() {
    // 44100 Hz stereo I16 => 4 bytes/frame => 176400 bytes = 44100 frames = 1.0s
    let buf = AudioBuffer::from_pcm(vec![0u8; 176_400], AudioConfig::cd_quality());
    assert!((buf.duration_secs() - 1.0).abs() < 1e-9);
}

#[test]
fn duration_one_second_high_quality() {
    // 48000 Hz stereo F32 => 8 bytes/frame => 384000 bytes = 48000 frames = 1.0s
    let buf = AudioBuffer::from_pcm(vec![0u8; 384_000], AudioConfig::high_quality());
    assert!((buf.duration_secs() - 1.0).abs() < 1e-9);
}

#[test]
fn duration_half_second() {
    // 16kHz mono I16 => 16000 bytes = 8000 frames = 0.5s
    let buf = AudioBuffer::from_pcm(vec![0u8; 16_000], AudioConfig::speech());
    assert!((buf.duration_secs() - 0.5).abs() < 1e-9);
}

#[test]
fn duration_empty_is_zero() {
    let buf = AudioBuffer::new(AudioConfig::high_quality());
    assert_eq!(buf.duration_secs(), 0.0);
}
