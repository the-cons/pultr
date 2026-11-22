//! macOS adapter: real implementations of the ports, by shelling out to proven tools.
//! Compiles on any OS (it only starts processes), but only `app` on macOS uses it.

mod blueutil;

use std::sync::Arc;

pub use blueutil::BlueUtil;
use pultr_platform::Platform;

/// What this Mac offers. Looks for helper tools once, here.
pub fn platform() -> Platform {
    Platform {
        // Always there: without blueutil the card shows how to install it instead of disappearing.
        bluetooth: Some(Arc::new(BlueUtil::find())),
        ..Platform::default()
    }
}
