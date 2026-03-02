use crate::error::DatasetResult;

/// Abstraction over tokenizers for token counting and encoding.
pub trait Tokenizer: Send + Sync {
    fn encode(&self, text: &str) -> DatasetResult<Vec<u32>>;
    fn decode(&self, ids: &[u32]) -> DatasetResult<String>;
    fn vocab_size(&self) -> usize;

    /// Count tokens in a text string.
    fn count_tokens(&self, text: &str) -> DatasetResult<usize> {
        Ok(self.encode(text)?.len())
    }
}

#[cfg(feature = "hf-tokenizer")]
pub mod hf;

#[cfg(feature = "tiktoken")]
pub mod tiktoken;

#[cfg(feature = "hf-tokenizer")]
pub use hf::HfTokenizer;

#[cfg(feature = "tiktoken")]
pub use tiktoken::TiktokenTokenizer;
