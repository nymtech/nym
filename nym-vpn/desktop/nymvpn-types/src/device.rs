use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    nymvpn_server::{DeviceInfo, DeviceType},
    wireguard::WireguardMetadata,
};

pub const LINUX: &str = "linux";
pub const MACOS: &str = "macos";
pub const WINDOWS: &str = "windows";
pub const ANDROID: &str = "android";
pub const IOS: &str = "ios";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceDetails {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub unique_id: Uuid,
    pub device_type: DeviceType,
    pub wireguard_meta: WireguardMetadata,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<&str> for DeviceType {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value.to_lowercase().as_str() {
            LINUX => DeviceType::Linux,
            MACOS => DeviceType::MacOS,
            WINDOWS => DeviceType::Windows,
            ANDROID => DeviceType::Android,
            IOS => DeviceType::IOS,
            val => Err(format!("invalid device type: {val}"))?,
        })
    }
}

impl From<DeviceType> for String {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Linux => LINUX,
            DeviceType::MacOS => MACOS,
            DeviceType::Windows => WINDOWS,
            DeviceType::IOS => IOS,
            DeviceType::Android => ANDROID,
        }
        .into()
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // WARN: do not use .to_string(): stack overflow from recursion
        write!(f, "{}", String::from(self.clone()))
    }
}

impl Display for DeviceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeviceDetails(name: {}, version: {}, arch: {}, unique_id: {}, device_type: {}, wireguard_meta: {}, created_at: {})",
            self.name, self.version, self.arch, self.unique_id, self.device_type, self.wireguard_meta, self.created_at
        )
    }
}

impl From<DeviceDetails> for DeviceInfo {
    fn from(value: DeviceDetails) -> Self {
        Self {
            name: value.name,
            version: value.version,
            arch: value.arch,
            public_key: value.wireguard_meta.public_key().to_base64(),
            unique_id: value.unique_id,
            device_type: value.device_type,
        }
    }
}
