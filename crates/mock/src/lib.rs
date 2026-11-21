//! Fake adapter: scripted data for tests, demos and screenshots. Works on any OS.

mod battery;
mod bluetooth;

use std::sync::Arc;

pub use battery::MockBattery;
pub use bluetooth::{MockBluetooth, bt_device};
use pultr_platform::Platform;

/// A machine where every feature exists and nothing touches the real system.
pub fn platform() -> Platform {
    Platform { battery: Some(Arc::new(MockBattery::cycling())), bluetooth: Some(Arc::new(MockBluetooth::demo())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_platform_has_everything() {
        let platform = platform();
        assert!(platform.battery.is_some());
        assert!(platform.bluetooth.is_some());
    }
}
