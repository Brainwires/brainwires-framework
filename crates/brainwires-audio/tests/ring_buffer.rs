//! Integration tests for AudioRingBuffer.

use brainwires_audio::{AudioConfig, AudioRingBuffer, SampleFormat};

fn tiny_config() -> AudioConfig {
    // 4 Hz mono I16 = 2 bytes/frame, so 1 second capacity = 8 bytes.
    AudioConfig {
        sample_rate: 4,
        channels: 1,
        sample_format: SampleFormat::I16,
    }
}

// ── Basic operations ─────────────────────────────────────────────────

#[test]
fn new_buffer_is_empty() {
    let buf = AudioRingBuffer::new(AudioConfig::speech(), 1.0);
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    assert!(!buf.is_full());
}

#[test]
fn capacity_matches_duration() {
    // 16kHz mono I16, 1 second => 16000 frames * 2 bytes = 32000 bytes
    let buf = AudioRingBuffer::new(AudioConfig::speech(), 1.0);
    assert_eq!(buf.capacity(), 32_000);
}

#[test]
fn capacity_stereo_f32() {
    // 48kHz stereo F32, 0.5 second => 24000 frames * 8 bytes = 192000 bytes
    let buf = AudioRingBuffer::new(AudioConfig::high_quality(), 0.5);
    assert_eq!(buf.capacity(), 192_000);
}

#[test]
fn push_and_read() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    buf.push(&[10, 20, 30]);
    assert_eq!(buf.len(), 3);
    assert_eq!(buf.read_all(), vec![10, 20, 30]);
}

#[test]
fn push_fills_to_capacity() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    assert_eq!(buf.capacity(), 8);
    buf.push(&[1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(buf.is_full());
    assert_eq!(buf.len(), 8);
    assert_eq!(buf.read_all(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

// ── Wrapping ─────────────────────────────────────────────────────────

#[test]
fn push_overwrites_oldest_when_full() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    buf.push(&[1, 2, 3, 4, 5, 6, 7, 8]); // fill
    buf.push(&[9, 10]); // overwrite first two
    let data = buf.read_all();
    assert_eq!(data, vec![3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn multiple_wraps_preserve_order() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    // Fill, then overwrite twice
    buf.push(&[1, 2, 3, 4, 5, 6, 7, 8]);
    buf.push(&[9, 10, 11, 12, 13, 14, 15, 16]);
    let data = buf.read_all();
    assert_eq!(data, vec![9, 10, 11, 12, 13, 14, 15, 16]);
}

// ── Duration ─────────────────────────────────────────────────────────

#[test]
fn duration_tracks_pushed_data() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    // Push 4 bytes = 2 frames at 4 Hz => 0.5 seconds
    buf.push(&[0, 0, 0, 0]);
    assert!((buf.duration_secs() - 0.5).abs() < 1e-9);
}

#[test]
fn duration_caps_at_capacity() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    // Push 12 bytes but capacity is 8 => duration = 4 frames / 4 Hz = 1.0s
    buf.push(&[0; 12]);
    assert!((buf.duration_secs() - 1.0).abs() < 1e-9);
}

// ── Clear ────────────────────────────────────────────────────────────

#[test]
fn clear_resets_to_empty() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    buf.push(&[1, 2, 3, 4, 5, 6, 7, 8]);
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.duration_secs(), 0.0);
}

#[test]
fn push_after_clear_works() {
    let mut buf = AudioRingBuffer::new(tiny_config(), 1.0);
    buf.push(&[1, 2, 3, 4]);
    buf.clear();
    buf.push(&[10, 20]);
    assert_eq!(buf.len(), 2);
    assert_eq!(buf.read_all(), vec![10, 20]);
}

// ── Config accessor ──────────────────────────────────────────────────

#[test]
fn config_accessor_returns_original() {
    let cfg = AudioConfig::cd_quality();
    let buf = AudioRingBuffer::new(cfg.clone(), 0.1);
    assert_eq!(buf.config().sample_rate, 44_100);
    assert_eq!(buf.config().channels, 2);
}
