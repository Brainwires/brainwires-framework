use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A discovered camera or video capture device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    /// Zero-based device index (passed to `open_camera`).
    pub index: u32,
    /// Human-readable device name (e.g. "HD Pro Webcam C920").
    pub name: String,
    /// Additional description from the OS, if available.
    pub description: Option<String>,
}

/// Frame resolution in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// Frame rate expressed as a fraction (e.g. 30/1 = 30 fps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub fn fps(fps: u32) -> Self {
        Self {
            numerator: fps,
            denominator: 1,
        }
    }

    /// Approximate fps as a float.
    pub fn as_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator.max(1) as f64
    }
}

impl std::fmt::Display for FrameRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} fps", self.as_f64())
    }
}

/// Pixel encoding format of a camera frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    /// 24-bit RGB, packed.
    Rgb,
    /// 24-bit BGR, packed.
    Bgr,
    /// 32-bit RGBA, packed.
    Rgba,
    /// YUV 4:2:2 packed (YUYV).
    Yuv422,
    /// Motion JPEG (compressed; decode to RGB before processing).
    Mjpeg,
    /// Platform-reported format not otherwise recognised.
    Unknown,
}

/// Capture format: resolution + frame rate + pixel encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraFormat {
    pub resolution: Resolution,
    pub frame_rate: FrameRate,
    pub pixel_format: PixelFormat,
}

impl CameraFormat {
    pub fn new(resolution: Resolution, frame_rate: FrameRate, pixel_format: PixelFormat) -> Self {
        Self {
            resolution,
            frame_rate,
            pixel_format,
        }
    }

    /// Common 1080p30 RGB format.
    pub fn hd_1080p() -> Self {
        Self::new(
            Resolution::new(1920, 1080),
            FrameRate::fps(30),
            PixelFormat::Mjpeg,
        )
    }

    /// Common 720p30 format.
    pub fn hd_720p() -> Self {
        Self::new(
            Resolution::new(1280, 720),
            FrameRate::fps(30),
            PixelFormat::Mjpeg,
        )
    }

    /// Common VGA 640x480 @ 30 fps.
    pub fn vga() -> Self {
        Self::new(
            Resolution::new(640, 480),
            FrameRate::fps(30),
            PixelFormat::Mjpeg,
        )
    }
}

/// A captured video frame.
#[derive(Debug, Clone)]
pub struct CameraFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Pixel format of `data`.
    pub format: PixelFormat,
    /// Raw pixel data.
    pub data: Vec<u8>,
    /// Milliseconds since capture stream was opened.
    pub timestamp_ms: u64,
}

impl CameraFrame {
    /// Bytes per pixel for packed formats; 0 for compressed formats like MJPEG.
    pub fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            PixelFormat::Rgb | PixelFormat::Bgr => 3,
            PixelFormat::Rgba => 4,
            PixelFormat::Yuv422 => 2,
            PixelFormat::Mjpeg | PixelFormat::Unknown => 0,
        }
    }
}

/// Errors that can occur during camera operations.
#[derive(Debug, Error)]
pub enum CameraError {
    #[error("no camera found at index {0}")]
    NotFound(u32),
    #[error("failed to open camera {index}: {reason}")]
    OpenFailed { index: u32, reason: String },
    #[error("failed to capture frame: {0}")]
    CaptureFailed(String),
    #[error("format not supported: {0:?}")]
    FormatNotSupported(CameraFormat),
    #[error("camera error: {0}")]
    Other(String),
}
