/// Transformer block structural definitions.
pub mod transformer;
/// Model architecture configurations and presets.
pub mod config;

pub use config::{TransformerConfig, SmallLmConfig};
pub use transformer::TransformerBlock;
