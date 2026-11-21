//! `bluetooth`: the radio and every paired device, in one card. Go inside to toggle a device.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail};
use async_trait::async_trait;
use pultr_core::{
    card::{ActionSpec, Card, CardView, Item, Line, State},
    config::CardConfig,
};
use pultr_platform::{Bluetooth, BtDevice, DeviceId};

const UNAVAILABLE: &str = "bluetooth not available";

/// `power`, `on`, `off`: the radio. The rest work on one device, given as argument: `toggle:AirPods Pro`.
const ACTIONS: [&str; 6] = ["power", "on", "off", "toggle", "connect", "disconnect"];

pub struct BluetoothCard {
    id: String,
    title: String,
    port: Option<Arc<dyn Bluetooth>>,
}

impl BluetoothCard {
    /// `port: None` = no Bluetooth here. The card still builds, so config keys stay valid.
    pub fn new(config: &CardConfig, port: Option<Arc<dyn Bluetooth>>) -> Self {
        Self { id: config.id.clone(), title: config.title_or("bluetooth"), port }
    }
}

#[async_trait]
impl Card for BluetoothCard {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn actions(&self) -> Vec<ActionSpec> {
        ACTIONS.map(|id| ActionSpec { id: id.to_string(), label: id.to_string(), key: None }).into()
    }

    fn refresh_interval(&self) -> Option<Duration> {
        Some(Duration::from_secs(5))
    }

    async fn refresh(&self) -> anyhow::Result<CardView> {
        let Some(port) = &self.port else {
            return Ok(CardView::unavailable(UNAVAILABLE));
        };
        let powered = port.powered().await?;
        let devices = port.devices().await?;
        Ok(view(powered, &devices))
    }

    async fn run(&self, action: &str, arg: Option<&str>) -> anyhow::Result<()> {
        let port = self.port.as_deref().context(UNAVAILABLE)?;
        match (action, arg) {
            ("power", _) => Ok(port.set_powered(!port.powered().await?).await?),
            ("on", _) => Ok(port.set_powered(true).await?),
            ("off", _) => Ok(port.set_powered(false).await?),
            ("connect", Some(wanted)) => connect(port, wanted).await,
            ("disconnect", Some(wanted)) => disconnect(port, wanted).await,
            ("toggle", Some(wanted)) => {
                if find(port, wanted).await?.connected {
                    disconnect(port, wanted).await
                } else {
                    connect(port, wanted).await
                }
            }
            ("connect" | "disconnect" | "toggle", None) => {
                bail!("`{action}` needs a device, like `{action}:AirPods Pro`")
            }
            _ => bail!("card `{}` has no action `{action}`", self.id),
        }
    }
}

/// Switches the radio on first if needed, so one key is enough. Already connected: nothing to do.
async fn connect(port: &dyn Bluetooth, wanted: &str) -> anyhow::Result<()> {
    if !port.powered().await? {
        port.set_powered(true).await?;
    }
    let device = find(port, wanted).await?;
    if !device.connected {
        port.connect(&device.id).await?;
    }
    Ok(())
}

/// Radio off or not connected: already where we want to be.
async fn disconnect(port: &dyn Bluetooth, wanted: &str) -> anyhow::Result<()> {
    if !port.powered().await? {
        return Ok(());
    }
    let device = find(port, wanted).await?;
    if device.connected {
        port.disconnect(&device.id).await?;
    }
    Ok(())
}

async fn find(port: &dyn Bluetooth, wanted: &str) -> anyhow::Result<BtDevice> {
    let devices = port.devices().await?;
    devices.into_iter().find(|d| matches(d, wanted)).with_context(|| format!("not paired: {wanted}"))
}

/// A device can be named by its address (any spelling) or its name (any case).
/// Rows pass the address; config keys usually the name.
fn matches(device: &BtDevice, wanted: &str) -> bool {
    device.id == DeviceId::new(wanted) || device.name.eq_ignore_ascii_case(wanted.trim())
}

/// Pure: radio + devices → view. One row per paired device, in the order the system lists them.
/// Only connected rows say so: "not connected" on every other row would crowd out the names.
fn view(powered: bool, devices: &[BtDevice]) -> CardView {
    let any_connected = devices.iter().any(|d| d.connected);
    let state = if any_connected { State::Active } else { State::Ok };
    let mut view = CardView::new(state).line(Line::strong(if powered { "on" } else { "off" }));

    if devices.is_empty() {
        return view.line(Line::dim("no paired devices"));
    }
    for device in devices {
        let (detail, state) = if device.connected { ("connected", State::Active) } else { ("", State::Ok) };
        view = view.item(Item::new(device.id.as_str(), &device.name).detail(detail).state(state).action("toggle"));
    }
    view
}

#[cfg(test)]
mod tests {
    use pultr_mock::{MockBluetooth, bt_device};

    use super::*;

    const AIRPODS: &str = "aa-00-00-00-00-01";

    fn devices() -> Vec<BtDevice> {
        vec![bt_device(AIRPODS, "AirPods Pro", false), bt_device("aa-00-00-00-00-02", "Magic Keyboard", true)]
    }

    fn texts(view: &CardView) -> Vec<&str> {
        view.lines.iter().map(|line| line.text.as_str()).collect()
    }

