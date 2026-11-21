use std::fmt;

use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait Bluetooth: Send + Sync {
    /// Is the Bluetooth radio on?
    async fn powered(&self) -> Result<bool>;
    async fn set_powered(&self, on: bool) -> Result<()>;

    /// Paired devices and whether each one is connected right now.
    async fn devices(&self) -> Result<Vec<BtDevice>>;
    async fn connect(&self, id: &DeviceId) -> Result<()>;
    async fn disconnect(&self, id: &DeviceId) -> Result<()>;
}

/// One paired device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtDevice {
    pub id: DeviceId,
    pub name: String,
    pub connected: bool,
}

/// A Bluetooth address in one spelling: `aa-bb-cc-dd-ee-ff`.
///
/// `AA:BB:CC:DD:EE:FF` and `aa-bb-cc-dd-ee-ff` are the same device, so both become the same id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(address: &str) -> Self {
        Self(address.trim().to_ascii_lowercase().replace(':', "-"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings_of_one_address_are_one_id() {
        let id = DeviceId::new("aa-bb-cc-dd-ee-ff");
        assert_eq!(DeviceId::new("AA:BB:CC:DD:EE:FF"), id);
        assert_eq!(DeviceId::new(" aa:bb-CC:dd-ee:FF "), id);
        assert_eq!(id.as_str(), "aa-bb-cc-dd-ee-ff");
        assert_eq!(id.to_string(), "aa-bb-cc-dd-ee-ff");
    }
}
