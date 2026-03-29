use serde::{Deserialize, Serialize};

/// Configuration for the GPIO pin manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioConfig {
    /// Allowed (chip, line) pairs — empty means no access.
    #[serde(default)]
    pub allowed_pins: Vec<(u32, u32)>,
    /// Maximum concurrent pins an agent may hold.
    pub max_concurrent_pins: usize,
    /// Timeout in seconds before auto-releasing a pin from an unhealthy agent.
    pub auto_release_timeout_secs: u64,
}

impl Default for GpioConfig {
    fn default() -> Self {
        Self {
            allowed_pins: Vec::new(),
            max_concurrent_pins: 4,
            auto_release_timeout_secs: 300,
        }
    }
}
