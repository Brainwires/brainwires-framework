//! Image preprocessing for the Gemma vision tower.
//!
//! Steps (single-crop MVP — Pan-and-Scan deferred):
//!   1. Decode the input bytes (image crate handles JPEG/PNG/WebP).
//!   2. Convert to RGB8.
//!   3. Resize to 896×896 (Lanczos3 by default).
//!   4. Normalize to f32 with mean=[0.5,0.5,0.5], std=[0.5,0.5,0.5]
//!      (SigLIP convention — NOT ImageNet's).
//!   5. Build a `[1, 3, 896, 896]` Tensor on the requested device.

use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;
use image::{DynamicImage, ImageError, imageops::FilterType};

/// 896×896 — the SigLIP-So400m / paligemma_3b_896 input resolution.
pub const GEMMA_VISION_INPUT_SIZE: u32 = 896;

/// Per-channel mean used by SigLIP normalization (NOT ImageNet's).
pub const GEMMA_VISION_NORM_MEAN: [f32; 3] = [0.5, 0.5, 0.5];

/// Per-channel standard deviation used by SigLIP normalization.
pub const GEMMA_VISION_NORM_STD: [f32; 3] = [0.5, 0.5, 0.5];

/// Errors emitted while turning raw image bytes into a Gemma vision-tower tensor.
#[derive(Debug, thiserror::Error)]
pub enum PreprocessError {
    /// Underlying image decode failed (unsupported format, truncated, etc.).
    #[error("image decode failed: {0}")]
    Decode(#[from] ImageError),
    /// Building the candle tensor from the normalized buffer failed.
    #[error("tensor build failed: {0}")]
    Tensor(String),
}

/// Decode + preprocess raw image bytes (JPEG/PNG/WebP/etc.).
/// Returns a `[1, 3, 896, 896]` f32 tensor, normalized.
pub fn preprocess_image_bytes(bytes: &[u8], device: &Device) -> Result<Tensor, PreprocessError> {
    let img = image::load_from_memory(bytes)?;
    preprocess_image_dynamic(img, device)
}

/// Same as `preprocess_image_bytes` but accepts an already-decoded image.
/// Useful when the caller already has a `DynamicImage` in hand.
pub fn preprocess_image_dynamic(
    img: DynamicImage,
    device: &Device,
) -> Result<Tensor, PreprocessError> {
    let rgb = img.to_rgb8();
    let resized = image::imageops::resize(
        &rgb,
        GEMMA_VISION_INPUT_SIZE,
        GEMMA_VISION_INPUT_SIZE,
        FilterType::Lanczos3,
    );

    let size = GEMMA_VISION_INPUT_SIZE as usize;
    let pixels = size * size;
    // CHW order: 3 channels × H × W.
    let mut buf = vec![0f32; 3 * pixels];

    // resized.as_raw() is HWC interleaved (R,G,B,R,G,B,...).
    let raw = resized.as_raw();
    debug_assert_eq!(raw.len(), 3 * pixels);

    for i in 0..pixels {
        let r = raw[3 * i] as f32 / 255.0;
        let g = raw[3 * i + 1] as f32 / 255.0;
        let b = raw[3 * i + 2] as f32 / 255.0;
        buf[i] = (r - GEMMA_VISION_NORM_MEAN[0]) / GEMMA_VISION_NORM_STD[0];
        buf[pixels + i] = (g - GEMMA_VISION_NORM_MEAN[1]) / GEMMA_VISION_NORM_STD[1];
        buf[2 * pixels + i] = (b - GEMMA_VISION_NORM_MEAN[2]) / GEMMA_VISION_NORM_STD[2];
    }

    Tensor::from_vec(buf, (1, 3, size, size), device)
        .map_err(|e| PreprocessError::Tensor(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn cpu() -> Device {
        Device::Cpu
    }

    #[test]
    fn test_preprocess_decodes_jpeg_to_correct_shape() {
        // Generate a 50×50 RGB image and encode as PNG (no extra deps required).
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(50, 50, Rgb([200u8, 100u8, 50u8]));
        let mut bytes: Vec<u8> = Vec::new();
        let dynimg = DynamicImage::ImageRgb8(img);
        dynimg
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode png");

        let device = cpu();
        let tensor = preprocess_image_bytes(&bytes, &device).expect("preprocess");
        assert_eq!(tensor.dims(), &[1, 3, 896, 896]);
    }

    #[test]
    fn test_preprocess_normalizes_to_zero_mean_when_input_is_gray_128() {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(50, 50, Rgb([128u8, 128u8, 128u8]));
        let dynimg = DynamicImage::ImageRgb8(img);

        let device = cpu();
        let tensor = preprocess_image_dynamic(dynimg, &device).expect("preprocess");
        // (128/255 - 0.5) / 0.5 ≈ 0.0039
        let flat: Vec<f32> = tensor
            .flatten_all()
            .expect("flatten")
            .to_vec1()
            .expect("to_vec1");
        let max_abs = flat.iter().map(|v| v.abs()).fold(0f32, f32::max);
        assert!(
            max_abs < 0.01,
            "expected near-zero values for gray 128 input, got max abs {}",
            max_abs
        );
    }

    #[test]
    fn test_preprocess_rejects_invalid_bytes() {
        let device = cpu();
        let err = preprocess_image_bytes(b"not an image", &device).expect_err("should fail");
        match err {
            PreprocessError::Decode(_) => {}
            other => panic!("expected Decode error, got {:?}", other),
        }
    }
}
