/// All clap CLI structs for matter-tool.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "matter-tool",
    about = "Commission and control Matter 1.3 devices using the Brainwires Matter stack",
    version
)]
pub struct Cli {
    /// Fabric storage directory.
    #[arg(long, value_name = "DIR", global = true)]
    pub fabric_dir: Option<PathBuf>,

    /// Enable debug-level tracing.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Emit machine-readable JSON output instead of pretty text.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Commission (pair) a Matter device into the local fabric.
    Pair {
        #[command(subcommand)]
        action: PairAction,
    },
    /// OnOff cluster commands.
    Onoff {
        #[command(subcommand)]
        action: OnoffAction,
    },
    /// LevelControl cluster commands.
    Level {
        #[command(subcommand)]
        action: LevelAction,
    },
    /// Thermostat cluster commands.
    Thermostat {
        #[command(subcommand)]
        action: ThermostatAction,
    },
    /// DoorLock cluster commands.
    Doorlock {
        #[command(subcommand)]
        action: DoorlockAction,
    },
    /// Send a raw cluster command (TLV payload).
    Invoke {
        /// Node ID of the target device.
        node_id: u64,
        /// Endpoint ID (e.g. 1).
        endpoint: u16,
        /// Cluster ID in hex (e.g. 0x0006).
        #[arg(value_parser = parse_hex_u32)]
        cluster_id: u32,
        /// Command ID in hex (e.g. 0x01).
        #[arg(value_parser = parse_hex_u32)]
        command_id: u32,
        /// Optional TLV payload bytes in hex (e.g. 2801).
        payload_hex: Option<String>,
    },
    /// Read a raw cluster attribute.
    Read {
        /// Node ID of the target device.
        node_id: u64,
        /// Endpoint ID (e.g. 1).
        endpoint: u16,
        /// Cluster ID in hex (e.g. 0x0006).
        #[arg(value_parser = parse_hex_u32)]
        cluster_id: u32,
        /// Attribute ID in hex (e.g. 0x0000).
        #[arg(value_parser = parse_hex_u32)]
        attribute_id: u32,
    },
    /// Browse for Matter devices on the local network via mDNS.
    Discover {
        /// How many seconds to listen for mDNS responses.
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },
    /// Run as a Matter device server (use another controller to commission us).
    Serve {
        /// Device name broadcast in mDNS.
        #[arg(long, default_value = "Brainwires Matter Device")]
        device_name: String,
        /// Vendor ID (hex, e.g. 0xFFF1).
        #[arg(long, default_value = "0xFFF1", value_parser = parse_hex_u16)]
        vendor_id: u16,
        /// Product ID (hex, e.g. 0x8001).
        #[arg(long, default_value = "0x8001", value_parser = parse_hex_u16)]
        product_id: u16,
        /// 12-bit discriminator (0–4095).
        #[arg(long, default_value = "3840")]
        discriminator: u16,
        /// Commissioning passcode.
        #[arg(long, default_value = "20202021")]
        passcode: u32,
        /// UDP port to listen on.
        #[arg(long, default_value = "5540")]
        port: u16,
        /// Storage path for server state.
        #[arg(long)]
        storage: Option<PathBuf>,
    },
    /// List all commissioned devices in the local fabric.
    Devices,
    /// Fabric management.
    Fabric {
        #[command(subcommand)]
        action: FabricAction,
    },
}

// ── Pair ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum PairAction {
    /// Commission via QR code (MT:...).
    Qr {
        /// Node ID to assign to this device.
        node_id: u64,
        /// QR code string starting with "MT:".
        qr_code: String,
    },
    /// Commission via 11-digit manual pairing code.
    Code {
        /// Node ID to assign to this device.
        node_id: u64,
        /// 11-digit decimal manual pairing code.
        manual_code: String,
    },
    /// Commission via BLE (requires --features ble).
    Ble {
        /// Node ID to assign to this device.
        node_id: u64,
        /// Commissioning passcode.
        passcode: u32,
        /// 12-bit discriminator.
        discriminator: u16,
    },
    /// Remove a commissioned device from the local fabric.
    Unpair {
        /// Node ID to remove.
        node_id: u64,
    },
}

// ── OnOff ────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum OnoffAction {
    /// Turn the device on.
    On { node_id: u64, endpoint: u16 },
    /// Turn the device off.
    Off { node_id: u64, endpoint: u16 },
    /// Toggle the device.
    Toggle { node_id: u64, endpoint: u16 },
    /// Read the current on/off state.
    Read { node_id: u64, endpoint: u16 },
}

// ── Level ────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum LevelAction {
    /// Set the level (0–254).
    Set {
        node_id: u64,
        endpoint: u16,
        /// Level value 0–254.
        level: u8,
        /// Transition time in tenths of a second (0 = immediate).
        #[arg(long, default_value = "0")]
        transition: u16,
    },
    /// Read the current level.
    Read { node_id: u64, endpoint: u16 },
}

// ── Thermostat ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ThermostatAction {
    /// Set the occupied heating setpoint (°C).
    Setpoint {
        node_id: u64,
        endpoint: u16,
        /// Target temperature in degrees Celsius (e.g. 21.5).
        celsius: f32,
    },
    /// Read current local temperature and setpoints.
    Read { node_id: u64, endpoint: u16 },
}

// ── DoorLock ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DoorlockAction {
    /// Lock the door.
    Lock { node_id: u64, endpoint: u16 },
    /// Unlock the door.
    Unlock { node_id: u64, endpoint: u16 },
    /// Read the current lock state.
    Read { node_id: u64, endpoint: u16 },
}

// ── Fabric ───────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum FabricAction {
    /// Print fabric ID, root CA fingerprint, and commissioned node count.
    Info,
    /// Wipe all fabric storage (interactive confirmation required).
    Reset,
}

// ── Hex parsers ───────────────────────────────────────────────────────────────

fn parse_hex_u16(s: &str) -> Result<u16, String> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(s, 16).map_err(|e| format!("invalid hex u16 '{s}': {e}"))
}

fn parse_hex_u32(s: &str) -> Result<u32, String> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(s, 16).map_err(|e| format!("invalid hex u32 '{s}': {e}"))
}
