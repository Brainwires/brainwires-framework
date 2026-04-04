/// Fabric directory resolution and device listing helpers.

use std::path::PathBuf;
use anyhow::Result;
use brainwires_hardware::homeauto::MatterDevice;

/// Resolve the fabric storage directory:
/// 1. Explicit `--fabric-dir` arg (if provided)
/// 2. `~/.local/share/matter-tool/` on Linux, `~/Library/Application Support/matter-tool/` on macOS
/// 3. `./.matter-tool/` as a final fallback
pub fn resolve_fabric_dir(override_path: Option<&PathBuf>) -> PathBuf {
    if let Some(p) = override_path {
        return p.clone();
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("matter-tool")
}

/// Read commissioned devices from fabric storage.
///
/// The `MatterController` persists devices as JSON under `<fabric_dir>/devices.json`.
/// This function loads them without opening a full controller session.
pub async fn load_devices(fabric_dir: &PathBuf) -> Result<Vec<MatterDevice>> {
    let path = fabric_dir.join("devices.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let devices: Vec<MatterDevice> = serde_json::from_str(&raw)?;
    Ok(devices)
}

/// Interactive "are you sure?" prompt for destructive operations.
/// Returns `true` if the user typed exactly `yes`.
pub fn confirm_destructive(prompt: &str) -> bool {
    use std::io::{self, Write};
    print!("{prompt} [type 'yes' to confirm]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim() == "yes"
}
