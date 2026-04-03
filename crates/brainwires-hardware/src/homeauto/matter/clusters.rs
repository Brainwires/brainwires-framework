/// Typed cluster helpers for Matter 1.3 (TLV-encoded command/attribute payloads).
///
/// The Matter interaction model uses TLV (Tag-Length-Value) encoding for all payloads.
/// These helpers produce the TLV bytes for the most common cluster interactions.
///
/// TLV encoding reference: Matter spec §A.7 (TLV Format).

use super::types::cluster_id;

// ── TLV primitives ────────────────────────────────────────────────────────────

/// Matter TLV element types (control byte upper nibble).
#[allow(dead_code)]
mod tlv {
    pub const TYPE_SIGNED_INT_1: u8 = 0x00;
    pub const TYPE_SIGNED_INT_2: u8 = 0x01;
    pub const TYPE_SIGNED_INT_4: u8 = 0x02;
    pub const TYPE_UNSIGNED_INT_1: u8 = 0x04;
    pub const TYPE_UNSIGNED_INT_2: u8 = 0x05;
    pub const TYPE_UNSIGNED_INT_4: u8 = 0x06;
    pub const TYPE_BOOL_FALSE: u8 = 0x08;
    pub const TYPE_BOOL_TRUE: u8 = 0x09;
    pub const TYPE_NULL: u8 = 0x14;
    pub const TYPE_STRUCTURE: u8 = 0x15;
    pub const TYPE_END_OF_CONTAINER: u8 = 0x18;

    pub const TAG_ANONYMOUS: u8 = 0x00; // anonymous (no tag)
    pub const TAG_CONTEXT_1: u8 = 0x20; // context-specific 1-byte tag
}

fn tlv_uint8(tag: u8, val: u8) -> Vec<u8> {
    vec![tlv::TAG_CONTEXT_1 | tlv::TYPE_UNSIGNED_INT_1, tag, val]
}

fn tlv_uint16(tag: u8, val: u16) -> Vec<u8> {
    let mut v = vec![tlv::TAG_CONTEXT_1 | tlv::TYPE_UNSIGNED_INT_2, tag];
    v.extend_from_slice(&val.to_le_bytes());
    v
}

#[allow(dead_code)]
fn tlv_bool(tag: u8, val: bool) -> Vec<u8> {
    let ty = if val { tlv::TYPE_BOOL_TRUE } else { tlv::TYPE_BOOL_FALSE };
    vec![tlv::TAG_CONTEXT_1 | ty, tag]
}

fn tlv_null(tag: u8) -> Vec<u8> {
    vec![tlv::TAG_CONTEXT_1 | tlv::TYPE_NULL, tag]
}

fn wrap_struct(inner: &[u8]) -> Vec<u8> {
    let mut v = vec![tlv::TYPE_STRUCTURE];
    v.extend_from_slice(inner);
    v.push(tlv::TYPE_END_OF_CONTAINER);
    v
}

// ── On/Off cluster (0x0006) ───────────────────────────────────────────────────

pub mod on_off {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::ON_OFF;

    // Attribute IDs
    pub const ATTR_ON_OFF: u32 = 0x0000;

    // Command IDs
    pub const CMD_OFF: u32 = 0x00;
    pub const CMD_ON: u32 = 0x01;
    pub const CMD_TOGGLE: u32 = 0x02;

    /// TLV payload for the On command (empty struct).
    pub fn on_tlv() -> Vec<u8> { wrap_struct(&[]) }
    /// TLV payload for the Off command (empty struct).
    pub fn off_tlv() -> Vec<u8> { wrap_struct(&[]) }
    /// TLV payload for the Toggle command (empty struct).
    pub fn toggle_tlv() -> Vec<u8> { wrap_struct(&[]) }
}

// ── Level Control cluster (0x0008) ───────────────────────────────────────────

pub mod level_control {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::LEVEL_CONTROL;

    pub const ATTR_CURRENT_LEVEL: u32 = 0x0000;
    pub const ATTR_REMAINING_TIME: u32 = 0x0001;
    pub const ATTR_ON_LEVEL: u32 = 0x0011;

