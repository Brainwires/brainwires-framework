//! Bridge from inbound JSON-RPC frames (over the WebRTC data channel)
//! into [`brainwires_agents::TaskAgent`]. Wired up in M4.
//!
//! M3 only needs a tiny ping/pong echo to prove the data channel actually
//! flows messages end-to-end. The full A2A bridge — `tasks/send`,
//! `tasks/sendSubscribe`, streaming task events, etc. — lands in M4.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// Build a JSON-RPC 2.0 reply to a `ping` request.
///
/// Mirrors the contract documented in `Phase-2-2-dial-home.md`: the home
/// daemon answers any `ping` over the `"a2a"` data channel with a
/// server-side timestamp. Used by [`crate::webrtc`]'s answerer event loop.
pub fn handle_jsonrpc_ping(req_id: Value) -> Value {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "result": { "ok": true, "ts": ts_ms },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_reply_matches_id_and_has_ok_true() {
        let reply = handle_jsonrpc_ping(json!(42));
        assert_eq!(reply["jsonrpc"], "2.0");
        assert_eq!(reply["id"], json!(42));
        assert_eq!(reply["result"]["ok"], json!(true));
        assert!(reply["result"]["ts"].as_u64().is_some());
    }

    #[test]
    fn ping_reply_supports_string_ids() {
        let reply = handle_jsonrpc_ping(json!("abc"));
        assert_eq!(reply["id"], json!("abc"));
    }
}
