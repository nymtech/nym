use std::fmt::Display;

use serde::{Deserialize, Serialize};
use talpid_types::net::wireguard;

use crate::nymvpn_server::DeviceAddresses;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WireguardMetadata {
    pub private_key: wireguard::PrivateKey,

    pub device_addresses: Option<DeviceAddresses>,
}

impl WireguardMetadata {
    pub fn public_key(&self) -> wireguard::PublicKey {
        self.private_key.public_key()
    }
}

impl Default for WireguardMetadata {
    fn default() -> Self {
        WireguardMetadata {
            private_key: wireguard::PrivateKey::new_from_random(),
            device_addresses: None,
        }
    }
}

impl Display for WireguardMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.device_addresses.is_some() {
            write!(
                f,
                "WireguardMetadata(public_key:{}, device_addresses:{})",
                self.private_key.public_key(),
                self.device_addresses.as_ref().unwrap(),
            )
        } else {
            write!(
                f,
                "WireguardMetadata(public_key:{}, device_addresses:None)",
                self.private_key.public_key(),
            )
        }
    }
}
