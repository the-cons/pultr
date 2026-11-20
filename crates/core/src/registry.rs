//! Card types by name: `type = "battery"` in the config → a constructor that builds the card.
//!
//! Core owns the mechanism. Who registers what is the caller's business:
//! built-in cards today, shell and plugin cards later.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, bail};

use crate::{card::Card, config::CardConfig};

/// Builds one card from its `[[card]]` table. May fail, e.g. on a missing `device`.
type Constructor = Box<dyn Fn(&CardConfig) -> anyhow::Result<Arc<dyn Card>>>;

#[derive(Default)]
pub struct Registry {
    constructors: BTreeMap<String, Constructor>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Panics if `kind` is taken: two constructors for one type is a bug, not a config error.
    pub fn register(
        &mut self,
        kind: &str,
        constructor: impl Fn(&CardConfig) -> anyhow::Result<Arc<dyn Card>> + 'static,
    ) {
        let previous = self.constructors.insert(kind.to_string(), Box::new(constructor));
        assert!(previous.is_none(), "card type `{kind}` registered twice");
    }

    /// All registered types, sorted.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.constructors.keys().map(String::as_str)
    }

    pub fn build(&self, config: &CardConfig) -> anyhow::Result<Arc<dyn Card>> {
        let Some(constructor) = self.constructors.get(&config.kind) else {
            let known = self.kinds().collect::<Vec<_>>().join(", ");
            bail!("card `{}`: unknown type `{}` (known: {known})", config.id, config.kind);
        };
        constructor(config).with_context(|| format!("card `{}`", config.id))
    }

    /// Every card, in config order. Stops at the first one that fails.
    pub fn build_all(&self, configs: &[CardConfig]) -> anyhow::Result<Vec<Arc<dyn Card>>> {
        configs.iter().map(|config| self.build(config)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_trait::async_trait;

    use super::*;
    use crate::card::{CardView, State};

    /// Just an id; enough to see which constructor ran.
    struct Named(String);

    #[async_trait]
    impl Card for Named {
        fn id(&self) -> &str {
            &self.0
        }
        fn title(&self) -> &str {
            &self.0
        }
        fn refresh_interval(&self) -> Option<Duration> {
            None
        }
        async fn refresh(&self) -> anyhow::Result<CardView> {
            Ok(CardView::new(State::Ok))
        }
    }

    fn registry() -> Registry {
        let mut registry = Registry::new();
        registry.register("named", |config| Ok(Arc::new(Named(config.id.clone()))));
        registry.register("broken", |_| bail!("no device given"));
        registry
    }

    #[test]
    fn builds_registered_types() {
        let card = registry().build(&CardConfig::new("a", "named")).unwrap();
        assert_eq!(card.id(), "a");
    }

    #[test]
    fn unknown_type_lists_the_known_ones() {
        let err = registry().build(&CardConfig::new("a", "nope")).err().unwrap();
        assert_eq!(err.to_string(), "card `a`: unknown type `nope` (known: broken, named)");
    }

    #[test]
    fn constructor_errors_name_the_card() {
        let err = registry().build(&CardConfig::new("x", "broken")).err().unwrap();
        assert_eq!(format!("{err:#}"), "card `x`: no device given");
    }

    #[test]
    fn build_all_keeps_config_order() {
        let configs = [CardConfig::new("b", "named"), CardConfig::new("a", "named")];
        let cards = registry().build_all(&configs).unwrap();
        let ids: Vec<&str> = cards.iter().map(|card| card.id()).collect();
        assert_eq!(ids, ["b", "a"]);
    }

    #[test]
    #[should_panic(expected = "card type `named` registered twice")]
    fn registering_twice_is_a_bug() {
        registry().register("named", |config| Ok(Arc::new(Named(config.id.clone()))));
    }
}
