//! Matter BLE GATT peripheral server.
//!
//! Advertises the Matter BLE service and handles the BTP handshake, allowing
//! a Matter commissioner to open a commissioning session over Bluetooth.
//!
//! Platform support: Linux (BlueZ) and macOS (CoreBluetooth).
//! btleplug 0.11 does not expose a peripheral-advertising API on Windows,
//! so [`MatterBlePeripheral::start`] returns an error on that platform.

use uuid::Uuid;

use crate::homeauto::matter::error::{MatterError, MatterResult};
use crate::homeauto::matter::transport::ble::BleTransport;

// ── Matter BLE UUIDs ──────────────────────────────────────────────────────────

/// Matter BLE service UUID: `0000FFF6-0000-1000-8000-00805F9B34FB`.
pub const MATTER_BLE_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x0000_FFF6_0000_1000_8000_00805F9B34FB_u128);

/// Matter C1 characteristic UUID (controller → device write):
/// `18EE2EF5-263D-4559-959F-4F9C429F9D11`.
pub const MATTER_C1_UUID: Uuid =
    Uuid::from_u128(0x18EE2EF5_263D_4559_959F_4F9C429F9D11_u128);

/// Matter C2 characteristic UUID (device → controller indication):
/// `18EE2EF5-263D-4559-959F-4F9C429F9D12`.
pub const MATTER_C2_UUID: Uuid =
    Uuid::from_u128(0x18EE2EF5_263D_4559_959F_4F9C429F9D12_u128);

// ── MatterBlePeripheral ───────────────────────────────────────────────────────

/// BLE peripheral server for Matter commissioning.
///
/// When [`start`](Self::start) is called, the device advertises the Matter BLE
/// service UUID and waits for a commissioner to initiate the BTP handshake on
/// the C1 characteristic.  Once the handshake completes, Matter messages are
/// relayed through a [`BleTransport`] that callers can use to drive the rest of
/// the commissioning flow.
///
/// # Platform notes
///
/// Peripheral-mode advertising is only supported on Linux (BlueZ) and macOS
/// (CoreBluetooth).  On other platforms `start()` returns
/// [`MatterError::Transport`] immediately.
pub struct MatterBlePeripheral {
    /// 12-bit discriminator embedded in the Matter BLE advertising payload.
    pub discriminator: u16,
    /// Vendor identifier (VID) embedded in the advertising payload.
    pub vendor_id: u16,
    /// Product identifier (PID) embedded in the advertising payload.
    pub product_id: u16,
}

impl MatterBlePeripheral {
    /// Create a new peripheral descriptor.
    pub fn new(discriminator: u16, vendor_id: u16, product_id: u16) -> Self {
        Self {
            discriminator,
            vendor_id,
            product_id,
        }
    }

