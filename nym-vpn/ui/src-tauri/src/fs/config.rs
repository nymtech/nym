use std::{net::Ipv4Addr, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Path pointing to an env file describing the network.
    pub config_env_file: Option<PathBuf>,
    /// Path to the data directory of a previously initialised mixnet client, where the keys reside.
    pub mixnet_client_path: Option<PathBuf>,
    /// Mixnet public ID of the entry gateway.
    pub entry_gateway: String,
    /// Mixnet recipient address.
    pub exit_router: String,
    /// Enable the wireguard traffic between the client and the entry gateway.
    pub enable_wireguard: Option<bool>,
    /// Associated private key.
    pub private_key: Option<String>,
    /// The IP address of the TUN device.
    // pub ip: Option<Ipv4Addr>,
    /// The MTU of the TUN device.
    pub mtu: Option<i32>,
    /// Disable routing all traffic through the VPN TUN device.
    pub disable_routing: Option<bool>,
    /// Enable two-hop mixnet traffic. This means that traffic jumps directly from entry gateway to
    /// exit gateway.
    pub enable_two_hop: Option<bool>,
    /// Enable Poission process rate limiting of outbound traffic.
    pub enable_poisson_rate: Option<bool>,
}
