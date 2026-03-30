//! Debug logging utilities
//!
//! Provides debug logging that works both in CLI and TUI modes

use std::sync::Mutex;

/// Global debug message handler
static DEBUG_HANDLER: Mutex<Option<Box<dyn Fn(String) + Send + Sync>>> =
    Mutex::new(None);

/// Set the debug message handler (called by TUI to capture messages)
pub fn set_debug_handler<F>(handler: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    let mut h = DEBUG_HANDLER.lock().unwrap();
    *h = Some(Box::new(handler));
}

/// Clear the debug handler
pub fn clear_debug_handler() {
    let mut h = DEBUG_HANDLER.lock().unwrap();
    *h = None;
}

/// Log a debug message
pub fn debug<S: AsRef<str>>(msg: S) {
    let message = msg.as_ref().to_string();

    // Try to use the handler first (TUI mode)
    if let Ok(handler) = DEBUG_HANDLER.lock() {
        if let Some(ref h) = *handler {
            h(message.clone());
            return;
        }
    }

    // Fall back to eprintln for CLI mode
    eprintln!("{}", message);
}

/// Debug macro - replacement for eprintln!
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::utils::debug::debug(format!($($arg)*))
    };
}
