//! Example: Cron Scheduler — scheduled tasks, cron triggers, and failure policies.
//!
//! ```bash
//! cargo run -p brainwires-autonomy --example cron_scheduler --features scheduler
//! ```

use brainwires_autonomy::config::SchedulerConfig;
use brainwires_autonomy::scheduler::{
    AutonomyScheduler, CronTrigger, FailurePolicy, ScheduledTask, ScheduledTaskType,
};

fn main() {
    println!("=== Cron Scheduler Example ===\n");

    // 1. Default configuration
    let config = SchedulerConfig::default();
    println!("--- SchedulerConfig ---");
    println!("  max_concurrent_tasks = {}", config.max_concurrent_tasks);
    println!("  defined tasks        = {}", config.tasks.len());
    println!();

    // 2. Create scheduled tasks
    println!("--- Scheduled Tasks ---");
    let tasks = vec![
        ScheduledTask::new(
            "quality-check".to_string(),
            "Hourly Code Quality".to_string(),
            "0 * * * *".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: "/home/user/project".to_string(),
            },
        ),
        ScheduledTask::new(
            "security-audit".to_string(),
            "Daily Security Audit".to_string(),
            "0 2 * * *".to_string(),
            ScheduledTaskType::SecurityAudit {
                repo_path: "/home/user/project".to_string(),
            },
        ),
        ScheduledTask::new(
            "dep-check".to_string(),
            "Weekly Dependency Check".to_string(),
            "0 9 * * 1".to_string(),
            ScheduledTaskType::DependencyUpdate {
                repo_path: "/home/user/project".to_string(),
            },
        ),
        ScheduledTask {
            id: "custom-lint".to_string(),
            name: "Custom Linter".to_string(),
            cron_expression: "*/30 * * * *".to_string(),
            task_type: ScheduledTaskType::CustomCommand {
                cmd: "cargo".to_string(),
                args: vec!["clippy".to_string(), "--all-targets".to_string()],
                working_dir: "/home/user/project".to_string(),
            },
            enabled: true,
            max_runtime_secs: 600,
            on_failure: FailurePolicy::Retry {
                max_retries: 2,
                backoff_secs: 120,
            },
        },
        ScheduledTask {
            id: "disabled-task".to_string(),
            name: "Disabled Task".to_string(),
            cron_expression: "0 0 * * *".to_string(),
            task_type: ScheduledTaskType::SelfImprove {
                repo_path: "/home/user/project".to_string(),
            },
            enabled: false,
            max_runtime_secs: 3600,
            on_failure: FailurePolicy::Disable,
        },
    ];

    for task in &tasks {
        println!("  {} ({})", task.name, task.id);
        println!("    cron: \"{}\"", task.cron_expression);
        println!("    enabled: {}, max_runtime: {}s", task.enabled, task.max_runtime_secs);
    }
    println!();

    // 3. Cron trigger parsing and next-fire computation
    println!("--- Cron Triggers ---");
    for task in &tasks {
        if !task.enabled {
            println!("  {}: DISABLED", task.id);
            continue;
        }
        match CronTrigger::new(task.clone()) {
            Ok(trigger) => {
                if let Some(next) = trigger.next_fire() {
                    println!("  {}: next fire at {}", trigger.task_id(), next.format("%H:%M:%S"));
                }
                if let Some(dur) = trigger.duration_until_next() {
                    println!("    ({:.0}s from now)", dur.as_secs_f64());
                }
            }
            Err(e) => println!("  {}: ERROR — {e}", task.id),
        }
    }
    println!();

    // 4. Add tasks to scheduler
    println!("--- Scheduler ---");
    let mut scheduler = AutonomyScheduler::new(config);

    for task in tasks {
        match scheduler.add_task(task) {
            Ok(()) => {}
            Err(e) => println!("  Failed to add task: {e}"),
        }
    }

    println!("  Active tasks: {}", scheduler.active_task_count());
    println!("  (disabled tasks are excluded from count)");

    // 5. Failure policies
    println!();
    println!("--- Failure Policies ---");
    let policies: Vec<(&str, FailurePolicy)> = vec![
        ("Ignore", FailurePolicy::Ignore),
        ("Retry(3x, 60s)", FailurePolicy::Retry {
            max_retries: 3,
            backoff_secs: 60,
        }),
        ("Disable", FailurePolicy::Disable),
        ("Escalate", FailurePolicy::Escalate),
    ];
    for (name, policy) in &policies {
        let json = serde_json::to_string(policy).unwrap();
        println!("  {name}: {json}");
    }

    println!("\nDone.");
}
