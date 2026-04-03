//! BLE transport for Matter commissioning.
//!
//! Implements Matter BLE Transport Protocol (BTP) per Matter spec §4.17.
//! Gate: `matter-ble` feature (requires btleplug peripheral mode).
//!
//! # Matter BLE UUIDs
//!
//! - Service UUID  : `0000FFF6-0000-1000-8000-00805F9B34FB`
//! - C1 (write)    : `18EE2EF5-263D-4559-959F-4F9C429F9D11`
//! - C2 (indicate) : `18EE2EF5-263D-4559-959F-4F9C429F9D12`

#[cfg(feature = "matter-ble")]
pub mod peripheral {
    // Placeholder — Phase 8 implements this fully.
    // Matter BLE service UUID: 0000FFF6-0000-1000-8000-00805F9B34FB
    // C1 characteristic (write): 18EE2EF5-263D-4559-959F-4F9C429F9D11
    // C2 characteristic (indicate): 18EE2EF5-263D-4559-959F-4F9C429F9D12
    pub struct MatterBleTransport;
    impl MatterBleTransport {
        pub fn placeholder() {}
    }
}