    pub const CMD_MOVE_TO_LEVEL: u32 = 0x00;
    pub const CMD_MOVE: u32 = 0x01;
    pub const CMD_STEP: u32 = 0x02;
    pub const CMD_STOP: u32 = 0x03;
    pub const CMD_MOVE_TO_LEVEL_WITH_ON_OFF: u32 = 0x04;

    /// TLV for MoveToLevel: `{ level(0): u8, transitionTime(1): u16 | null, optionsMask(2): u8, optionsOverride(3): u8 }`
    pub fn move_to_level_tlv(level: u8, transition_time_tenths: Option<u16>) -> Vec<u8> {
        let mut inner = tlv_uint8(0, level);
        inner.extend_from_slice(&match transition_time_tenths {
            Some(t) => tlv_uint16(1, t),
            None => tlv_null(1),
        });
        inner.extend_from_slice(&tlv_uint8(2, 0)); // optionsMask
        inner.extend_from_slice(&tlv_uint8(3, 0)); // optionsOverride
        wrap_struct(&inner)
    }
}

// ── Color Control cluster (0x0300) ───────────────────────────────────────────

pub mod color_control {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::COLOR_CONTROL;

    pub const ATTR_CURRENT_HUE: u32 = 0x0000;
    pub const ATTR_CURRENT_SAT: u32 = 0x0001;
    pub const ATTR_COLOR_TEMP_MIREDS: u32 = 0x0007;
    pub const ATTR_COLOR_MODE: u32 = 0x0008;

    pub const CMD_MOVE_TO_HUE: u32 = 0x00;
    pub const CMD_MOVE_TO_SAT: u32 = 0x03;
    pub const CMD_MOVE_TO_HUE_AND_SAT: u32 = 0x06;
    pub const CMD_MOVE_TO_COLOR_TEMP: u32 = 0x0A;

    /// TLV for MoveToHueAndSaturation.
    pub fn move_to_hue_and_sat_tlv(hue: u8, sat: u8, transition_time_tenths: u16) -> Vec<u8> {
        let mut inner = tlv_uint8(0, hue);
        inner.extend_from_slice(&tlv_uint8(1, sat));
        inner.extend_from_slice(&tlv_uint16(2, transition_time_tenths));
        inner.extend_from_slice(&tlv_uint8(3, 0)); // optionsMask
        inner.extend_from_slice(&tlv_uint8(4, 0)); // optionsOverride
        wrap_struct(&inner)
    }

    /// TLV for MoveToColorTemperature.
    pub fn move_to_color_temp_tlv(mireds: u16, transition_time_tenths: u16) -> Vec<u8> {
        let mut inner = tlv_uint16(0, mireds);
        inner.extend_from_slice(&tlv_uint16(1, transition_time_tenths));
        inner.extend_from_slice(&tlv_uint8(2, 0));
        inner.extend_from_slice(&tlv_uint8(3, 0));
        wrap_struct(&inner)
    }
}

// ── Thermostat cluster (0x0201) ───────────────────────────────────────────────

pub mod thermostat {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::THERMOSTAT;

    pub const ATTR_LOCAL_TEMP: u32 = 0x0000;
    pub const ATTR_OCCUPIED_COOLING_SETPOINT: u32 = 0x0011;
    pub const ATTR_OCCUPIED_HEATING_SETPOINT: u32 = 0x0012;
    pub const ATTR_SYSTEM_MODE: u32 = 0x001C;

    pub const CMD_SET_WEEKLY_SCHEDULE: u32 = 0x01;
    pub const CMD_SET_SETPOINT_RAISE_LOWER: u32 = 0x00;

    /// TLV for SetpointRaiseLower: `{ mode(0): u8, amount(1): i8 }`
    /// `mode`: 0=Heat, 1=Cool, 2=Both. `amount`: signed 0.1°C steps.
    pub fn setpoint_raise_lower_tlv(mode: u8, amount: i8) -> Vec<u8> {
        let mut inner = tlv_uint8(0, mode);
        inner.push(tlv::TAG_CONTEXT_1 | tlv::TYPE_SIGNED_INT_1);
        inner.push(1);
        inner.push(amount as u8);
        wrap_struct(&inner)
    }
}