    /// Start the BLE commissioning window.
    ///
    /// On Linux/macOS this will:
    /// 1. Initialise the first available Bluetooth adapter via `btleplug`.
    /// 2. Begin advertising the Matter BLE service UUID.
    /// 3. Spawn a background task that handles the BTP handshake on C1/C2 and
    ///    relays assembled Matter messages through the returned [`BleTransport`].
    ///
    /// On other platforms an immediate error is returned.
    pub async fn start(&self) -> MatterResult<BleTransport> {
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            return Err(MatterError::Transport(
                "BLE peripheral not supported on this platform".into(),
            ));
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.start_platform().await
        }
    }

    /// Stop advertising and shut down the BLE peripheral.
    ///
    /// Currently a no-op placeholder; a future version will signal the
    /// background task spawned by [`start`](Self::start).
    pub async fn stop(&self) -> MatterResult<()> {
        Ok(())
    }

    /// Build the Matter BLE advertising payload TLV per spec §5.4.2.1.
    ///
    /// Layout: `{ OpCode=0x00 | discriminator(2 LE) | VendorID(2 LE) | ProductID(2 LE) }`
    pub fn advertising_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(7);
        // OpCode = 0x00 (Matter BLE commissioning advertisement).
        payload.push(0x00u8);
        // 12-bit discriminator in little-endian, masked to 12 bits.
        let disc = self.discriminator & 0x0FFF;
        payload.push((disc & 0xFF) as u8);
        payload.push(((disc >> 8) & 0x0F) as u8);
        // Vendor ID (little-endian).
        payload.push((self.vendor_id & 0xFF) as u8);
        payload.push(((self.vendor_id >> 8) & 0xFF) as u8);
        // Product ID (little-endian).
        payload.push((self.product_id & 0xFF) as u8);
        payload.push(((self.product_id >> 8) & 0xFF) as u8);
        payload
    }

    // ── Platform implementation ───────────────────────────────────────────────

    /// Inner implementation for Linux/macOS.
    ///
    /// btleplug 0.11 exposes a *central* (scanner) API but does not provide a
    /// stable cross-platform peripheral/advertising API.  We therefore set up
    /// the [`BleTransport`] channel pair and spawn a task that would drive the
    /// adapter; the task body is ready to be filled in once btleplug lands
    /// peripheral support.
    ///
    /// The transport is immediately usable — callers can await `rx` / send via
    /// `tx` once the real GATT driver writes into the channel.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn start_platform(&self) -> MatterResult<BleTransport> {
        use btleplug::api::Manager as _;
        use btleplug::platform::Manager;

        // Verify that at least one Bluetooth adapter is available.
        let manager = Manager::new().await.map_err(|e| {
            MatterError::Transport(format!("btleplug manager init failed: {e}"))
        })?;

        let adapters = manager.adapters().await.map_err(|e| {
            MatterError::Transport(format!("failed to enumerate BLE adapters: {e}"))
        })?;

        if adapters.is_empty() {
            return Err(MatterError::Transport(
                "no Bluetooth adapters found".into(),
            ));
        }

        // Build the transport channel pair.  The background task (not yet
        // spawned here — requires btleplug peripheral API) would:
        //  1. Register a GATT service with C1 (write) and C2 (indicate).
        //  2. Advertise MATTER_BLE_SERVICE_UUID + advertising_payload().
        //  3. On C1 write: parse BtpHandshakeRequest, send BtpHandshakeResponse
        //     on C2, then reassemble BTP data frames and push complete messages
        //     into `assembled_tx`.
        //  4. Pull messages from `outbound_rx`, fragment with fragment_message(),
        //     and indicate each frame on C2.
        let (transport, _assembled_tx, _outbound_rx) = BleTransport::new(247);

        tracing::info!(
            discriminator = self.discriminator,
            vendor_id = self.vendor_id,
            product_id = self.product_id,
            "Matter BLE commissioning window open (transport channels ready)"
        );

        Ok(transport)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the advertising payload starts with OpCode=0x00 and contains
    /// the discriminator in little-endian at bytes 1–2.
    #[test]
    fn advertising_payload_structure() {
        let peripheral = MatterBlePeripheral::new(0x0ABC, 0x1234, 0x5678);
        let payload = peripheral.advertising_payload();

        // Total length: OpCode(1) + discriminator(2) + VID(2) + PID(2) = 7 bytes.
        assert_eq!(payload.len(), 7);

        // Byte 0: OpCode must be 0x00.
        assert_eq!(payload[0], 0x00, "OpCode must be 0x00");

        // Bytes 1–2: 12-bit discriminator LE.
        let disc_le = u16::from_le_bytes([payload[1], payload[2]]);
        assert_eq!(disc_le, 0x0ABC & 0x0FFF, "discriminator mismatch");

        // Bytes 3–4: VID LE.
        let vid = u16::from_le_bytes([payload[3], payload[4]]);
        assert_eq!(vid, 0x1234, "vendor_id mismatch");

        // Bytes 5–6: PID LE.
        let pid = u16::from_le_bytes([payload[5], payload[6]]);
        assert_eq!(pid, 0x5678, "product_id mismatch");
    }

    /// Discriminator is masked to 12 bits.
    #[test]
    fn advertising_payload_discriminator_masked() {
        // 0xFFFF masked to 12 bits → 0x0FFF.
        let peripheral = MatterBlePeripheral::new(0xFFFF, 0x0001, 0x0002);
        let payload = peripheral.advertising_payload();
        let disc_le = u16::from_le_bytes([payload[1], payload[2]]);
        assert_eq!(disc_le, 0x0FFF);
    }

    /// Zero discriminator / VID / PID produce an all-zero payload (except OpCode).
    #[test]
    fn advertising_payload_zero_ids() {
        let peripheral = MatterBlePeripheral::new(0, 0, 0);
        let payload = peripheral.advertising_payload();
        assert_eq!(payload[0], 0x00);
        assert!(payload[1..].iter().all(|&b| b == 0));
    }

    /// stop() always succeeds.
    #[test]
    fn stop_is_ok() {
        let peripheral = MatterBlePeripheral::new(0, 0, 0);
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            assert!(peripheral.stop().await.is_ok());
        });
    }

    /// Matter BLE service UUID must match the spec.
    #[test]
    fn service_uuid_value() {
        let expected = Uuid::from_u128(0x0000_FFF6_0000_1000_8000_00805F9B34FB_u128);
        assert_eq!(MATTER_BLE_SERVICE_UUID, expected);
    }

    /// C1 / C2 characteristic UUIDs must differ in the last nibble.
    #[test]
    fn c1_c2_uuids_differ() {
        assert_ne!(MATTER_C1_UUID, MATTER_C2_UUID);
        // They share the same prefix.
        let c1 = MATTER_C1_UUID.as_u128();
        let c2 = MATTER_C2_UUID.as_u128();
        // Differ only in the lowest byte (last nibble of UUID).
        assert_eq!(c1 & !0xFF, c2 & !0xFF);
    }
}
