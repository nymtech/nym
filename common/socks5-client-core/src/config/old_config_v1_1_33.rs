// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{Config, Socks5, Socks5Debug};
pub use nym_client_core::config::old_config_v1_1_33::ConfigV1_1_33 as BaseClientConfigV1_1_33;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::SocketAddr;

// TODO: those should really be redefined here in case we change them...
use nym_service_providers_common::interface::ProviderInterfaceVersion;
use nym_socks5_requests::Socks5ProtocolVersion;

const DEFAULT_CONNECTION_START_SURBS: u32 = 20;
const DEFAULT_PER_REQUEST_SURBS: u32 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_33 {
    #[serde(flatten)]
    pub base: BaseClientConfigV1_1_33,

    pub socks5: Socks5V1_1_33,
}

impl From<ConfigV1_1_33> for Config {
    fn from(value: ConfigV1_1_33) -> Self {
        Config {
            base: value.base.into(),
            socks5: value.socks5.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5V1_1_33 {
    /// The address on which the client will be listening for incoming requests
    /// (default: 127.0.0.1:1080)
    // there was a typo in here, so accept the wrong name for the purposes of backwards compatibility
    #[serde(alias = "bind_adddress")]
    pub bind_address: SocketAddr,

    /// The mix address of the provider to which all requests are going to be sent.
    pub provider_mix_address: String,

    /// The version of the 'service provider' this client is going to use in its communication with the
    /// specified socks5 provider.
    // if in doubt, use the legacy version as initially nobody will be using the updated binaries
    #[serde(default)]
    pub provider_interface_version: ProviderInterfaceVersion,

    #[serde(default)]
    pub socks5_protocol_version: Socks5ProtocolVersion,

    /// Specifies whether this client is going to use an anonymous sender tag for communication with the service provider.
    /// While this is going to hide its actual address information, it will make the actual communication
    /// slower and consume nearly double the bandwidth as it will require sending reply SURBs.
    ///
    /// Note that some service providers might not support this.
    #[serde(default)]
    pub send_anonymously: bool,

    #[serde(default)]
    pub socks5_debug: Socks5DebugV1_1_33,
}

impl From<Socks5V1_1_33> for Socks5 {
    fn from(value: Socks5V1_1_33) -> Self {
        Socks5 {
            bind_address: value.bind_address,
            provider_mix_address: value.provider_mix_address,
            provider_interface_version: value.provider_interface_version,
            socks5_protocol_version: value.socks5_protocol_version,
            send_anonymously: value.send_anonymously,
            socks5_debug: value.socks5_debug.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Socks5DebugV1_1_33 {
    /// Number of reply SURBs attached to each `Request::Connect` message.
    pub connection_start_surbs: u32,

    /// Number of reply SURBs attached to each `Request::Send` message.
    pub per_request_surbs: u32,
}

impl From<Socks5DebugV1_1_33> for Socks5Debug {
    fn from(value: Socks5DebugV1_1_33) -> Self {
        Socks5Debug {
            connection_start_surbs: value.connection_start_surbs,
            per_request_surbs: value.per_request_surbs,
        }
    }
}

impl Default for Socks5DebugV1_1_33 {
    fn default() -> Self {
        Socks5DebugV1_1_33 {
            connection_start_surbs: DEFAULT_CONNECTION_START_SURBS,
            per_request_surbs: DEFAULT_PER_REQUEST_SURBS,
        }
    }
}
