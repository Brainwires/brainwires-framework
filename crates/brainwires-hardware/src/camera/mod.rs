//! Camera and webcam capture.
//!
//! Cross-platform video frame capture using
//! [`nokhwa`](https://crates.io/crates/nokhwa):
//! - **Linux** — Video4Linux2 (V4L2)
//! - **macOS** — AVFoundation
//! - **Windows** — Media Foundation (MSMF)
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use brainwires_hardware::camera;
//! use brainwires_hardware::camera::CameraCapture;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // List available cameras
//!     for cam in camera::list_cameras() {
//!         println!("[{}] {}", cam.index, cam.name);
//!     }
//!
//!     // Open first camera with default format
//!     let mut cap = camera::open_camera(0, None)?;
//!     println!("Format: {:?}", cap.format());
//!
//!     // Capture 5 frames
//!     for i in 0..5 {
//!         let frame = cap.capture_frame().await?;
//!         println!("Frame {i}: {}x{} @ {}ms", frame.width, frame.height, frame.timestamp_ms);
//!     }
//!
//!     cap.stop();
//!     Ok(())
//! }
//! ```
//!
//! ## Requesting a specific format
//!
//! ```rust,no_run
//! use brainwires_hardware::camera::{self, CameraFormat};
//!
//! let mut cap = camera::open_camera(0, Some(CameraFormat::hd_720p()))?;
//! ```

pub mod capture;
pub mod device;
pub mod types;

pub use capture::{CameraCapture, NokhwaCapture};
pub use device::{list_cameras, open_camera};
pub use types::{
    CameraDevice, CameraError, CameraFormat, CameraFrame, FrameRate, PixelFormat, Resolution,
};
