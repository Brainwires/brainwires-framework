use crate::error::DatasetResult;

/// Abstraction over tokenizers for token counting and encoding.
pub trait Tokenizer: Send + Sync {
    /// Encode text into a sequence of token IDs.
    fn encode(&self, text: &str) -> DatasetResult<Vec<u32>>;
    /// Decode a sequence of token IDs back into text.
    fn decode(&self, ids: &[u32]) -> DatasetResult<String>;
    /// Return the vocabulary size.
    fn vocab_size(&self) -> usize;

    /// Count tokens in a text string.
    fn count_tokens(&self, text: &str) -> DatasetResult<usize> {
        Ok(self.encode(text)?.len())
    }
}

/// HuggingFace tokenizer integration.
#[cfg(feature = "hf-tokenizer")]
pub mod hf;

/// Tiktoken tokenizer integration.
#[cfg(feature = "tiktoken")]
pub mod tiktoken;

#[cfg(feature = "hf-tokenizer")]
pub use hf::HfTokenizer;

#[cfg(feature = "tiktoken")]
pub use tiktoken::TiktokenTokenizer;
