use anyhow::Result;
use crate::utils::cost_tracker::CostTracker;

pub async fn handle_cost(_period: Option<String>, _reset: bool) -> Result<()> {
    let tracker = CostTracker::load().await?;
    let summary = tracker.get_usage_summary("all");
    println!("\n{}", summary);
    Ok(())
}
