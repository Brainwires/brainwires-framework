/// OpenAI fine-tuning format converter.
pub mod openai;
/// Together AI fine-tuning format converter.
pub mod together;
/// Alpaca instruction-following format converter.
pub mod alpaca;
/// ShareGPT conversation format converter.
pub mod sharegpt;
/// ChatML template format converter.
pub mod chatml;

use crate::error::DatasetResult;
use crate::types::TrainingExample;

/// Convert training examples to/from a specific provider format.
pub trait FormatConverter: Send + Sync {
    /// Name of this format (e.g., "openai", "alpaca").
    fn name(&self) -> &str;

    /// Convert a TrainingExample to this format's JSON representation.
    fn to_json(&self, example: &TrainingExample) -> DatasetResult<serde_json::Value>;

    /// Parse this format's JSON back into a TrainingExample.
    fn parse_json(&self, value: &serde_json::Value) -> DatasetResult<TrainingExample>;

    /// Convert a batch of examples to this format.
    fn to_json_batch(&self, examples: &[TrainingExample]) -> DatasetResult<Vec<serde_json::Value>> {
        examples.iter().map(|e| self.to_json(e)).collect()
    }

    /// Parse a batch of JSON values into training examples.
    fn parse_json_batch(&self, values: &[serde_json::Value]) -> DatasetResult<Vec<TrainingExample>> {
        values.iter().map(|v| self.parse_json(v)).collect()
    }
}

pub use openai::OpenAiFormat;
pub use together::TogetherFormat;
pub use alpaca::AlpacaFormat;
pub use sharegpt::ShareGptFormat;
pub use chatml::ChatMlFormat;
