use anyhow::Result;
use clap::Subcommand;
use console::style;

use brainwires_analytics::query::AnalyticsQuery;

#[derive(Subcommand)]
pub enum AnalyticsCommands {
    /// Show cost breakdown by provider and model
    Cost {
        /// Number of days to look back (default: all time)
        #[arg(long)]
        days: Option<u32>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show tool call frequency
    Tools {
        /// Number of days to look back (default: all time)
        #[arg(long)]
        days: Option<u32>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show daily agent run summaries
    Summary {
        /// Number of days to look back (default: 7)
        #[arg(long, default_value = "7")]
        days: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show recent raw events
    Events {
        /// Maximum number of events to show
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Filter by event type (provider_call, agent_run, tool_call, mcp_request, …)
        #[arg(long)]
        r#type: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Rebuild materialized summary tables from the raw event log
    Rebuild,
}

pub async fn handle_analytics(cmd: AnalyticsCommands) -> Result<()> {
    match cmd {
        AnalyticsCommands::Cost { days, json } => handle_cost(days, json),
        AnalyticsCommands::Tools { days, json } => handle_tools(days, json),
        AnalyticsCommands::Summary { days, json } => handle_summary(days, json),
        AnalyticsCommands::Events {
            limit,
            r#type,
            json,
        } => handle_events(limit, r#type, json),
        AnalyticsCommands::Rebuild => handle_rebuild(),
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Convert `--days N` into an inclusive `(from, to)` date pair in YYYY-MM-DD.
fn date_range(days: Option<u32>) -> (Option<String>, Option<String>) {
    match days {
        None => (None, None),
        Some(n) => {
            let to = chrono::Local::now().format("%Y-%m-%d").to_string();
            let from = (chrono::Local::now() - chrono::Duration::days(n as i64))
                .format("%Y-%m-%d")
                .to_string();
            (Some(from), Some(to))
        }
    }
}

fn open_query() -> Result<AnalyticsQuery> {
    AnalyticsQuery::new_default().map_err(|e| {
        anyhow::anyhow!(
            "Could not open analytics database: {e}\n\
             Hint: run at least one chat/task command first to create the database."
        )
    })
}

// ── cost ─────────────────────────────────────────────────────────────────────

fn handle_cost(days: Option<u32>, json: bool) -> Result<()> {
    let q = open_query()?;
    q.rebuild_summaries()?;

    let (from, to) = date_range(days);
    let rows = q.cost_by_model(from.as_deref(), to.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    let period = match days {
        None => "all time".to_string(),
        Some(1) => "last 1 day".to_string(),
        Some(n) => format!("last {n} days"),
    };
    println!(
        "\n{} ({})\n",
        style("Cost by Provider / Model").cyan().bold(),
        period
    );

    if rows.is_empty() {
        println!("  {}", style("No data yet.").dim());
        println!();
        return Ok(());
    }

    let total: f64 = rows.iter().map(|r| r.total_cost_usd).sum();

    println!(
        "  {:<12} {:<40} {:>8} {:>12} {:>14} {:>10}",
        style("Date").bold(),
        style("Model").bold(),
        style("Calls").bold(),
        style("Prompt tok").bold(),
        style("Compl tok").bold(),
        style("Cost USD").bold(),
    );
    println!("  {}", "─".repeat(102));

    for r in &rows {
        let label = format!("{} / {}", r.provider, r.model);
        println!(
            "  {:<12} {:<40} {:>8} {:>12} {:>14} {:>10}",
            r.date,
            label,
            r.call_count,
            r.total_prompt_tokens,
            r.total_completion_tokens,
            format!("${:.4}", r.total_cost_usd),
        );
    }

    println!("  {}", "─".repeat(102));
    println!("  {:>96}", style(format!("Total  ${total:.4}")).bold());
    println!();

    Ok(())
}

// ── tools ─────────────────────────────────────────────────────────────────────

fn handle_tools(days: Option<u32>, json: bool) -> Result<()> {
    let q = open_query()?;
    q.rebuild_summaries()?;

    let (from, to) = date_range(days);
    let rows = q.tool_frequency(from.as_deref(), to.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    let period = match days {
        None => "all time".to_string(),
        Some(1) => "last 1 day".to_string(),
        Some(n) => format!("last {n} days"),
    };
    println!(
        "\n{} ({})\n",
        style("Tool Usage Frequency").cyan().bold(),
        period
    );

    if rows.is_empty() {
        println!("  {}", style("No data yet.").dim());
        println!();
        return Ok(());
    }

    println!(
        "  {:<12} {:<35} {:>10} {:>10} {:>10}",
        style("Date").bold(),
        style("Tool").bold(),
        style("Calls").bold(),
        style("Errors").bold(),
        style("Err %").bold(),
    );
    println!("  {}", "─".repeat(83));

    for r in &rows {
        let err_pct = if r.call_count > 0 {
            format!("{:.1}%", r.error_count as f64 / r.call_count as f64 * 100.0)
        } else {
            "0.0%".to_string()
        };
        let errors_styled = if r.error_count > 0 {
            style(r.error_count.to_string()).red()
        } else {
            style("0".to_string()).dim()
        };
        println!(
            "  {:<12} {:<35} {:>10} {:>10} {:>10}",
            r.date, r.tool_name, r.call_count, errors_styled, err_pct,
        );
    }

    println!();
    Ok(())
}

// ── summary ───────────────────────────────────────────────────────────────────

fn handle_summary(days: u32, json: bool) -> Result<()> {
    let q = open_query()?;
    q.rebuild_summaries()?;

    let (from, to) = date_range(Some(days));
    let rows = q.daily_summaries(from.as_deref(), to.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    println!(
        "\n{} (last {} days)\n",
        style("Daily Agent Run Summary").cyan().bold(),
        days
    );

    if rows.is_empty() {
        println!("  {}", style("No data yet.").dim());
        println!();
        return Ok(());
    }

    println!(
        "  {:<12} {:>8} {:>8} {:>8} {:>8} {:>12} {:>14}",
        style("Date").bold(),
        style("Runs").bold(),
        style("Success").bold(),
        style("Failed").bold(),
        style("Suc%").bold(),
        style("Cost USD").bold(),
        style("Avg iters").bold(),
    );
    println!("  {}", "─".repeat(80));

    for r in &rows {
        let suc_pct = if r.total_runs > 0 {
            format!(
                "{:.0}%",
                r.success_count as f64 / r.total_runs as f64 * 100.0
            )
        } else {
            "—".to_string()
        };
        println!(
            "  {:<12} {:>8} {:>8} {:>8} {:>8} {:>12} {:>14}",
            r.date,
            r.total_runs,
            style(r.success_count.to_string()).green(),
            if r.failure_count > 0 {
                style(r.failure_count.to_string()).red()
            } else {
                style("0".to_string()).dim()
            },
            suc_pct,
            format!("${:.4}", r.total_cost_usd),
            format!("{:.1}", r.avg_iterations),
        );
    }

    println!();
    Ok(())
}

// ── events ────────────────────────────────────────────────────────────────────

fn handle_events(limit: usize, event_type: Option<String>, json: bool) -> Result<()> {
    let q = open_query()?;
    let rows = q.recent_events(limit, event_type.as_deref())?;

    // Raw events are always output as JSON — human-readable formatting
    // would require decomposing every event variant and adds little value.
    let _ = json; // flag accepted for CLI consistency but JSON is always used
    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}

// ── rebuild ───────────────────────────────────────────────────────────────────

fn handle_rebuild() -> Result<()> {
    let q = open_query()?;
    q.rebuild_summaries()?;
    println!(
        "{} Materialized summary tables rebuilt.",
        style("✓").green()
    );
    Ok(())
}
