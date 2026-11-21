use std::sync::Mutex;

use async_trait::async_trait;
use pultr_platform::{Bluetooth, BtDevice, DeviceId, Error, Result};

/// Bluetooth that lives in memory: connecting flips a flag, nothing more.
pub struct MockBluetooth {
    state: Mutex<Radio>,
}

struct Radio {
    powered: bool,
    devices: Vec<BtDevice>,
}

impl MockBluetooth {
    /// Radio on, a few made-up devices, keyboard and mouse connected. For demos.
    pub fn demo() -> Self {
        Self::with(vec![
            bt_device("aa-00-00-00-00-01", "AirPods Pro", false),
            bt_device("aa-00-00-00-00-02", "Magic Keyboard", true),
            bt_device("aa-00-00-00-00-03", "MX Master 3S", true),
        ])
    }

    /// Radio on, exactly these devices. For tests.
    pub fn with(devices: Vec<BtDevice>) -> Self {
        Self { state: Mutex::new(Radio { powered: true, devices }) }
    }

    /// Radio off. For tests.
    pub fn off(self) -> Self {
        self.radio().set_powered(false);
        self
    }

    // A std Mutex is fine: it is never held across an `.await`.
    fn radio(&self) -> std::sync::MutexGuard<'_, Radio> {
        self.state.lock().expect("mock bluetooth lock poisoned")
    }
}

/// Shorthand for building test data.
pub fn bt_device(address: &str, name: &str, connected: bool) -> BtDevice {
    BtDevice { id: DeviceId::new(address), name: name.to_string(), connected }
}

#[async_trait]
impl Bluetooth for MockBluetooth {
    async fn powered(&self) -> Result<bool> {
        Ok(self.radio().powered)
    }

    async fn set_powered(&self, on: bool) -> Result<()> {
        self.radio().set_powered(on);
        Ok(())
    }

    async fn devices(&self) -> Result<Vec<BtDevice>> {
        Ok(self.radio().devices.clone())
    }

    async fn connect(&self, id: &DeviceId) -> Result<()> {
        self.radio().set_connected(id, true)
    }

    async fn disconnect(&self, id: &DeviceId) -> Result<()> {
        self.radio().set_connected(id, false)
    }
}

impl Radio {
    fn set_powered(&mut self, on: bool) {
        self.powered = on;
        if !on {
            // Like the real thing: radio off drops every connection.
            self.devices.iter_mut().for_each(|d| d.connected = false);
        }
    }

    fn set_connected(&mut self, id: &DeviceId, connected: bool) -> Result<()> {
        if !self.powered {
            return Err(Error::Failed("bluetooth is off".to_string()));
        }
        let Some(device) = self.devices.iter_mut().find(|d| &d.id == id) else {
            return Err(Error::Failed(format!("no paired device {id}")));
        };
        device.connected = connected;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn airpods() -> DeviceId {
        DeviceId::new("aa-00-00-00-00-01")
    }

    async fn connected(bt: &MockBluetooth, id: &DeviceId) -> bool {
        bt.devices().await.unwrap().iter().any(|d| &d.id == id && d.connected)
    }

    #[tokio::test]
    async fn connect_and_disconnect() {
        let bt = MockBluetooth::demo();
        assert!(!connected(&bt, &airpods()).await);

        bt.connect(&airpods()).await.unwrap();
        assert!(connected(&bt, &airpods()).await);

        bt.disconnect(&airpods()).await.unwrap();
        assert!(!connected(&bt, &airpods()).await);
    }

    #[tokio::test]
    async fn unknown_device_is_an_error() {
        let err = MockBluetooth::demo().connect(&DeviceId::new("ff-ff-ff-ff-ff-ff")).await.unwrap_err();
        assert_eq!(err.to_string(), "no paired device ff-ff-ff-ff-ff-ff");
    }

    #[tokio::test]
    async fn radio_off_drops_connections_and_refuses_new_ones() {
        let bt = MockBluetooth::demo();
        bt.set_powered(false).await.unwrap();

        assert!(!bt.powered().await.unwrap());
        assert!(bt.devices().await.unwrap().iter().all(|d| !d.connected));
        assert_eq!(bt.connect(&airpods()).await.unwrap_err().to_string(), "bluetooth is off");
    }

    #[tokio::test]
    async fn off_starts_powered_down_with_nothing_connected() {
        let bt = MockBluetooth::demo().off();
        assert!(!bt.powered().await.unwrap());
        assert!(bt.devices().await.unwrap().iter().all(|d| !d.connected));
    }
}
