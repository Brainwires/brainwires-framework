use std::path::PathBuf;
use anyhow::Result;
use crate::fabric;
use crate::output::Output;

pub async fn run(fabric_dir: &PathBuf, out: &Output) -> Result<()> {
    let devices = fabric::load_devices(fabric_dir).await?;
    out.devices(&devices);
    Ok(())
}
