use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
};

use nym_network_defaults::WG_PORT;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wireguard {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    pub private_network_prefix: u8,
}

impl Default for Wireguard {
    fn default() -> Self {
        Wireguard {
            enabled: false,
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), WG_PORT),
            announced_port: WG_PORT,
            private_network_prefix: 16,
        }
    }
}

impl Wireguard {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        nym_config::read_config_from_toml_file(path)
    }
}
