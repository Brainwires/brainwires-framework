use crate::fabric;
use crate::output::Output;
use anyhow::Result;
use std::path::PathBuf;

pub async fn run(fabric_dir: &PathBuf, out: &Output) -> Result<()> {
    let devices = fabric::load_devices(fabric_dir).await?;
    out.devices(&devices);
    Ok(())
}
