/**
 * Push notification configuration types.
 */

/** Authentication details for push notifications. */
export interface AuthenticationInfo {
  /** HTTP authentication scheme (e.g. `Bearer`, `Basic`). */
  scheme: string;
  /** Credentials (format depends on scheme). */
  credentials?: string;
}

/** Push notification configuration for a task. */
export interface TaskPushNotificationConfig {
  /** Optional tenant identifier. */
  tenant?: string;
  /** Configuration identifier. */
  id?: string;
  /** Associated task identifier. */
  taskId: string;
  /** URL where the notification should be sent. */
  url: string;
  /** Session/task-specific token. */
  token?: string;
  /** Authentication information for sending the notification. */
  authentication?: AuthenticationInfo;
}
