// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_network_defaults::{
    WG_PORT, WG_TUN_DEVICE_IP_ADDRESS_V4, WG_TUN_DEVICE_IP_ADDRESS_V6, WG_TUN_DEVICE_NETMASK_V4,
    WG_TUN_DEVICE_NETMASK_V6,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub use nym_client_core::config::Config as BaseClientConfig;
pub use persistence::AuthenticatorPaths;

pub mod persistence;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub authenticator: Authenticator,

    pub storage_paths: AuthenticatorPaths,
}

impl Config {
    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Authenticator {
    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Private IP address of the wireguard gateway.
    /// default: `10.1.0.1`
    pub private_ipv4: Ipv4Addr,

    /// Private IP address of the wireguard gateway.
    /// default: `fc01::1`
    pub private_ipv6: Ipv6Addr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
    /// The maximum value for IPv4 is 32
    pub private_network_prefix_v4: u8,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
    /// The maximum value for IPv6 is 128
    pub private_network_prefix_v6: u8,
}

impl Default for Authenticator {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), WG_PORT),
            private_ipv4: WG_TUN_DEVICE_IP_ADDRESS_V4,
            private_ipv6: WG_TUN_DEVICE_IP_ADDRESS_V6,
            announced_port: WG_PORT,
            private_network_prefix_v4: WG_TUN_DEVICE_NETMASK_V4,
            private_network_prefix_v6: WG_TUN_DEVICE_NETMASK_V6,
        }
    }
}

impl From<Authenticator> for nym_wireguard_types::Config {
    fn from(value: Authenticator) -> Self {
        nym_wireguard_types::Config {
            bind_address: value.bind_address,
            private_ipv4: value.private_ipv4,
            private_ipv6: value.private_ipv6,
            announced_port: value.announced_port,
            private_network_prefix_v4: value.private_network_prefix_v4,
            private_network_prefix_v6: value.private_network_prefix_v6,
        }
    }
}
