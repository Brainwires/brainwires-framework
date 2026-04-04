/// Output rendering helpers: pretty text or machine-readable JSON.

use brainwires_hardware::homeauto::{AttributeValue, MatterDevice};

pub struct Output {
    pub json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    /// Print a simple success message.
    pub fn ok(&self, msg: &str) {
        if self.json {
            println!("{{\"ok\":true,\"msg\":{}}}", serde_json::to_string(msg).unwrap());
        } else {
            println!("✓ {msg}");
        }
    }

    /// Print an error (does not exit).
    pub fn err(&self, msg: &str) {
        if self.json {
            eprintln!("{{\"ok\":false,\"error\":{}}}", serde_json::to_string(msg).unwrap());
        } else {
            eprintln!("✗ {msg}");
        }
    }

    /// Print a device list.
    pub fn devices(&self, devices: &[MatterDevice]) {
        if self.json {
            println!("{}", serde_json::to_string_pretty(devices).unwrap_or_else(|_| "[]".into()));
        } else if devices.is_empty() {
            println!("No commissioned devices.");
        } else {
            println!("{:<10} {:<8} {:<8} {:<20} {}", "NODE-ID", "VID", "PID", "NAME", "STATUS");
            println!("{}", "-".repeat(62));
            for d in devices {
                let name = d.name.as_deref().unwrap_or("-");
                let status = if d.online { "online" } else { "offline" };
                println!(
                    "{:<10} {:#06x}   {:#06x}   {:<20} {}",
                    d.node_id, d.vendor_id, d.product_id, name, status
                );
            }
        }
    }

    /// Print an attribute value (result of a Read operation).
    pub fn attribute(&self, node_id: u64, endpoint: u16, cluster: u32, attr: u32, value: &AttributeValue) {
        if self.json {
            let v = attribute_value_to_json(value);
            println!(
                "{{\"node_id\":{node_id},\"endpoint\":{endpoint},\"cluster\":{cluster},\"attribute\":{attr},\"value\":{v}}}"
            );
        } else {
            println!(
                "node={node_id} ep={endpoint} cluster={cluster:#010x} attr={attr:#010x} → {value}"
            );
        }
    }

    /// Print a generic key-value pair.
    pub fn kv(&self, key: &str, value: &str) {
        if self.json {
            println!(
                "{{\"{}\":{}}}",
                key,
                serde_json::to_string(value).unwrap()
            );
        } else {
            println!("{key}: {value}");
        }
    }

    /// Print raw text (used for QR codes, banners etc.) — always plain regardless of --json.
    pub fn raw(&self, msg: &str) {
        println!("{msg}");
    }
}

fn attribute_value_to_json(v: &AttributeValue) -> String {
    match v {
        AttributeValue::Bool(b) => b.to_string(),
        AttributeValue::U8(n) => n.to_string(),
        AttributeValue::U16(n) => n.to_string(),
        AttributeValue::U32(n) => n.to_string(),
        AttributeValue::U64(n) => n.to_string(),
        AttributeValue::I8(n) => n.to_string(),
        AttributeValue::I16(n) => n.to_string(),
        AttributeValue::I32(n) => n.to_string(),
        AttributeValue::F32(n) => n.to_string(),
        AttributeValue::F64(n) => n.to_string(),
        AttributeValue::String(s) => serde_json::to_string(s).unwrap(),
        AttributeValue::Bytes(b) => format!("\"{}\"", hex::encode(b)),
        AttributeValue::Null => "null".into(),
    }
}
