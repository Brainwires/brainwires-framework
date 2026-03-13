//! # Discovery Layer
//!
//! How agents find each other on the network. The [`Discovery`] trait
//! provides a uniform interface for registering an agent's presence,
//! discovering peers, and looking up specific agents.
//!
//! ## Provided implementations
//!
//! | Implementation | Feature flag | Description |
//! |---------------|-------------|-------------|
//! | [`ManualDiscovery`] | *(always)* | Explicit peer list — no network calls |
//! | [`RegistryDiscovery`] | `registry-discovery` | HTTP-backed central agent registry |

mod traits;
mod manual;

#[cfg(feature = "registry-discovery")]
mod registry;

pub use traits::{Discovery, DiscoveryProtocol};
pub use manual::ManualDiscovery;

#[cfg(feature = "registry-discovery")]
pub use registry::RegistryDiscovery;
