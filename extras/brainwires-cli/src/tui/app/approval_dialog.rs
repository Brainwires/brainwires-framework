//! Approval dialog state management.
//!
//! This module contains the state and logic for the tool approval dialog.

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use crate::approval::{ApprovalAction, ApprovalDetails, ApprovalRequest, ApprovalResponse};

/// State for the approval dialog
#[derive(Debug)]
pub struct ApprovalDialogState {
    /// Current pending approval request
    pub current_request: Option<PendingApproval>,
    /// Session-level approval decisions (tool_name -> response)
    pub session_decisions: HashMap<String, ApprovalResponse>,
}

/// A pending approval request with the response channel
pub struct PendingApproval {
    /// Request ID
    pub id: String,
    /// Tool name
    pub tool_name: String,
    /// Action being performed
    pub action: ApprovalAction,
    /// Additional details
    pub details: ApprovalDetails,
    /// Channel to send response
    pub response_tx: oneshot::Sender<ApprovalResponse>,
}

// Manual Debug impl since oneshot::Sender doesn't implement Debug
impl std::fmt::Debug for PendingApproval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingApproval")
            .field("id", &self.id)
            .field("tool_name", &self.tool_name)
            .field("action", &self.action)
            .finish_non_exhaustive()
    }
}

impl ApprovalDialogState {
    /// Create a new approval dialog state
    pub fn new() -> Self {
        Self {
            current_request: None,
            session_decisions: HashMap::new(),
        }
    }

    /// Check if there's a pending approval request
    pub fn has_pending_request(&self) -> bool {
        self.current_request.is_some()
    }

    /// Set the current pending request
    pub fn set_request(&mut self, request: ApprovalRequest) {
        self.current_request = Some(PendingApproval {
            id: request.id,
            tool_name: request.tool_name,
            action: request.action,
            details: request.details,
            response_tx: request.response_tx,
        });
    }

    /// Check if we have a session decision for a tool
    pub fn get_session_decision(&self, tool_name: &str) -> Option<ApprovalResponse> {
        self.session_decisions.get(tool_name).copied()
    }

    /// Record a session decision
    pub fn record_session_decision(&mut self, tool_name: &str, response: ApprovalResponse) {
        if response.is_session_persistent() {
            self.session_decisions.insert(tool_name.to_string(), response);
        }
    }

    /// Send response for current request
    pub fn respond(&mut self, response: ApprovalResponse) -> bool {
        if let Some(pending) = self.current_request.take() {
            // Record session decision if applicable
            self.record_session_decision(&pending.tool_name, response);

            // Send response (ignore error if receiver dropped)
            let _ = pending.response_tx.send(response);
            true
        } else {
            false
        }
    }

    /// Get info about current request for display
    pub fn get_display_info(&self) -> Option<ApprovalDisplayInfo> {
        self.current_request.as_ref().map(|req| ApprovalDisplayInfo {
            tool_name: req.tool_name.clone(),
            action_description: req.action.description(),
            action_category: req.action.category(),
            severity: req.action.severity(),
            tool_description: req.details.tool_description.clone(),
            parameters: req.details.parameters.clone(),
        })
    }

    /// Clear session decisions
    pub fn clear_session_decisions(&mut self) {
        self.session_decisions.clear();
    }
}

impl Default for ApprovalDialogState {
    fn default() -> Self {
        Self::new()
    }
}

/// Display information for the approval dialog
#[derive(Debug, Clone)]
pub struct ApprovalDisplayInfo {
    pub tool_name: String,
    pub action_description: String,
    pub action_category: &'static str,
    pub severity: crate::approval::types::ApprovalSeverity,
    pub tool_description: String,
    pub parameters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_dialog_state() {
        let mut state = ApprovalDialogState::new();
        assert!(!state.has_pending_request());
        assert!(state.get_session_decision("write_file").is_none());
    }

    #[test]
    fn test_session_decisions() {
        let mut state = ApprovalDialogState::new();

        // Record session approval
        state.record_session_decision("write_file", ApprovalResponse::ApproveForSession);
        assert_eq!(
            state.get_session_decision("write_file"),
            Some(ApprovalResponse::ApproveForSession)
        );

        // Non-session response shouldn't be recorded
        state.record_session_decision("delete_file", ApprovalResponse::Approve);
        assert!(state.get_session_decision("delete_file").is_none());
    }
}
