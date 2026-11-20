//! `battery`: charge, charging state and time remaining.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use pultr_core::{
    card::{Card, CardView, Line, State},
    config::CardConfig,
};
use pultr_platform::{Battery, BatteryStatus, ChargeState};

/// At or below this, a discharging battery shows as a warning.
const LOW_PERCENT: u8 = 20;

pub struct BatteryCard {
    id: String,
    title: String,
    port: Option<Arc<dyn Battery>>,
}

impl BatteryCard {
    /// `port: None` = this machine has no battery (or no adapter for it yet).
    pub fn new(config: &CardConfig, port: Option<Arc<dyn Battery>>) -> Self {
        Self { id: config.id.clone(), title: config.title_or("battery"), port }
    }
}

#[async_trait]
impl Card for BatteryCard {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn refresh_interval(&self) -> Option<Duration> {
        Some(Duration::from_secs(5))
    }

    async fn refresh(&self) -> anyhow::Result<CardView> {
        let Some(port) = &self.port else {
            return Ok(CardView::unavailable("no battery found"));
        };
        let status = port.status().await?; // platform::Error → anyhow::Error via `?`
        Ok(view(&status))
    }
}

/// Pure: status → view. All the card's logic, testable without async.
fn view(s: &BatteryStatus) -> CardView {
    let state = match s.state {
        ChargeState::Charging => State::Active,
        ChargeState::Discharging if s.percent <= LOW_PERCENT => State::Warning,
        _ => State::Ok,
    };

    let detail = match (s.state, s.time_remaining) {
        (ChargeState::Charging, Some(t)) => format!("charging · {} to full", human(t)),
        (ChargeState::Charging, None) => "charging".to_string(),
        (ChargeState::Discharging, Some(t)) => format!("{} left", human(t)),
        (ChargeState::Discharging, None) => "on battery".to_string(),
        (ChargeState::Full, _) => "full".to_string(),
        (ChargeState::NotCharging, _) => "plugged in, not charging".to_string(),
    };

    CardView::new(state)
        .line(Line::strong(format!("{} %", s.percent)))
        .line(Line::new(detail))
        .gauge(f32::from(s.percent) / 100.0)
}

/// `1 h 05 min`, `45 min`, `0 min`.
fn human(d: Duration) -> String {
    let minutes = d.as_secs() / 60;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, m) => format!("{h} h {m:02} min"),
    }
}

#[cfg(test)]
mod tests {
    use pultr_mock::MockBattery;

    use super::*;

    fn status(percent: u8, state: ChargeState, minutes: Option<u64>) -> BatteryStatus {
        BatteryStatus { percent, state, time_remaining: minutes.map(|m| Duration::from_secs(m * 60)) }
    }

    fn texts(view: &CardView) -> Vec<&str> {
        view.lines.iter().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn discharging_shows_time_left() {
        let v = view(&status(80, ChargeState::Discharging, Some(480)));
        assert_eq!(v.state, State::Ok);
        assert_eq!(texts(&v), ["80 %", "8 h 00 min left"]);
        assert_eq!(v.gauge, Some(0.8));
    }

    #[test]
    fn low_battery_is_a_warning() {
        assert_eq!(view(&status(20, ChargeState::Discharging, None)).state, State::Warning);
        assert_eq!(view(&status(21, ChargeState::Discharging, None)).state, State::Ok);
        // charging at 10 % is fine: help is on the way
        assert_eq!(view(&status(10, ChargeState::Charging, None)).state, State::Active);
    }

    #[test]
    fn every_charge_state_has_a_detail_line() {
        assert_eq!(texts(&view(&status(60, ChargeState::Charging, Some(40))))[1], "charging · 40 min to full");
        assert_eq!(texts(&view(&status(60, ChargeState::Charging, None)))[1], "charging");
        assert_eq!(texts(&view(&status(60, ChargeState::Discharging, None)))[1], "on battery");
        assert_eq!(texts(&view(&status(100, ChargeState::Full, None)))[1], "full");
        assert_eq!(texts(&view(&status(80, ChargeState::NotCharging, None)))[1], "plugged in, not charging");
    }

    #[test]
    fn human_durations() {
        let min = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(human(min(0)), "0 min");
        assert_eq!(human(min(45)), "45 min");
        assert_eq!(human(min(65)), "1 h 05 min");
        assert_eq!(human(Duration::from_secs(59)), "0 min"); // rounds down
    }

    #[tokio::test]
    async fn without_port_the_card_is_unavailable() {
        let v = card(None).refresh().await.unwrap();
        assert_eq!(v.state, State::Unavailable);
    }

    #[tokio::test]
    async fn reads_from_the_port() {
        let port = MockBattery::fixed(status(42, ChargeState::Discharging, Some(90)));
        let v = card(Some(Arc::new(port))).refresh().await.unwrap();
        assert_eq!(texts(&v), ["42 %", "1 h 30 min left"]);
    }

    fn card(port: Option<Arc<dyn Battery>>) -> BatteryCard {
        BatteryCard::new(&CardConfig::new("battery", "battery"), port)
    }
}
