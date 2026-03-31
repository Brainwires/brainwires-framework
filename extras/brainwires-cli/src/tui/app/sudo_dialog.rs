//! Sudo password dialog state management.
//!
//! This module contains the state and logic for the sudo password dialog.

use tokio::sync::oneshot;
use zeroize::Zeroizing;

use crate::sudo::{SudoPasswordRequest, SudoPasswordResponse};

/// State for the sudo password dialog
pub struct SudoDialogState {
    /// Current pending sudo request
    pub current_request: Option<PendingSudoRequest>,
    /// Masked password buffer
    password_buffer: Zeroizing<String>,
    /// Cursor position in the password buffer
    pub cursor_pos: usize,
}

/// A pending sudo password request with the response channel
pub struct PendingSudoRequest {
    /// Request ID
    pub id: String,
    /// The command that requires sudo
    pub command: String,
    /// Channel to send response
    response_tx: oneshot::Sender<SudoPasswordResponse>,
}

// Manual Debug impl since oneshot::Sender doesn't implement Debug
impl std::fmt::Debug for PendingSudoRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingSudoRequest")
            .field("id", &self.id)
            .field("command", &self.command)
            .finish_non_exhaustive()
    }
}

// Manual Debug for SudoDialogState (password_buffer should not be printed)
impl std::fmt::Debug for SudoDialogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SudoDialogState")
            .field("current_request", &self.current_request)
            .field("cursor_pos", &self.cursor_pos)
            .field("password_len", &self.password_buffer.len())
            .finish()
    }
}

impl SudoDialogState {
    /// Create a new sudo dialog state
    pub fn new() -> Self {
        Self {
            current_request: None,
            password_buffer: Zeroizing::new(String::new()),
            cursor_pos: 0,
        }
    }

    /// Set the current pending request
    pub fn set_request(&mut self, request: SudoPasswordRequest) {
        self.current_request = Some(PendingSudoRequest {
            id: request.id,
            command: request.command,
            response_tx: request.response_tx,
        });
        // Reset password buffer for new request
        self.password_buffer = Zeroizing::new(String::new());
        self.cursor_pos = 0;
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        self.password_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.password_buffer.remove(self.cursor_pos);
        }
    }

    /// Get the length of the password buffer (for masked display)
    pub fn password_len(&self) -> usize {
        self.password_buffer.len()
    }

    /// Submit the password (sends response via channel)
    pub fn submit(&mut self) -> bool {
        if self.password_buffer.is_empty() {
            return false;
        }
        if let Some(pending) = self.current_request.take() {
            let password =
                std::mem::replace(&mut self.password_buffer, Zeroizing::new(String::new()));
            let _ = pending
                .response_tx
                .send(SudoPasswordResponse::Password(password));
            self.cursor_pos = 0;
            true
        } else {
            false
        }
    }

    /// Cancel the request (sends Cancelled response via channel)
    pub fn cancel(&mut self) -> bool {
        if let Some(pending) = self.current_request.take() {
            let _ = pending.response_tx.send(SudoPasswordResponse::Cancelled);
            self.password_buffer = Zeroizing::new(String::new());
            self.cursor_pos = 0;
            true
        } else {
            false
        }
    }
}
