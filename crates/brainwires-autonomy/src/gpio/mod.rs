//! GPIO hardware access for autonomous agents.
//!
//! Provides safe, controlled access to GPIO pins with strict allow-lists,
//! auto-release on agent unhealthy, and direction change approval.
//! Uses `gpio-cdev` (modern character device API) as primary,
//! with `sysfs_gpio` as fallback for older kernels.

pub mod device;
pub mod pin_manager;
pub mod pwm;
pub mod safety;

pub use device::{GpioChipInfo, GpioLineInfo};
pub use pin_manager::{GpioPin, GpioPinManager};
pub use safety::GpioSafetyPolicy;
