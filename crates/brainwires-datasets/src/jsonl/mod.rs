/// Streaming JSONL reader.
pub mod reader;
/// Buffered JSONL writer.
pub mod writer;

pub use reader::{JsonlReader, read_jsonl};
pub use writer::{JsonlWriter, write_jsonl};
