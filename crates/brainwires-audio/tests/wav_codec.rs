//! Integration tests for WAV encoding and decoding.

use brainwires_audio::{decode_wav, encode_wav, AudioBuffer, AudioConfig, SampleFormat};

// ── I16 round-trips ──────────────────────────────────────────────────

#[test]
fn wav_roundtrip_mono_i16() {
    let cfg = AudioConfig::speech();
    let samples: Vec<i16> = (0..1600).map(|i| (i as i16).wrapping_mul(13)).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.config.sample_rate, 16_000);
    assert_eq!(decoded.config.channels, 1);
    assert_eq!(decoded.config.sample_format, SampleFormat::I16);
    assert_eq!(decoded.data, data);
}

#[test]
fn wav_roundtrip_stereo_i16() {
    let cfg = AudioConfig::cd_quality();
    let samples: Vec<i16> = (0..4410).map(|i| (i as i16).wrapping_mul(7)).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.config.sample_rate, 44_100);
    assert_eq!(decoded.config.channels, 2);
    assert_eq!(decoded.data, data);
}

// ── F32 round-trips ──────────────────────────────────────────────────

#[test]
fn wav_roundtrip_stereo_f32() {
    let cfg = AudioConfig::high_quality();
    let samples: Vec<f32> = (0..960).map(|i| (i as f32) / 960.0).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.config.sample_rate, 48_000);
    assert_eq!(decoded.config.channels, 2);
    assert_eq!(decoded.config.sample_format, SampleFormat::F32);
    assert_eq!(decoded.data, data);
}

#[test]
fn wav_roundtrip_f32_negative_values() {
    let cfg = AudioConfig {
        sample_rate: 22_050,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    let samples: Vec<f32> = (0..500).map(|i| (i as f32) / 250.0 - 1.0).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.config.sample_rate, 22_050);
    assert_eq!(decoded.data, data);
}

// ── Edge cases ───────────────────────────────────────────────────────

#[test]
fn wav_roundtrip_empty_buffer() {
    let buffer = AudioBuffer::new(AudioConfig::speech());
    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();
    assert!(decoded.is_empty());
    assert_eq!(decoded.num_frames(), 0);
}

#[test]
fn wav_roundtrip_single_sample() {
    let data: Vec<u8> = 42i16.to_le_bytes().to_vec();
    let buffer = AudioBuffer::from_pcm(data.clone(), AudioConfig::speech());

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.num_frames(), 1);
    assert_eq!(decoded.data, data);
}

#[test]
fn wav_roundtrip_preserves_duration() {
    // 1 second of audio
    let cfg = AudioConfig::speech();
    let data = vec![0u8; 32_000]; // 16000 frames * 2 bytes
    let buffer = AudioBuffer::from_pcm(data, cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert!((decoded.duration_secs() - 1.0).abs() < 1e-9);
}

#[test]
fn wav_encoded_starts_with_riff_header() {
    let buffer = AudioBuffer::from_pcm(vec![0u8; 100], AudioConfig::speech());
    let wav = encode_wav(&buffer).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
}

#[test]
fn decode_wav_rejects_garbage() {
    let result = decode_wav(b"not a wav file at all");
    assert!(result.is_err());
}

// ── Custom sample rates ──────────────────────────────────────────────

#[test]
fn wav_roundtrip_8khz_mono() {
    let cfg = AudioConfig {
        sample_rate: 8_000,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
    let samples: Vec<i16> = (0..800).map(|i| (i * 3) as i16).collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let buffer = AudioBuffer::from_pcm(data.clone(), cfg);

    let wav = encode_wav(&buffer).unwrap();
    let decoded = decode_wav(&wav).unwrap();

    assert_eq!(decoded.config.sample_rate, 8_000);
    assert_eq!(decoded.data, data);
}
