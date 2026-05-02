//! Re-export shim for the session-broker types.
//!
//! The canonical home for [`SessionBroker`] and friends is `brainwires-session`
//! (moved there in Phase 4 of the layout refactor). This module exists so
//! existing `crate::sessions::broker::*` import paths and the
//! `brainwires_tools::{SessionBroker, ...}` top-level re-exports keep working
//! without any consumer changes.

pub use brainwires_session::SessionId;
pub use brainwires_session::broker::{
    SessionBroker, SessionMessage, SessionSummary, SpawnRequest, SpawnedSession,
};
