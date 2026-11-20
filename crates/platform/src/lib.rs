//! Ports: what pultr needs from the operating system, written as traits.
//!
//! Adapters (`mock`, later `macos` and `linux`) implement them.
//! `core` and the cards only ever see these traits, never an adapter.

mod battery;
mod error;

use std::sync::Arc;

pub use battery::{Battery, BatteryStatus, ChargeState};
pub use error::{Error, Result};

/// Everything this machine offers. `None` = not available here.
#[derive(Clone, Default)]
pub struct Platform {
    pub battery: Option<Arc<dyn Battery>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_platform_offers_nothing() {
        assert!(Platform::default().battery.is_none());
    }
}
