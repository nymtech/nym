// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// re-export to not break existing imports
pub use deprecated::*;
pub use nyx::*;
pub use wireguard::*;

// all of those should be obtained via nym-node, et al. crate instead
pub mod deprecated {
    pub const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;

    // 'GATEWAY'
    pub const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;

    // 'MIXNODE'
    pub const DEFAULT_VERLOC_LISTENING_PORT: u16 = 1790;
    pub const DEFAULT_HTTP_API_LISTENING_PORT: u16 = 8000;

    // 'CLIENT'
    pub const DEFAULT_WEBSOCKET_LISTENING_PORT: u16 = 1977;

    // 'SOCKS5' CLIENT
    pub const DEFAULT_SOCKS5_LISTENING_PORT: u16 = 1080;

    // NYM-API
    pub const DEFAULT_NYM_API_PORT: u16 = 8080;

    pub const NYM_API_VERSION: &str = "v1";

    // NYM-NODE
    pub const DEFAULT_NYM_NODE_HTTP_PORT: u16 = 8080;
}

pub mod nyx {
    /// Defaults Cosmos Hub/ATOM path
    pub const COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";

    // as set by validators in their configs
    // (note that the 'amount' postfix is relevant here as the full gas price also includes denom)
    pub const GAS_PRICE_AMOUNT: f64 = 0.025;

    // TODO: is there a way to get this from the chain
    pub const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
}

pub mod wireguard {
    use std::net::{IpAddr, Ipv4Addr};

    pub const WG_PORT: u16 = 51822;

    // The interface used to route traffic
    pub const WG_TUN_BASE_NAME: &str = "nymwg";
    pub const WG_TUN_DEVICE_ADDRESS: &str = "10.1.0.1";
    pub const WG_TUN_DEVICE_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 1, 0, 1));
    pub const WG_TUN_DEVICE_NETMASK: &str = "255.255.255.0";
}
