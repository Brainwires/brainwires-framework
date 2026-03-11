use std::env;
use std::process::{Command, ExitCode};

struct Step {
    key: &'static str,
    name: &'static str,
    cmd: &'static [&'static str],
}

const STEPS: &[Step] = &[
    Step {
        key: "fmt",
        name: "Format",
        cmd: &["cargo", "fmt", "--all", "--check"],
    },
    Step {
        key: "check",
        name: "Check",
        cmd: &["cargo", "check", "--workspace"],
    },
    Step {
        key: "clippy",
        name: "Clippy",
        cmd: &["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
    },
    Step {
        key: "test",
        name: "Test",
        cmd: &["cargo", "test", "--workspace"],
    },
    Step {
        key: "doc",
        name: "Doc",
        cmd: &["cargo", "doc", "--workspace", "--no-deps"],
    },
];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let filter: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    if filter.contains(&"--help") || filter.contains(&"-h") {
        println!("Usage: cargo ci [step ...]");
        println!();
        println!("Steps: fmt, check, clippy, test, doc");
        println!("Run with no arguments to execute all steps.");
        return ExitCode::SUCCESS;
    }

    let steps: Vec<&Step> = if filter.is_empty() {
        STEPS.iter().collect()
    } else {
        let mut selected = Vec::new();
        for name in &filter {
            match STEPS.iter().find(|s| s.key.eq_ignore_ascii_case(name)) {
                Some(s) => selected.push(s),
                None => {
                    eprintln!("Unknown step: {name}");
                    eprintln!("Valid steps: fmt, check, clippy, test, doc");
                    return ExitCode::FAILURE;
                }
            }
        }
        selected
    };

    let total = steps.len();
    let mut passed = 0usize;
    let mut failed_names: Vec<&str> = Vec::new();

    // Match CI environment
    // SAFETY: single-threaded at this point, before spawning any child processes.
    unsafe { env::set_var("CARGO_TERM_COLOR", "always") };

    println!("Brainwires Framework — Local CI");
    println!(
        "Steps: {}",
        steps.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")
    );
    println!("============================================");

    for (i, step) in steps.iter().enumerate() {
        println!("\n[{}/{}] {}", i + 1, total, step.name);
        let status = Command::new(step.cmd[0]).args(&step.cmd[1..]).status();
        match status {
            Ok(s) if s.success() => {
                println!("PASS {}", step.name);
                passed += 1;
            }
            _ => {
                println!("FAIL {}", step.name);
                failed_names.push(step.name);
            }
        }
    }

    println!("\n============================================");
    if failed_names.is_empty() {
        println!("All {passed} steps passed.");
        ExitCode::SUCCESS
    } else {
        println!(
            "{}/{total} steps failed: {}",
            failed_names.len(),
            failed_names.join(", ")
        );
        ExitCode::FAILURE
    }
}
