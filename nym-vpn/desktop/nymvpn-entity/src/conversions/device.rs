use serde_json::json;
use nymvpn_types::device::DeviceDetails;
use uuid::Uuid;

use crate::device::Model as Device;

impl TryFrom<Device> for DeviceDetails {
    type Error = String;
    fn try_from(value: Device) -> Result<Self, Self::Error> {
        let wireguard_meta: nymvpn_types::wireguard::WireguardMetadata = match value.ipv4_address {
            Some(ipv4_address) => serde_json::from_value(json!({
                "private_key": value.private_key,
                "device_addresses": {
                    "ipv4_address": ipv4_address
                }
            })),
            None => serde_json::from_value(json!({
                "private_key": value.private_key,
            })),
        }
        .map_err(|e| format!("failed to read wireguard meta from db: {e}"))?;

        Ok(Self {
            name: value.name,
            version: value.version,
            arch: value.arch,
            unique_id: Uuid::parse_str(&value.unique_id).map_err(|e| e.to_string())?,
            device_type: value.device_type.as_str().try_into()?,
            wireguard_meta,
            created_at: value
                .created_at
                .parse()
                .map_err(|e| format!("cannot convert created_at for device: {e}"))?,
        })
    }
}

impl From<DeviceDetails> for Device {
    fn from(value: DeviceDetails) -> Self {
        Self {
            name: value.name,
            version: value.version,
            arch: value.arch,
            unique_id: value.unique_id.to_string(),
            device_type: value.device_type.into(),
            private_key: value.wireguard_meta.private_key.to_base64(),
            ipv4_address: value
                .wireguard_meta
                .device_addresses
                .map(|da| da.ipv4_address.to_string()),
            created_at: value.created_at.to_rfc3339(),
        }
    }
}
