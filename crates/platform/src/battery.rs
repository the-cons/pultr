use async_trait::async_trait;
use std::time::Duration;

use crate::Result;

#[async_trait]
pub trait Battery: Send + Sync {
    async fn status(&self) -> Result<BatteryStatus>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    /// 0–100.
    pub percent: u8,
    pub state: ChargeState,
    /// Until empty while discharging, until full while charging. `None` = unknown / still estimating.
    pub time_remaining: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeState {
    Charging,
    Discharging,
    Full,
    /// Plugged in, but not charging (e.g. macOS holding at 80 % to protect the battery).
    NotCharging,
}
