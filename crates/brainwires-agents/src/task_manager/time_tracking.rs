//! Time Tracking
//!
//! Types and functions for task time tracking and statistics.

use brainwires_core::TaskStatus;

/// Statistics about tasks
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub blocked: usize,
    pub skipped: usize,
}

/// Time tracking information for a single task
#[derive(Debug, Clone)]
pub struct TaskTimeInfo {
    pub task_id: String,
    pub description: String,
    pub status: TaskStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_secs: Option<i64>,
    pub elapsed_secs: Option<i64>,
}

impl TaskTimeInfo {
    /// Format duration as human-readable string
    pub fn format_duration(&self) -> String {
        if let Some(duration) = self.duration_secs {
            format_duration_secs(duration)
        } else if let Some(elapsed) = self.elapsed_secs {
            format!("{}...", format_duration_secs(elapsed))
        } else {
            "-".to_string()
        }
    }
}

/// Time statistics for all tasks
#[derive(Debug, Clone, Default)]
pub struct TimeStats {
    pub total_duration_secs: i64,
    pub completed_tasks: usize,
    pub average_duration_secs: Option<i64>,
    pub current_elapsed_secs: i64,
    pub in_progress_tasks: usize,
}

impl TimeStats {
    /// Format total duration as human-readable string
    pub fn format_total(&self) -> String {
        format_duration_secs(self.total_duration_secs)
    }

    /// Format average duration as human-readable string
    pub fn format_average(&self) -> String {
        if let Some(avg) = self.average_duration_secs {
            format_duration_secs(avg)
        } else {
            "-".to_string()
        }
    }

    /// Format current elapsed time (in-progress tasks)
    pub fn format_elapsed(&self) -> String {
        format_duration_secs(self.current_elapsed_secs)
    }
}

/// Format duration in seconds as human-readable string (e.g., "2m 34s", "1h 5m")
pub fn format_duration_secs(secs: i64) -> String {
    if secs < 0 {
        return "-".to_string();
    }

    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
