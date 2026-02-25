//! Provider implementations for the Brainwires Agent Framework.
//!
//! Contains concrete provider implementations, feature-gated:
//! - `brainwires_http` — HTTP backend provider (requires `native` feature)
//! - `local_llm` — Local CPU inference via llama.cpp (requires `local-llm` feature)

#[cfg(feature = "native")]
pub mod brainwires_http;

#[cfg(feature = "native")]
pub use brainwires_http::*;

pub mod local_llm;
pub use local_llm::*;
