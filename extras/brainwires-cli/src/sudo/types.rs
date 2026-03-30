//! Types for the sudo password system

use tokio::sync::oneshot;
use zeroize::Zeroizing;

/// A request for the user's sudo password
pub struct SudoPasswordRequest {
    /// Unique identifier for this request
    pub id: String,
    /// The command that requires sudo
    pub command: String,
    /// Channel to send the response back
    pub response_tx: oneshot::Sender<SudoPasswordResponse>,
}

/// User's response to a sudo password request
pub enum SudoPasswordResponse {
    /// User provided a password
    Password(Zeroizing<String>),
    /// User cancelled the request
    Cancelled,
}
