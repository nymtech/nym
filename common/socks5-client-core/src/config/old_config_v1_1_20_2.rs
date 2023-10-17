// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::old_config_v1_1_30::{ConfigV1_1_30, Socks5DebugV1_1_30, Socks5V1_1_30};
pub use nym_client_core::config::old_config_v1_1_20_2::ConfigV1_1_20_2 as BaseClientConfigV1_1_20_2;
pub use nym_service_providers_common::interface::ProviderInterfaceVersion;
pub use nym_socks5_requests::Socks5ProtocolVersion;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

const DEFAULT_CONNECTION_START_SURBS: u32 = 20;
const DEFAULT_PER_REQUEST_SURBS: u32 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20_2 {
    #[serde(flatten)]
    pub base: BaseClientConfigV1_1_20_2,

    pub socks5: Socks5V1_1_20_2,
}

impl From<ConfigV1_1_20_2> for ConfigV1_1_30 {
    fn from(value: ConfigV1_1_20_2) -> Self {
        ConfigV1_1_30 {
            base: value.base.into(),
            socks5: value.socks5.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5V1_1_20_2 {
    pub listening_port: u16,
    pub provider_mix_address: String,
    #[serde(default = "ProviderInterfaceVersion::new_legacy")]
    pub provider_interface_version: ProviderInterfaceVersion,
    #[serde(default = "Socks5ProtocolVersion::new_legacy")]
    pub socks5_protocol_version: Socks5ProtocolVersion,
    #[serde(default)]
    pub send_anonymously: bool,
    #[serde(default)]
    pub socks5_debug: Socks5DebugV1_1_20_2,
}

impl From<Socks5V1_1_20_2> for Socks5V1_1_30 {
    fn from(value: Socks5V1_1_20_2) -> Self {
        Socks5V1_1_30 {
            listening_port: value.listening_port,
            provider_mix_address: value.provider_mix_address,
            provider_interface_version: value.provider_interface_version,
            socks5_protocol_version: value.socks5_protocol_version,
            send_anonymously: value.send_anonymously,
            socks5_debug: value.socks5_debug.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5DebugV1_1_20_2 {
    /// Number of reply SURBs attached to each `Request::Connect` message.
    pub connection_start_surbs: u32,

    /// Number of reply SURBs attached to each `Request::Send` message.
    pub per_request_surbs: u32,
}

impl From<Socks5DebugV1_1_20_2> for Socks5DebugV1_1_30 {
    fn from(value: Socks5DebugV1_1_20_2) -> Self {
        Socks5DebugV1_1_30 {
            connection_start_surbs: value.connection_start_surbs,
            per_request_surbs: value.per_request_surbs,
        }
    }
}

impl Default for Socks5DebugV1_1_20_2 {
    fn default() -> Self {
        Socks5DebugV1_1_20_2 {
            connection_start_surbs: DEFAULT_CONNECTION_START_SURBS,
            per_request_surbs: DEFAULT_PER_REQUEST_SURBS,
        }
    }
}
