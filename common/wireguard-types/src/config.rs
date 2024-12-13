// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Config {
    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Private IPv4 address of the wireguard gateway.
    /// default: `10.1.0.1`
    pub private_ipv4: Ipv4Addr,

    /// Private IPv6 address of the wireguard gateway.
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
