//! brainwires-chat-pwa-wasm — wasm32 entry point for the chat PWA.
//!
//! NOTE: exports here are stubs; the real chat / streaming / voice
//! bindings land in task #4. This crate exists right now only so the
//! build pipeline (wasm-pack → esbuild → sw.js patcher) has something
//! concrete to compile.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn __start() {
    console_error_panic_hook::set_once();
}

/// Returns the crate version string baked in at compile time.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

/// Initializes the chat surface. Currently a no-op; task #4 will wire up
/// providers, the streaming bridge, and the voice loop.
#[wasm_bindgen]
pub fn init() -> Result<(), JsValue> {
    Ok(())
}
