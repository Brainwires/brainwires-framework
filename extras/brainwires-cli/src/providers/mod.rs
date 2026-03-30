// Re-export provider trait from core
pub use brainwires::core::provider::Provider;

// Re-export specific items from framework providers crate (not glob, to avoid ProviderFactory collision)
pub use brainwires::providers::{
    ProviderType, ProviderConfig,
    OllamaProvider, BrainwiresHttpProvider,
    RateLimiter, RateLimitedClient,
    // Model listing
    AvailableModel, ModelCapability, ModelLister, create_model_lister,
};

// Re-export sub-modules for `use crate::providers::local_llm::Foo` patterns
pub mod local_llm {
    pub use brainwires::providers::local_llm::*;
}

// Chat provider factory (canonical factory for creating providers)
pub use brainwires::providers::ChatProviderFactory;

// CLI-specific: factory depends on SessionManager/AuthClient
mod factory;
pub use factory::*;
