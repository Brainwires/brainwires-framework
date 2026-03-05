use flacenc::component::BitRepr;
use flacenc::error::Verify;

use crate::error::{AudioError, AudioResult};
use crate::types::{AudioBuffer, SampleFormat};

/// Encode an [`AudioBuffer`] to FLAC format bytes.
///
/// The input buffer must use `I16` or `F32` sample format. F32 samples are
/// quantised to 24-bit integers before encoding (FLAC is integer-only).
pub fn encode_flac(buffer: &AudioBuffer) -> AudioResult<Vec<u8>> {
    let bits_per_sample = match buffer.config.sample_format {
        SampleFormat::I16 => 16,
        SampleFormat::F32 => 24,
    };

    // Convert PCM bytes → interleaved i32 samples (FLAC native format).
    let samples: Vec<i32> = match buffer.config.sample_format {
        SampleFormat::I16 => buffer
            .data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as i32)
            .collect(),
        SampleFormat::F32 => buffer
            .data
            .chunks_exact(4)
            .map(|c| {
                let f = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                // Clamp to [-1, 1] then scale to 24-bit range.
                let clamped = f.clamp(-1.0, 1.0);
                (clamped * 8_388_607.0) as i32
            })
            .collect(),
    };

    let source = flacenc::source::MemSource::from_samples(
        &samples,
        buffer.config.channels as usize,
        bits_per_sample,
        buffer.config.sample_rate as usize,
    );

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| AudioError::Format(format!("FLAC config error: {e:?}")))?;

    let stream =
        flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|e| AudioError::Format(format!("FLAC encode error: {e}")))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| AudioError::Format(format!("FLAC write error: {e}")))?;

    Ok(sink.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioBuffer, AudioConfig};

    #[test]
    fn test_encode_flac_i16() {
        let config = AudioConfig::speech();
        // 0.1s of 16 kHz mono = 1600 samples
        let samples: Vec<i16> = (0..1600).map(|i| ((i % 256) as i16) * 100).collect();
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let buffer = AudioBuffer::from_pcm(data, config);

        let flac_bytes = encode_flac(&buffer).unwrap();

        // FLAC magic: "fLaC"
        assert_eq!(&flac_bytes[..4], b"fLaC");
        // Compressed should be smaller than raw PCM (3200 bytes).
        assert!(flac_bytes.len() < 3200);
    }

    #[test]
    fn test_encode_flac_f32() {
        let config = AudioConfig::high_quality(); // 48 kHz stereo f32
        // 0.01s of 48 kHz stereo = 960 samples
        let samples: Vec<f32> = (0..960).map(|i| (i as f32) / 960.0 * 2.0 - 1.0).collect();
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let buffer = AudioBuffer::from_pcm(data, config);

        let flac_bytes = encode_flac(&buffer).unwrap();
        assert_eq!(&flac_bytes[..4], b"fLaC");
    }

    #[test]
    fn test_encode_flac_empty() {
        let config = AudioConfig::speech();
        let buffer = AudioBuffer::new(config);

        let flac_bytes = encode_flac(&buffer).unwrap();
        assert_eq!(&flac_bytes[..4], b"fLaC");
    }
}
