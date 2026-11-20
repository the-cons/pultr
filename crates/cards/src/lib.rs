mod battery;
mod placeholder;

use std::sync::Arc;

pub use battery::BatteryCard;
use placeholder::Placeholder;
use pultr_core::card::Card;
use pultr_platform::Platform;

/// The fixed grid until M§ builds it from config.
pub fn default_cards(platform: &Platform) -> Vec<Arc<dyn Card>> {
    vec![
        Arc::new(Placeholder("bluetooth")),
        Arc::new(Placeholder("sidecar")),
        Arc::new(Placeholder("audio")),
        Arc::new(Placeholder("media")),
        Arc::new(BatteryCard::new(platform.battery.clone())),
        Arc::new(Placeholder("system")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grid_has_battery_in_slot_4() {
        let cards = default_cards(&Platform::default());
        let ids: Vec<&str> = cards.iter().map(|card| card.id()).collect();
        assert_eq!(ids, ["bluetooth", "sidecar", "audio", "media", "battery", "system"]);
    }
}