// ── Door Lock cluster (0x0101) ────────────────────────────────────────────────

pub mod door_lock {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::DOOR_LOCK;

    pub const ATTR_LOCK_STATE: u32 = 0x0000;
    pub const ATTR_LOCK_TYPE: u32 = 0x0001;

    pub const CMD_LOCK_DOOR: u32 = 0x00;
    pub const CMD_UNLOCK_DOOR: u32 = 0x01;

    /// TLV for LockDoor / UnlockDoor: `{ PINCode(0)?: octet_string }` (PIN optional)
    pub fn lock_tlv(pin: Option<&[u8]>) -> Vec<u8> {
        let inner = if let Some(p) = pin {
            // TLV octet string: context_tag(0) + type_octet_string + length(1B) + data
            let mut v = vec![0x30u8, 0, p.len() as u8];
            v.extend_from_slice(p);
            v
        } else {
            vec![]
        };
        wrap_struct(&inner)
    }
}

// ── Window Covering cluster (0x0102) ─────────────────────────────────────────

pub mod window_covering {
    use super::*;
    pub const CLUSTER_ID: u32 = cluster_id::WINDOW_COVERING;

    pub const ATTR_CURRENT_POSITION_LIFT_PCT: u32 = 0x0008;
    pub const ATTR_CURRENT_POSITION_TILT_PCT: u32 = 0x0009;

    pub const CMD_UP_OR_OPEN: u32 = 0x00;
    pub const CMD_DOWN_OR_CLOSE: u32 = 0x01;
    pub const CMD_STOP_MOTION: u32 = 0x02;
    pub const CMD_GO_TO_LIFT_PERCENTAGE: u32 = 0x05;
    pub const CMD_GO_TO_TILT_PERCENTAGE: u32 = 0x08;

    pub fn go_to_lift_percentage_tlv(percent: u8) -> Vec<u8> {
        let inner = tlv_uint8(0, percent);
        wrap_struct(&inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_off_command_encodes_correctly() {
        let on = on_off::on_tlv();
        assert_eq!(on, vec![tlv::TYPE_STRUCTURE, tlv::TYPE_END_OF_CONTAINER]);
        let off = on_off::off_tlv();
        assert_eq!(on, off); // both empty structs
    }

    #[test]
    fn level_control_move_to_level_tlv() {
        let tlv = level_control::move_to_level_tlv(128, Some(10));
        // Should start with structure type and end with end-of-container
        assert_eq!(tlv[0], 0x15); // TYPE_STRUCTURE
        assert_eq!(*tlv.last().unwrap(), 0x18); // END_OF_CONTAINER
        // level byte (128)
        assert!(tlv.contains(&128));
    }

    #[test]
    fn thermostat_setpoint_tlv_roundtrip() {
        let tlv = thermostat::setpoint_raise_lower_tlv(0, 10); // Heat, +1.0°C
        assert_eq!(tlv[0], 0x15); // TYPE_STRUCTURE
        assert_eq!(*tlv.last().unwrap(), 0x18);
    }

    #[test]
    fn door_lock_lock_no_pin() {
        let tlv = door_lock::lock_tlv(None);
        assert_eq!(tlv, vec![0x15, 0x18]); // empty struct
    }

    #[test]
    fn door_lock_lock_with_pin() {
        let tlv = door_lock::lock_tlv(Some(b"1234"));
        assert!(tlv.len() > 2);
        assert_eq!(tlv[0], 0x15); // structure
    }

    #[test]
    fn color_temp_tlv_encodes_mireds() {
        let tlv = color_control::move_to_color_temp_tlv(300, 10);
        assert_eq!(tlv[0], 0x15);
        // mireds 300 = 0x012C, should appear as LE bytes somewhere in TLV
        let has_mireds = tlv.windows(2).any(|w| w == [0x2C, 0x01]);
        assert!(has_mireds);
    }
}
