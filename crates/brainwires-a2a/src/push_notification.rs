//! Push notification configuration types.

use serde::{Deserialize, Serialize};

/// Authentication details for push notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationInfo {
    /// HTTP authentication scheme (e.g. `Bearer`, `Basic`).
    pub scheme: String,
    /// Credentials (format depends on scheme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

/// Push notification configuration for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPushNotificationConfig {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Configuration identifier.
    #[serde(rename = "configId", skip_serializing_if = "Option::is_none")]
    pub config_id: Option<String>,
    /// Associated task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// URL where the notification should be sent.
    pub url: String,
    /// Session/task-specific token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Authentication information for sending the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthenticationInfo>,
    /// ISO 8601 timestamp when this configuration was created.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}
