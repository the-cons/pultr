use std::{sync::Mutex, time::Duration};

use async_trait::async_trait;
use pultr_platform::{Battery, BatteryStatus, ChargeState, Result};

/// Below this the simulated charger gets plugged in.
const LOW: u8 = 20;

/// A battery that lives in memory.
pub struct MockBattery {
    current: Mutex<BatteryStatus>,
    cycling: bool,
}

impl MockBattery {
    /// Drains 1 % per call down to 20 %, charges back up to full, repeats. Looks alive in a demo.
    pub fn cycling() -> Self {
        Self { current: Mutex::new(discharging(80)), cycling: true }
    }

    /// Always gives the same answer. For snapshot tests.
    pub fn fixed(status: BatteryStatus) -> Self {
        Self { current: Mutex::new(status), cycling: false }
    }
}

#[async_trait]
impl Battery for MockBattery {
    async fn status(&self) -> Result<BatteryStatus> {
        // A std Mutex is fine here: it is never held across an `.await`.
        let mut current = self.current.lock().expect("mock battery lock poisoned");
        let now = *current;
        if self.cycling {
            *current = next(now);
        }
        Ok(now)
    }
}

/// Pure simulation step: easy to test, no clock involved.
fn next(s: BatteryStatus) -> BatteryStatus {
    match s.state {
        ChargeState::Discharging if s.percent <= LOW => charging(s.percent),
        ChargeState::Discharging => discharging(s.percent - 1),
        ChargeState::Charging if s.percent >= 98 => full(),
        ChargeState::Charging => charging(s.percent + 2),
        ChargeState::Full | ChargeState::NotCharging => discharging(s.percent),
    }
}

fn discharging(percent: u8) -> BatteryStatus {
    // ~10 h for a full battery
    BatteryStatus { percent, state: ChargeState::Discharging, time_remaining: Some(minutes(percent as u64 * 6)) }
}

fn charging(percent: u8) -> BatteryStatus {
    // ~1 min per missing percent
    BatteryStatus { percent, state: ChargeState::Charging, time_remaining: Some(minutes(100 - percent as u64)) }
}

fn full() -> BatteryStatus {
    BatteryStatus { percent: 100, state: ChargeState::Full, time_remaining: None }
}

fn minutes(m: u64) -> Duration {
    Duration::from_secs(m * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixed_never_changes() {
        let battery = MockBattery::fixed(full());
        for _ in 0..3 {
            assert_eq!(battery.status().await.unwrap(), full());
        }
    }

    #[tokio::test]
    async fn cycling_starts_at_80_and_drains() {
        let battery = MockBattery::cycling();
        assert_eq!(battery.status().await.unwrap().percent, 80);
        assert_eq!(battery.status().await.unwrap().percent, 79);
    }

    #[test]
    fn low_battery_starts_charging() {
        assert_eq!(next(discharging(LOW)).state, ChargeState::Charging);
    }

    #[test]
    fn charging_ends_full() {
        assert_eq!(next(charging(98)), full());
    }

    #[test]
    fn full_cycle_stays_in_bounds_and_visits_every_state() {
        let mut s = discharging(80);
        let mut seen = Vec::new();
        for _ in 0..500 {
            assert!((LOW..=100).contains(&s.percent), "{s:?}");
            if !seen.contains(&s.state) {
                seen.push(s.state);
            }
            s = next(s);
        }
        assert_eq!(seen.len(), 3, "{seen:?}"); // Discharging, Charging, Full
    }
}
