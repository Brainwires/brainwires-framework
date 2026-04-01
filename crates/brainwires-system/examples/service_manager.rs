//! Example: Service Manager — safety policies, allow/deny lists, and operations.
//!
//! ```bash
//! cargo run -p brainwires-system --example service_manager --features services
//! ```

use brainwires_system::config::ServiceConfig;
use brainwires_system::services::{
    CRITICAL_SERVICES, ServiceInfo, ServiceOperation, ServiceSafety, ServiceStatus, ServiceType,
};

fn main() {
    println!("=== Service Manager Example ===\n");

    // 1. Default configuration (read-only)
    let default_config = ServiceConfig::default();
    println!("--- Default ServiceConfig ---");
    println!("  read_only          = {}", default_config.read_only);
    println!(
        "  allowed_services   = {:?}",
        default_config.allowed_services
    );
    println!(
        "  forbidden_services = {:?}",
        default_config.forbidden_services
    );
    println!();

    // 2. Critical services deny-list
    println!("--- Critical Services (always denied) ---");
    for (i, svc) in CRITICAL_SERVICES.iter().enumerate() {
        print!("  {svc}");
        if i < CRITICAL_SERVICES.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 5 == 0 {
            println!();
        }
    }
    println!("\n  Total: {} services\n", CRITICAL_SERVICES.len());

    // 3. Read-only mode (default)
    println!("--- Read-Only Mode ---");
    let read_only_safety = ServiceSafety::from_config(&default_config);

    let ops = vec![
        ("List", ServiceOperation::List),
        (
            "Status(myapp)",
            ServiceOperation::Status("myapp".to_string()),
        ),
        (
            "Logs(myapp)",
            ServiceOperation::Logs {
                name: "myapp".to_string(),
                lines: 50,
            },
        ),
        ("Start(myapp)", ServiceOperation::Start("myapp".to_string())),
        ("Stop(myapp)", ServiceOperation::Stop("myapp".to_string())),
        (
            "Restart(myapp)",
            ServiceOperation::Restart("myapp".to_string()),
        ),
    ];

    for (label, op) in &ops {
        let result = read_only_safety.check(op);
        let status = if result.is_ok() { "ALLOWED" } else { "BLOCKED" };
        let read_only = if op.is_read_only() {
            " (read-only)"
        } else {
            ""
        };
        println!("  {label}{read_only}: {status}");
    }
    println!();

    // 4. Write mode with allow-list
    println!("--- Write Mode with Allow-List ---");
    let write_config = ServiceConfig {
        read_only: false,
        allowed_services: vec![
            "myapp".to_string(),
            "myapp-worker".to_string(),
            "nginx".to_string(),
        ],
        ..Default::default()
    };
    let write_safety = ServiceSafety::from_config(&write_config);

    let test_ops = vec![
        ("Start(myapp)", ServiceOperation::Start("myapp".to_string())),
        (
            "Restart(nginx)",
            ServiceOperation::Restart("nginx".to_string()),
        ),
        (
            "Stop(postgres)",
            ServiceOperation::Stop("postgres".to_string()),
        ),
        (
            "Restart(sshd)",
            ServiceOperation::Restart("sshd".to_string()),
        ),
        (
            "Restart(dbus)",
            ServiceOperation::Restart("dbus".to_string()),
        ),
    ];

    for (label, op) in &test_ops {
        match write_safety.check(op) {
            Ok(()) => println!("  {label}: ALLOWED"),
            Err(reason) => println!("  {label}: BLOCKED — {reason}"),
        }
    }
    println!();

    // 5. Open mode (no allow-list, but still has deny-list)
    println!("--- Open Mode (empty allow-list) ---");
    let open_config = ServiceConfig {
        read_only: false,
        allowed_services: vec![], // empty = allow all non-critical
        ..Default::default()
    };
    let open_safety = ServiceSafety::from_config(&open_config);

    let test_ops = vec![
        ("Start(redis)", ServiceOperation::Start("redis".to_string())),
        (
            "Restart(myapp)",
            ServiceOperation::Restart("myapp".to_string()),
        ),
        ("Stop(sshd)", ServiceOperation::Stop("sshd".to_string())),
    ];

    for (label, op) in &test_ops {
        match open_safety.check(op) {
            Ok(()) => println!("  {label}: ALLOWED"),
            Err(reason) => println!("  {label}: BLOCKED — {reason}"),
        }
    }
    println!();

    // 6. ServiceInfo types
    println!("--- Service Info ---");
    let services = vec![
        ServiceInfo {
            name: "myapp".to_string(),
            service_type: ServiceType::DockerContainer,
            status: ServiceStatus::Running,
            pid: Some(12345),
        },
        ServiceInfo {
            name: "nginx".to_string(),
            service_type: ServiceType::Systemd,
            status: ServiceStatus::Running,
            pid: Some(1001),
        },
        ServiceInfo {
            name: "worker".to_string(),
            service_type: ServiceType::Process,
            status: ServiceStatus::Stopped,
            pid: None,
        },
    ];

    for svc in &services {
        println!(
            "  {} ({}) — {:?} (pid: {:?})",
            svc.name, svc.service_type, svc.status, svc.pid
        );
    }

    println!("\nDone.");
}
