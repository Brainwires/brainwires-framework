//! Example: File System Reactor — rules, debouncing, and event matching.
//!
//! ```bash
//! cargo run -p brainwires-autonomy --example fs_reactor --features reactor
//! ```

use brainwires_autonomy::config::ReactorConfig;
use brainwires_autonomy::reactor::{EventDebouncer, FsEventType, ReactorAction, ReactorRule};

fn main() {
    println!("=== File System Reactor Example ===\n");

    // 1. Default configuration
    let config = ReactorConfig::default();
    println!("--- ReactorConfig ---");
    println!("  max_events_per_minute = {}", config.max_events_per_minute);
    println!("  global_debounce_ms    = {}ms", config.global_debounce_ms);
    println!("  max_watch_depth       = {}", config.max_watch_depth);
    println!();

    // 2. Define reactor rules
    println!("--- Reactor Rules ---");
    let rules = vec![
        ReactorRule {
            id: "log-errors".to_string(),
            name: "Watch Log Files".to_string(),
            watch_paths: vec!["/var/log/myapp".to_string()],
            patterns: vec!["*.log".to_string()],
            exclude_patterns: vec!["*.gz".to_string()],
            event_types: vec![FsEventType::Modified],
            debounce_ms: 2000,
            action: ReactorAction::InvestigateLogError {
                log_pattern: r"ERROR|FATAL|panic".to_string(),
            },
            enabled: true,
        },
        ReactorRule {
            id: "source-lint".to_string(),
            name: "Auto-Lint on Source Change".to_string(),
            watch_paths: vec!["src/".to_string()],
            patterns: vec!["*.rs".to_string()],
            exclude_patterns: vec!["*.generated.rs".to_string()],
            event_types: vec![FsEventType::Modified, FsEventType::Created],
            debounce_ms: 5000,
            action: ReactorAction::ExecuteCommand {
                cmd: "cargo".to_string(),
                args: vec!["clippy".to_string(), "--quiet".to_string()],
                working_dir: Some(".".to_string()),
            },
            enabled: true,
        },
        ReactorRule {
            id: "notify-delete".to_string(),
            name: "Notify on File Deletion".to_string(),
            watch_paths: vec!["config/".to_string()],
            patterns: vec![], // match everything
            exclude_patterns: vec![],
            event_types: vec![FsEventType::Deleted],
            debounce_ms: 1000,
            action: ReactorAction::Notify {
                message: "Config file deleted: ${FILE_PATH}".to_string(),
            },
            enabled: true,
        },
    ];

    for rule in &rules {
        println!("  Rule: {} ({})", rule.name, rule.id);
        println!("    watch: {:?}", rule.watch_paths);
        println!(
            "    patterns: {:?} (exclude: {:?})",
            rule.patterns, rule.exclude_patterns
        );
        println!("    events: {:?}", rule.event_types);
        println!("    debounce: {}ms", rule.debounce_ms);
    }
    println!();

    // 3. Pattern matching
    println!("--- Pattern Matching ---");
    let log_rule = &rules[0]; // *.log, exclude *.gz
    let test_paths = [
        ("app.log", true),
        ("error.log", true),
        ("app.txt", false),
        ("archive.log.gz", false),
    ];
    for (path, expected) in &test_paths {
        let matches = log_rule.matches_path(path);
        let status = if matches == *expected {
            "OK"
        } else {
            "MISMATCH"
        };
        println!("  log_rule.matches_path(\"{path}\"): {matches} [{status}]");
    }

    let src_rule = &rules[1]; // *.rs, exclude *.generated.rs
    let test_paths = [
        ("main.rs", true),
        ("lib.rs", true),
        ("bindings.generated.rs", false),
        ("config.toml", false),
    ];
    for (path, expected) in &test_paths {
        let matches = src_rule.matches_path(path);
        let status = if matches == *expected {
            "OK"
        } else {
            "MISMATCH"
        };
        println!("  src_rule.matches_path(\"{path}\"): {matches} [{status}]");
    }
    println!();

    // 4. Event type matching
    println!("--- Event Type Matching ---");
    let events = [
        FsEventType::Created,
        FsEventType::Modified,
        FsEventType::Deleted,
        FsEventType::Renamed,
    ];
    for event in &events {
        println!(
            "  log_rule ({event}): {}",
            log_rule.matches_event_type(event)
        );
    }
    println!();

    // 5. Event debouncer
    println!("--- Event Debouncer ---");
    let mut debouncer = EventDebouncer::new(1000, 5); // 1s debounce, 5/min max

    let keys = [
        "file_a.rs",
        "file_b.rs",
        "file_a.rs",
        "file_c.rs",
        "file_d.rs",
    ];
    for key in &keys {
        let processed = debouncer.should_process(key, 1000);
        println!("  should_process(\"{key}\"): {processed}");
    }
    println!("  Event count in window: {}", debouncer.event_count());

    // Hit rate limit
    println!();
    println!("  Hitting rate limit (max 5/min):");
    for i in 0..3 {
        let key = format!("extra_{i}");
        let processed = debouncer.should_process(&key, 0); // 0ms debounce
        println!("    should_process(\"{key}\"): {processed}");
    }

    println!("\nDone.");
}
