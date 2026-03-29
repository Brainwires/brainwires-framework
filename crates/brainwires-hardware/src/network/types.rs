use std::net::IpAddr;

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};

/// A physical or virtual network interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// OS-assigned interface name (e.g. "eth0", "wlan0", "lo").
    pub name: String,
    /// Interface classification.
    pub kind: InterfaceKind,
    /// MAC address, if available.
    pub mac: Option<String>,
    /// Assigned IP addresses with prefix lengths.
    pub addrs: Vec<IpNetwork>,
    /// Whether the interface is administratively up.
    pub is_up: bool,
}

/// Classification of a network interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceKind {
    /// Wired Ethernet (e.g. eth0, enp3s0).
    Wired,
    /// Wireless / Wi-Fi (e.g. wlan0, wlp2s0).
    Wireless,
    /// Loopback (lo, lo0).
    Loopback,
    /// Virtual, tunnel, or bridge interface.
    Virtual,
    /// Could not be determined.
    Unknown,
}

/// IP configuration for a single interface, including default gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpConfig {
    /// Interface name.
    pub interface: String,
    /// Assigned addresses (CIDR notation).
    pub addrs: Vec<IpNetwork>,
    /// Default gateway, if known.
    pub gateway: Option<IpAddr>,
}

/// Result of a single port probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanResult {
    /// Target host.
    pub host: IpAddr,
    /// Target port.
    pub port: u16,
    /// Observed state.
    pub state: PortState,
}

/// Observed TCP port state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortState {
    /// Connection succeeded — service is listening.
    Open,
    /// Connection refused — port is closed.
    Closed,
    /// No response within timeout — port may be filtered.
    Filtered,
}

/// A host discovered on the local network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHost {
    /// IP address.
    pub ip: IpAddr,
    /// MAC address from ARP reply, if available.
    pub mac: Option<String>,
    /// Reverse-DNS hostname, if resolved.
    pub hostname: Option<String>,
}
