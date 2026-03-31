//! Approval manager for tracking session-level approval decisions

use std::collections::HashMap;
use tokio::sync::mpsc;

use super::types::{ApprovalRequest, ApprovalResponse};

/// Manages approval state for a session
///
/// Tracks which tools have been approved/denied for the session,
/// so users don't have to repeatedly approve the same tool.
pub struct ApprovalManager {
    /// Session-level approval decisions (tool_name -> response)
    session_approvals: HashMap<String, ApprovalResponse>,
    /// Channel sender for approval requests (to TUI)
    approval_tx: Option<mpsc::Sender<ApprovalRequest>>,
}

impl ApprovalManager {
    /// Create a new approval manager
    pub fn new() -> Self {
        Self {
            session_approvals: HashMap::new(),
            approval_tx: None,
        }
    }

    /// Create with an approval channel for TUI communication
    pub fn with_channel(approval_tx: mpsc::Sender<ApprovalRequest>) -> Self {
        Self {
            session_approvals: HashMap::new(),
            approval_tx: Some(approval_tx),
        }
    }

    /// Set the approval channel
    pub fn set_channel(&mut self, approval_tx: mpsc::Sender<ApprovalRequest>) {
        self.approval_tx = Some(approval_tx);
    }

    /// Get the approval channel sender (for cloning to executor)
    pub fn get_channel(&self) -> Option<mpsc::Sender<ApprovalRequest>> {
        self.approval_tx.clone()
    }

    /// Check if a tool has a remembered session decision
    pub fn get_session_decision(&self, tool_name: &str) -> Option<ApprovalResponse> {
        self.session_approvals.get(tool_name).copied()
    }

    /// Record a session-level decision
    pub fn record_session_decision(&mut self, tool_name: &str, response: ApprovalResponse) {
        if response.is_session_persistent() {
            self.session_approvals
                .insert(tool_name.to_string(), response);
        }
    }

    /// Clear all session decisions (for testing or reset)
    pub fn clear_session_decisions(&mut self) {
        self.session_approvals.clear();
    }

    /// Get count of remembered decisions
    pub fn session_decision_count(&self) -> usize {
        self.session_approvals.len()
    }

    /// Check if we have an approval channel configured
    pub fn has_channel(&self) -> bool {
        self.approval_tx.is_some()
    }

    /// Get list of tools with session decisions
    pub fn get_session_tools(&self) -> Vec<(&String, &ApprovalResponse)> {
        self.session_approvals.iter().collect()
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_decisions() {
        let mut manager = ApprovalManager::new();

        // No decision initially
        assert!(manager.get_session_decision("write_file").is_none());

        // Record a session-level approval
        manager.record_session_decision("write_file", ApprovalResponse::ApproveForSession);
        assert_eq!(
            manager.get_session_decision("write_file"),
            Some(ApprovalResponse::ApproveForSession)
        );

        // Non-session decisions shouldn't be recorded
        manager.record_session_decision("delete_file", ApprovalResponse::Approve);
        assert!(manager.get_session_decision("delete_file").is_none());

        // Session deny should be recorded
        manager.record_session_decision("execute_command", ApprovalResponse::DenyForSession);
        assert_eq!(
            manager.get_session_decision("execute_command"),
            Some(ApprovalResponse::DenyForSession)
        );

        // Check count
        assert_eq!(manager.session_decision_count(), 2);

        // Clear
        manager.clear_session_decisions();
        assert_eq!(manager.session_decision_count(), 0);
    }

    #[test]
    fn test_channel_management() {
        let manager = ApprovalManager::new();
        assert!(!manager.has_channel());

        let (tx, _rx) = mpsc::channel(16);
        let manager = ApprovalManager::with_channel(tx);
        assert!(manager.has_channel());
    }
}
