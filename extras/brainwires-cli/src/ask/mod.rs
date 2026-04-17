//! Out-of-band "ask the user" channel.
//!
//! The executor exposes an `ask_user_question` tool that pauses the agent
//! and prompts the user through whichever UI is active. This module defines
//! the request / response types carried over the
//! `mpsc<UserQuestionRequest> + oneshot<UserQuestionResponse>` pattern
//! already used by approval (`crate::approval`) and sudo (`crate::sudo`).

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct UserQuestionRequest {
    pub id: String,
    pub question: String,
    /// If set, the user picks from these. If empty, they type free-text.
    pub options: Vec<String>,
    /// Only honored when `options` is non-empty. When `true`, the TUI may
    /// let the user tick multiple boxes.
    pub multi_select: bool,
    pub response_tx: oneshot::Sender<UserQuestionResponse>,
}

#[derive(Debug, Clone)]
pub enum UserQuestionResponse {
    /// Free-text or single-choice answer.
    Answer(String),
    /// Multi-select answer (only emitted when `multi_select` was set).
    Selected(Vec<String>),
    /// User cancelled (Esc, Ctrl+C, or non-TTY with no way to prompt).
    Cancelled,
}
