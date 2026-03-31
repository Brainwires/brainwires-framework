//! Plan Mode State
//!
//! Contains the state for isolated plan mode context that persists separately
//! from the main conversation context.

use serde::{Deserialize, Serialize};

use crate::types::message::Message;
use brainwires::agent_network::ipc::DisplayMessage;

/// Plan mode session state - isolated context for planning.
///
/// When plan mode is active, the user's conversation happens in this
/// isolated context. All planning research and exploration stays here,
/// and only the final plan output is returned to the main context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModeState {
    /// Unique plan mode session ID (separate from main session)
    pub plan_session_id: String,

    /// Linked main session ID
    pub main_session_id: String,

    /// Display messages for TUI (plan mode conversations)
    pub messages: Vec<DisplayMessage>,

    /// Full conversation history for API calls
    pub conversation_history: Vec<Message>,

    /// When plan mode was entered (Unix timestamp)
    pub started_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,

    /// Optional description/focus for this planning session
    pub focus: Option<String>,

    /// Whether plan mode is actively enabled
    pub active: bool,
}

impl PlanModeState {
    /// Create a new plan mode state
    pub fn new(main_session_id: String, focus: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            plan_session_id: format!("plan-{}", uuid::Uuid::new_v4()),
            main_session_id,
            messages: Vec::new(),
            conversation_history: Vec::new(),
            started_at: now,
            updated_at: now,
            focus,
            active: true,
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Add a display message to the plan mode conversation
    pub fn add_message(&mut self, message: DisplayMessage) {
        self.messages.push(message);
        self.touch();
    }

    /// Add a message to the conversation history (for API calls)
    pub fn add_to_history(&mut self, message: Message) {
        self.conversation_history.push(message);
        self.touch();
    }

    /// Clear the plan mode conversation
    pub fn clear(&mut self) {
        self.messages.clear();
        self.conversation_history.clear();
        self.touch();
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Deactivate plan mode (keeps history for resume)
    pub fn deactivate(&mut self) {
        self.active = false;
        self.touch();
    }

    /// Reactivate plan mode
    pub fn activate(&mut self) {
        self.active = true;
        self.touch();
    }
}

/// Saved main context when entering plan mode.
///
/// This allows restoring the main conversation state when exiting plan mode.
#[derive(Debug, Clone)]
pub struct SavedMainContext {
    /// Display messages from main context
    pub messages: Vec<DisplayMessage>,

    /// Full conversation history from main context
    pub conversation_history: Vec<Message>,

    /// Scroll position
    pub scroll: u16,

    /// Current status message
    pub status: String,
}

impl SavedMainContext {
    /// Create a new saved context
    pub fn new(
        messages: Vec<DisplayMessage>,
        conversation_history: Vec<Message>,
        scroll: u16,
        status: String,
    ) -> Self {
        Self {
            messages,
            conversation_history,
            scroll,
            status,
        }
    }
}
