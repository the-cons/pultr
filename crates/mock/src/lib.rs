//! Fake adapter: scripted data for tests, demos and screenshots. Works on any OS.

mod battery;

use std::sync::Arc;

pub use battery::MockBattery;
use pultr_platform::Platform;

/// A machine where every feature exists and nothing touches the real system.
pub fn platform() -> Platform {
    Platform { battery: Some(Arc::new(MockBattery::cycling())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_platform_has_a_battery() {
        assert!(platform().battery.is_some());
    }
}
