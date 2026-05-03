//! Image preprocessing for the Gemma-4 vision tower.
//!
//! Gemma-4 uses 16×16 patches and performs its own `(patch - 0.5) * 2.0`
//! scaling internally, so we deliver pixels in `[0, 1]` — NOT the
//! SigLIP-style `(x - 0.5) / 0.5` normalization used for Gemma-3.

use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;
use image::{DynamicImage, ImageError, imageops::FilterType};

/// Gemma-4 vision tower patch size.
pub const GEMMA4_VISION_PATCH_SIZE: usize = 16;

/// Errors emitted while preprocessing an image for Gemma-4.
#[derive(Debug, thiserror::Error)]
pub enum Gemma4PreprocessError {
    /// Underlying image decode failed.
    #[error("image decode failed: {0}")]
    Decode(#[from] ImageError),
    /// Building the candle tensor from the normalized buffer failed.
    #[error("tensor build failed: {0}")]
    Tensor(String),
}

/// Decode + preprocess raw image bytes for Gemma-4's vision tower.
///
/// The target resolution should be a multiple of `GEMMA4_VISION_PATCH_SIZE` (16).
/// Returns a `[1, 3, target_size, target_size]` f32 tensor in `[0, 1]`.
pub fn preprocess_image_for_gemma4(
    bytes: &[u8],
    device: &Device,
    target_size: u32,
) -> Result<Tensor, Gemma4PreprocessError> {
    let img = image::load_from_memory(bytes)?;
    preprocess_dynamic_for_gemma4(img, device, target_size)
}

/// Same as [`preprocess_image_for_gemma4`] but accepts an already-decoded image.
pub fn preprocess_dynamic_for_gemma4(
    img: DynamicImage,
    device: &Device,
    target_size: u32,
) -> Result<Tensor, Gemma4PreprocessError> {
    debug_assert_eq!(
        target_size as usize % GEMMA4_VISION_PATCH_SIZE,
        0,
        "target_size must be a multiple of {GEMMA4_VISION_PATCH_SIZE}"
    );

    let rgb = img.to_rgb8();
    let resized =
        image::imageops::resize(&rgb, target_size, target_size, FilterType::Lanczos3);

    let size = target_size as usize;
    let pixels = size * size;
    let mut buf = vec![0f32; 3 * pixels];

    let raw = resized.as_raw();
    debug_assert_eq!(raw.len(), 3 * pixels);

    for i in 0..pixels {
        buf[i] = raw[3 * i] as f32 / 255.0;
        buf[pixels + i] = raw[3 * i + 1] as f32 / 255.0;
        buf[2 * pixels + i] = raw[3 * i + 2] as f32 / 255.0;
    }

    Tensor::from_vec(buf, (1, 3, size, size), device)
        .map_err(|e| Gemma4PreprocessError::Tensor(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn cpu() -> Device {
        Device::Cpu
    }

    #[test]
    fn correct_shape_and_range() {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(50, 50, Rgb([200u8, 100u8, 50u8]));
        let mut bytes: Vec<u8> = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode png");

        let tensor = preprocess_image_for_gemma4(&bytes, &cpu(), 768).expect("preprocess");
        assert_eq!(tensor.dims(), &[1, 3, 768, 768]);

        let flat: Vec<f32> = tensor
            .flatten_all()
            .expect("flatten")
            .to_vec1()
            .expect("to_vec1");
        assert!(flat.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}
