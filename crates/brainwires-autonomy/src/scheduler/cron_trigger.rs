//! Cron-based trigger that fires events on a schedule.

use cron::Schedule;
use std::str::FromStr;

use super::task_schedule::ScheduledTask;

/// Cron trigger that computes the next fire time for a scheduled task.
///
/// Accepts both standard 5-field and extended 7-field cron expressions,
/// automatically prepending "0" for seconds if needed.
pub struct CronTrigger {
    schedule: Schedule,
    task: ScheduledTask,
}

impl CronTrigger {
    /// Create a new cron trigger from a scheduled task.
    pub fn new(task: ScheduledTask) -> anyhow::Result<Self> {
        // The cron crate requires 7-field expressions (sec min hour dom mon dow year)
        // but standard cron uses 5 fields. Prepend "0" for seconds if needed.
        let expr = if task.cron_expression.split_whitespace().count() == 5 {
            format!("0 {}", task.cron_expression)
        } else {
            task.cron_expression.clone()
        };

        let schedule = Schedule::from_str(&expr).map_err(|e| {
            anyhow::anyhow!("Invalid cron expression '{}': {}", task.cron_expression, e)
        })?;

        Ok(Self { schedule, task })
    }

    /// Get the next fire time from now.
    pub fn next_fire(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.schedule.upcoming(chrono::Utc).next()
    }

    /// Get the duration until the next fire time.
    pub fn duration_until_next(&self) -> Option<std::time::Duration> {
        self.next_fire().map(|next| {
            let now = chrono::Utc::now();
            if next > now {
                (next - now)
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(1))
            } else {
                std::time::Duration::from_secs(0)
            }
        })
    }

    /// Get a reference to the scheduled task.
    pub fn task(&self) -> &ScheduledTask {
        &self.task
    }

    /// Get the task ID.
    pub fn task_id(&self) -> &str {
        &self.task.id
    }
}

#[cfg(test)]
mod tests {
    use super::super::task_schedule::ScheduledTaskType;
    use super::*;

    #[test]
    fn cron_trigger_parses_five_field_expression() {
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "*/5 * * * *".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        let trigger = CronTrigger::new(task).unwrap();
        assert!(trigger.next_fire().is_some());
    }

    #[test]
    fn cron_trigger_rejects_invalid_expression() {
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "not a cron".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        assert!(CronTrigger::new(task).is_err());
    }

    #[test]
    fn duration_until_next_returns_some() {
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "* * * * *".to_string(), // every minute
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        let trigger = CronTrigger::new(task).unwrap();
        assert!(trigger.duration_until_next().is_some());
    }
}
