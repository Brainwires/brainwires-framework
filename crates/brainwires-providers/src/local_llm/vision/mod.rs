//! Multimodal vision support for Gemma-family local models.
//!
//! Architecture:
//!   image bytes → \[preprocess\] → 896×896 RGB tensor
//!                → \[siglip encoder\] → 256 × 1152 image tokens   ← Stage B
//!                → \[projector\] → 256 × <gemma hidden_size>      ← Stage C
//!                → \[token splice\] → input embeddings stream     ← Stage D
//!                → \[gemma decoder\] → text reply
//!
//! Stage A scaffolds the module + preprocess + token markers.

pub mod preprocess;
pub mod projector;
pub mod siglip;
pub mod tokens;

// Re-exports follow as later stages land.
pub use preprocess::{
    GEMMA_VISION_INPUT_SIZE, GEMMA_VISION_NORM_MEAN, GEMMA_VISION_NORM_STD, PreprocessError,
    preprocess_image_bytes, preprocess_image_dynamic,
};
pub use projector::{DEFAULT_EPS as PROJECTOR_DEFAULT_EPS, MultiModalProjector, ProjectorError};
pub use siglip::{SiglipError, SiglipVisionTower};
pub use tokens::{
    GEMMA_IMAGE_TOKEN_COUNT, TOKEN_END_OF_IMAGE, TOKEN_START_OF_IMAGE, splice_image_token_block,
};
