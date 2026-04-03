/// Example: Commission a Matter device and toggle it on/off.
///
/// Usage:
/// ```bash
/// # Commission a new device using its QR code:
/// cargo run --example matter_on_off --features matter -- commission "MT:Y.K9042C00KA0648G00"
///
/// # Run a Matter device server (expose this agent as a Matter light):
/// cargo run --example matter_on_off --features matter -- serve
/// ```

use std::env;
use anyhow::Result;
use brainwires_hardware::homeauto::matter::{
    MatterController, MatterDeviceConfig, MatterDeviceServer,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let subcommand = env::args().nth(1).unwrap_or_else(|| "serve".into());
    match subcommand.as_str() {
        "commission" => {
            let qr_code = env::args()
                .nth(2)
                .unwrap_or_else(|| "MT:Y.K9042C00KA0648G00".into());
            commission_device(&qr_code).await
        }
        "serve" => serve_as_matter_device().await,
        _ => {
            eprintln!("Usage: matter_on_off [commission <qr-code> | serve]");
            Ok(())
        }
    }
}

async fn commission_device(qr_code: &str) -> Result<()> {
    let storage = std::path::Path::new("/tmp/brainwires-matter-controller");
    let controller = MatterController::new("Brainwires Fabric", storage).await?;

    info!("Commissioning device via QR code: {qr_code}");
    let device = controller.commission_qr(qr_code, 1).await?;
    info!("Device commissioned: node_id={} VID={:#06x}", device.node_id, device.vendor_id);

    info!("Turning device on…");
    if let Err(e) = controller.on_off(&device, 1, true).await {
        // Expected until rs-matter controller API is implemented
        info!("on_off not yet wired: {e}");
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    info!("Turning device off…");
    if let Err(e) = controller.on_off(&device, 1, false).await {
        info!("on_off not yet wired: {e}");
    }

    Ok(())
}

async fn serve_as_matter_device() -> Result<()> {
    let config = MatterDeviceConfig::builder()
        .device_name("Brainwires On/Off Light")
        .vendor_id(0xFFF1)
        .product_id(0x8001)
        .discriminator(3840)
        .passcode(20202021)
        .storage_path("/tmp/brainwires-matter-device")
        .port(5540)
        .build();

    let server = MatterDeviceServer::new(config).await?;
    server.set_on_off_handler(|on| {
        info!("On/Off command received: {on}");
    });
    server.set_level_handler(|level| {
        info!("Level command received: {level}/254");
    });

    info!("QR code: {}", server.qr_code());
    info!("Pairing code: {}", server.pairing_code());
    info!("Matter device started. Scan the QR code with your Matter controller.");

    server.start().await?;
    Ok(())
}
