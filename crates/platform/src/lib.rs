//! Ports: what pultr needs from the operating system, written as traits.
//!
//! Adapters (`mock`, later `macos` and `linux`) implement them.
//! `core` and the cards only ever see these traits, never an adapter.

mod battery;
mod bluetooth;
mod error;

use std::sync::Arc;

pub use battery::{Battery, BatteryStatus, ChargeState};
pub use bluetooth::{Bluetooth, BtDevice, DeviceId};
pub use error::{Error, Result};

/// Everything this machine offers. `None` = not available here.
#[derive(Clone, Default)]
pub struct Platform {
    pub battery: Option<Arc<dyn Battery>>,
    pub bluetooth: Option<Arc<dyn Bluetooth>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_platform_offers_nothing() {
        assert!(Platform::default().battery.is_none());
        assert!(Platform::default().bluetooth.is_none());
    }
}
