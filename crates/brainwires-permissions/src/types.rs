//! Core permission system types
//!
//! These types are defined in brainwires-core and re-exported here for
//! backwards compatibility.

pub use brainwires_core::permission::{
    AgentCapabilities, CapabilityProfile, FilesystemCapabilities, GitCapabilities, GitOperation,
    NetworkCapabilities, PathPattern, ResourceQuotas, SpawningCapabilities, ToolCapabilities,
    ToolCategory,
};
