//! Concrete [`StorageBackend`](super::backend::StorageBackend) implementations.

/// LanceDB backend — embedded vector database (default).
#[cfg(feature = "native")]
pub mod lance;

#[cfg(feature = "native")]
pub use lance::LanceBackend;
