//! Network hardware discovery, interface enumeration, and port scanning.
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`interfaces`] | Enumerate wired/wireless NICs and their IP addresses |
//! | [`ipconfig`] | IP configuration and default gateway per interface |
//! | [`discovery`] | ARP-based host discovery on local subnets |
//! | [`portscan`] | Async TCP connect-based port scanning |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use brainwires_hardware::network;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // List interfaces
//!     for iface in network::list_interfaces() {
//!         println!("{} ({:?}) — {:?}", iface.name, iface.kind, iface.addrs);
//!     }
//!
//!     // IP config with gateways
//!     for cfg in network::get_ip_configs() {
//!         println!("{}: gateway={:?}", cfg.interface, cfg.gateway);
//!     }
//!
//!     // Port scan
//!     let results = network::scan_common_ports(
//!         "192.168.1.1".parse().unwrap(),
//!         Duration::from_millis(500),
//!     ).await;
//!     for r in results.iter().filter(|r| r.state == network::PortState::Open) {
//!         println!("Open: {}", r.port);
//!     }
//! }
//! ```

pub mod discovery;
pub mod interfaces;
pub mod ipconfig;
pub mod portscan;
pub mod types;

pub use discovery::{arp_probe, arp_scan};
pub use interfaces::list_interfaces;
pub use ipconfig::{get_interface_addrs, get_ip_configs};
pub use portscan::{scan_common_ports, scan_ports, scan_range};
pub use types::{
    DiscoveredHost, IpConfig, InterfaceKind, NetworkInterface, PortScanResult, PortState,
};
