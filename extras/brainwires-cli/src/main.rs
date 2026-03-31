use anyhow::Result;
use brainwires_cli::cli::app::App;

fn main() -> Result<()> {
    // Create runtime manually so we can control shutdown behavior
    // This allows us to use shutdown_timeout() to prevent hanging on
    // lingering spawned tasks (especially after session reattachment)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let result = runtime.block_on(async {
        // Note: Logging initialization moved to individual commands
        // to support TUI mode without console pollution

        // Run CLI app
        let app = App::new();
        let outcome = app.run().await;

        // Flush and shut down analytics before the runtime exits so queued
        // events are not lost. Fail-open: analytics errors never surface to users.
        if let Some(collector) = brainwires_cli::utils::logger::analytics_collector() {
            collector.shutdown().await;
        }

        outcome
    });

    // Shutdown runtime with a timeout to avoid hanging on lingering tasks
    // Some async resources (IPC connections, spawned tasks) may not clean up
    // instantly, so we give them 100ms then force shutdown
    runtime.shutdown_timeout(std::time::Duration::from_millis(100));

    // If this was a reattached session, kill the attacher process so it doesn't
    // hang in its pause() loop waiting for SIGHUP that will never come.
    // The attacher PID was passed via environment variable from reattach_terminal().
    #[cfg(unix)]
    if let Ok(attacher_pid_str) = std::env::var("BRAINWIRES_ATTACHER_PID")
        && let Ok(attacher_pid) = attacher_pid_str.parse::<i32>()
        && attacher_pid > 0
    {
        unsafe {
            // Send SIGHUP to attacher to wake it from pause() and exit
            libc::kill(attacher_pid, libc::SIGHUP);
        }
    }

    result
}
