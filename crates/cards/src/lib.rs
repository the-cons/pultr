//! Built-in cards, and the registry that knows them by `type`.

mod battery;
mod placeholder;

use std::sync::Arc;

pub use battery::BatteryCard;
use placeholder::Placeholder;
use pultr_core::registry::Registry;
use pultr_platform::Platform;

/// Used when there is no config file: the grid pultr has always shown.
pub const DEFAULT_CONFIG: &str = include_str!("default.toml");

/// Planned card types, shown as placeholders so example configs run today.
/// Each one leaves this list when its real card lands.
const PLANNED: &[&str] = &["bluetooth", "sidecar", "audio-output", "volume", "media", "system", "wifi"];

/// Every built-in card type, wired to this machine's ports.
pub fn registry(platform: &Platform) -> Registry {
    let mut registry = Registry::new();

    let battery = platform.battery.clone();
    registry.register("battery", move |config| Ok(Arc::new(BatteryCard::new(config, battery.clone()))));

    for kind in PLANNED {
        registry.register(kind, |config| Ok(Arc::new(Placeholder::new(config))));
    }
    registry
}

#[cfg(test)]
mod tests {
    use pultr_core::config::Config;

    use super::*;

    fn ids(config: &str) -> Vec<String> {
        let config = Config::parse(config).unwrap();
        let cards = registry(&Platform::default()).build_all(&config.cards).unwrap();
        cards.iter().map(|card| card.id().to_string()).collect()
    }

    #[test]
    fn default_config_builds_the_familiar_grid() {
        assert_eq!(ids(DEFAULT_CONFIG), ["bluetooth", "sidecar", "audio", "media", "battery", "system"]);
    }

    #[test]
    fn every_type_in_the_example_config_is_known() {
        assert_eq!(
            ids(include_str!("../../../config.example.toml")),
            ["headphones", "ipad", "battery", "music", "out"]
        );
    }

    #[test]
    fn id_and_title_come_from_the_config() {
        let config = Config::parse("[[card]]\nid = \"power\"\ntype = \"battery\"\ntitle = \"Power\"").unwrap();
        let card = registry(&Platform::default()).build(&config.cards[0]).unwrap();
        assert_eq!((card.id(), card.title()), ("power", "Power"));
    }

    #[test]
    fn battery_has_no_actions_to_put_keys_on() {
        let config = Config::parse("[[card]]\nid = \"b\"\ntype = \"battery\"\nkeys = { toggle = \"t\" }").unwrap();
        let err = registry(&Platform::default()).build(&config.cards[0]).err().unwrap();
        assert_eq!(err.to_string(), "card `b`: no action `toggle` to put a key on (it has: none)");
    }
}
