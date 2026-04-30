//! Multimodal vision support for Gemma-family local models.
//!
//! Two pipelines:
//!
//! **Gemma-3** (SigLIP-based):
//!   image bytes → preprocess (896×896) → SigLIP encoder → projector → splice → Gemma3 decoder
//!
//! **Gemma-4** (native vision tower):
//!   image bytes → preprocess ([0,1]) → Gemma4 VisionTower → MultimodalEmbedder → mask replace → Gemma4 TextModel

// ── Gemma-3 modules ──
pub mod gemma3_mm;
pub mod mm_pipeline;
pub mod preprocess;
pub mod projector;
pub mod siglip;
pub mod tokens;

// ── Gemma-4 modules ──
pub mod gemma4_mm;
pub mod gemma4_preprocess;

// ── Re-exports: Gemma-3 ──
pub use mm_pipeline::{Gemma3MultiModal, ImageInput, MmPipelineError};
pub use preprocess::{
    GEMMA_VISION_INPUT_SIZE, GEMMA_VISION_NORM_MEAN, GEMMA_VISION_NORM_STD, PreprocessError,
    preprocess_image_bytes, preprocess_image_dynamic,
};
pub use projector::{DEFAULT_EPS as PROJECTOR_DEFAULT_EPS, MultiModalProjector, ProjectorError};
pub use siglip::{SiglipError, SiglipVisionTower};
pub use tokens::{
    GEMMA_IMAGE_TOKEN_COUNT, TOKEN_END_OF_IMAGE, TOKEN_START_OF_IMAGE, splice_image_token_block,
};

// ── Re-exports: Gemma-4 ──
pub use gemma4_mm::{GEMMA4_IMAGE_TOKEN_ID, Gemma4MultiModal, Gemma4PipelineError};
pub use gemma4_preprocess::{
    GEMMA4_VISION_PATCH_SIZE, Gemma4PreprocessError, preprocess_image_for_gemma4,
};
