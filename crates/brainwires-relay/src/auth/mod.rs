/// Authentication client for the Brainwires backend.
pub mod client;
/// Authentication types (session, profile, config).
pub mod types;
/// Session persistence and management.
pub mod session;

#[cfg(feature = "auth-keyring")]
pub mod keyring;

pub use types::*;
pub use client::AuthClient;
pub use session::SessionManager;
