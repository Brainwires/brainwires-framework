//! Model listing — re-exported from `brainwires-providers` for convenience.
//!
//! The actual listing logic lives in the providers crate since it's
//! API-level (not chat-domain-level) functionality.

pub use brainwires_providers::model_listing::{
    AvailableModel, ModelCapability, ModelLister, create_model_lister,
};