    /// `label: detail` per row.
    fn rows(view: &CardView) -> Vec<String> {
        view.items.iter().map(|item| format!("{}: {}", item.label, item.detail)).collect()
    }

    fn card(port: Option<MockBluetooth>) -> BluetoothCard {
        BluetoothCard::new(&CardConfig::new("bt", "bluetooth"), port.map(|p| Arc::new(p) as _))
    }

    async fn rows_after(card: &BluetoothCard) -> Vec<String> {
        rows(&card.refresh().await.unwrap())
    }

    // ── view ─────────────────────────────────────────────

    #[test]
    fn one_row_per_device() {
        let v = view(true, &devices());
        assert_eq!((v.state, texts(&v)), (State::Active, vec!["on"]));
        assert_eq!(rows(&v), ["AirPods Pro: ", "Magic Keyboard: connected"]);

        let airpods = &v.items[0];
        assert_eq!(
            (airpods.id.as_str(), airpods.state, airpods.action.as_deref()),
            (AIRPODS, State::Ok, Some("toggle"))
        );
        assert_eq!(v.items[1].state, State::Active);
    }

    #[test]
    fn nothing_connected_is_calm() {
        let v = view(false, &[bt_device(AIRPODS, "AirPods Pro", false)]);
        assert_eq!((v.state, texts(&v)), (State::Ok, vec!["off"]));
    }

    #[test]
    fn no_devices() {
        let v = view(true, &[]);
        assert_eq!(texts(&v), ["on", "no paired devices"]);
        assert!(v.items.is_empty());
    }

    #[test]
    fn device_by_address_or_name() {
        let airpods = bt_device(AIRPODS, "AirPods Pro", false);
        for wanted in [AIRPODS, "AA:00:00:00:00:01", "AirPods Pro", "airpods pro", " AirPods Pro "] {
            assert!(matches(&airpods, wanted), "{wanted}");
        }
        assert!(!matches(&airpods, "AirPods"));
    }

    // ── actions ──────────────────────────────────────────

    #[tokio::test]
    async fn row_toggle_connects_then_disconnects() {
        let card = card(Some(MockBluetooth::with(devices())));

        card.run("toggle", Some(AIRPODS)).await.unwrap(); // what a row sends: the address
        assert_eq!(rows_after(&card).await[0], "AirPods Pro: connected");

        card.run("toggle", Some("airpods pro")).await.unwrap(); // what a config key usually sends: the name
        assert_eq!(rows_after(&card).await[0], "AirPods Pro: ");
    }

    #[tokio::test]
    async fn connect_switches_the_radio_on_first() {
        let card = card(Some(MockBluetooth::with(devices()).off()));
        card.run("connect", Some("AirPods Pro")).await.unwrap();

        let v = card.refresh().await.unwrap();
        assert_eq!(texts(&v), ["on"]);
        assert_eq!(rows(&v)[0], "AirPods Pro: connected");
    }

    #[tokio::test]
    async fn connect_and_disconnect_are_idempotent() {
        let keyboard = card(Some(MockBluetooth::with(devices())));
        keyboard.run("connect", Some("Magic Keyboard")).await.unwrap(); // already connected
        keyboard.run("disconnect", Some("Magic Keyboard")).await.unwrap();
        keyboard.run("disconnect", Some("Magic Keyboard")).await.unwrap(); // already disconnected

        let radio_off = card(Some(MockBluetooth::with(devices()).off()));
        radio_off.run("disconnect", Some(AIRPODS)).await.unwrap(); // nothing to do, radio stays off
        assert_eq!(texts(&radio_off.refresh().await.unwrap()), ["off"]);
    }

    #[tokio::test]
    async fn power_on_and_off() {
        let card = card(Some(MockBluetooth::with(devices())));

        card.run("power", None).await.unwrap();
        assert_eq!(texts(&card.refresh().await.unwrap()), ["off"]);
        card.run("power", None).await.unwrap();
        assert_eq!(texts(&card.refresh().await.unwrap()), ["on"]);
        card.run("off", None).await.unwrap();
        card.run("off", None).await.unwrap(); // stays off
        assert_eq!(texts(&card.refresh().await.unwrap()), ["off"]);
        card.run("on", None).await.unwrap();
        assert_eq!(rows_after(&card).await, ["AirPods Pro: ", "Magic Keyboard: "]); // radio off dropped them
    }

    #[tokio::test]
    async fn device_actions_need_a_device() {
        let card = card(Some(MockBluetooth::with(devices())));
        let err = card.run("toggle", None).await.unwrap_err();
        assert_eq!(err.to_string(), "`toggle` needs a device, like `toggle:AirPods Pro`");

        let err = card.run("connect", Some("Beats")).await.unwrap_err();
        assert_eq!(err.to_string(), "not paired: Beats");

        let err = card.run("explode", None).await.unwrap_err();
        assert_eq!(err.to_string(), "card `bt` has no action `explode`");
    }

    #[tokio::test]
    async fn without_port_the_card_is_unavailable() {
        let card = card(None);
        assert_eq!(card.refresh().await.unwrap().state, State::Unavailable);
        assert_eq!(card.run("power", None).await.unwrap_err().to_string(), UNAVAILABLE);
        assert_eq!(card.actions().len(), ACTIONS.len()); // keys still validate
    }
}
