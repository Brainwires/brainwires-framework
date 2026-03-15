//! Integration tests for FLAC encoding and decoding.
//!
//! Only compiled when the `flac` feature is enabled (on by default via `native`).

#![cfg(feature = "flac")]

use brainwires_audio::{AudioBuffer, AudioConfig, SampleFormat, decode_flac, encode_flac};

// ── I16 encoding ─────────────────────────────────────────────────────

#[test]
fn flac_encode_i16_has_magic_header() {
    let samples: Vec<i16> = (0..1600).map(|i| ((i % 256) as i16) * 100).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data, AudioConfig::speech());

    let flac = encode_flac(&buffer).unwrap();
    assert_eq!(&flac[..4], b"fLaC");
}

#[test]
fn flac_encode_i16_compresses() {
    let samples: Vec<i16> = (0..1600).map(|i| ((i % 256) as i16) * 100).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let raw_size = data.len();
    let buffer = AudioBuffer::from_pcm(data, AudioConfig::speech());

    let flac = encode_flac(&buffer).unwrap();
    assert!(
        flac.len() < raw_size,
        "FLAC should compress repetitive data"
    );
}

// ── F32 encoding ─────────────────────────────────────────────────────

#[test]
fn flac_encode_f32_has_magic_header() {
    let samples: Vec<f32> = (0..960).map(|i| (i as f32) / 960.0 * 2.0 - 1.0).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data, AudioConfig::high_quality());

    let flac = encode_flac(&buffer).unwrap();
    assert_eq!(&flac[..4], b"fLaC");
}

// ── Empty buffer ─────────────────────────────────────────────────────

#[test]
fn flac_encode_empty_buffer() {
    let buffer = AudioBuffer::new(AudioConfig::speech());
    let flac = encode_flac(&buffer).unwrap();
    assert_eq!(&flac[..4], b"fLaC");
}

// ── I16 round-trip ───────────────────────────────────────────────────

#[test]
fn flac_roundtrip_i16_lossless() {
    let samples: Vec<i16> = (0..1600).map(|i| ((i % 256) as i16) * 100).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), AudioConfig::speech());

    let flac = encode_flac(&buffer).unwrap();
    let decoded = decode_flac(&flac).unwrap();

    assert_eq!(decoded.config.sample_rate, 16_000);
    assert_eq!(decoded.config.channels, 1);
    assert_eq!(decoded.config.sample_format, SampleFormat::I16);
    assert_eq!(decoded.data, data, "I16 FLAC round-trip should be lossless");
}

#[test]
fn flac_roundtrip_stereo_i16() {
    let cfg = AudioConfig::cd_quality();
    let samples: Vec<i16> = (0..4410).map(|i| (i as i16).wrapping_mul(11)).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let flac = encode_flac(&buffer).unwrap();
    let decoded = decode_flac(&flac).unwrap();

    assert_eq!(decoded.config.sample_rate, 44_100);
    assert_eq!(decoded.config.channels, 2);
    assert_eq!(decoded.data, data);
}

// ── F32 round-trip (lossy due to 24-bit quantization) ────────────────

#[test]
fn flac_roundtrip_f32_preserves_approximately() {
    let cfg = AudioConfig::high_quality();
    let samples: Vec<f32> = (0..960).map(|i| (i as f32) / 960.0 * 2.0 - 1.0).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data, cfg);

    let flac = encode_flac(&buffer).unwrap();
    let decoded = decode_flac(&flac).unwrap();

    // F32 -> 24-bit -> F32 is lossy, but should preserve sample rate/channels.
    assert_eq!(decoded.config.sample_rate, 48_000);
    assert_eq!(decoded.config.channels, 2);
    // Decoded will be F32 (24-bit normalised back to float).
    assert_eq!(decoded.config.sample_format, SampleFormat::F32);
    assert!(!decoded.is_empty());
}

// ── Decode garbage ───────────────────────────────────────────────────

#[test]
fn flac_decode_rejects_garbage() {
    let result = decode_flac(b"this is not FLAC data");
    assert!(result.is_err());
}
