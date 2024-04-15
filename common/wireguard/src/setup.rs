use std::net::IpAddr;

use base64::{engine::general_purpose, Engine as _};
#[cfg(target_os = "linux")]
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use log::{info, warn};
use serde::Deserialize;

// The wireguard UDP listener
pub const WG_ADDRESS: &str = "0.0.0.0";

// The private key of the listener
// Corresponding public key: "WM8s8bYegwMa0TJ+xIwhk+dImk2IpDUKslDBCZPizlE="
pub(crate) const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

#[derive(Deserialize, Debug)]
pub struct PeerPair {
    pub addr: String,
    pub public_key: String,
}

fn decode_base64_key(base64_key: &str) -> Result<[u8; 32], String> {
    general_purpose::STANDARD
        .decode(base64_key)
        .map_err(|_| String::from("Could not decode"))?
        .try_into()
        .map_err(|_| String::from("Not enough bytes"))
}

pub fn server_static_private_key() -> x25519_dalek::StaticSecret {
    // TODO: this is a temporary solution for development
    let static_private_bytes: [u8; 32] = decode_base64_key(PRIVATE_KEY).unwrap();
    let static_private = x25519_dalek::StaticSecret::from(static_private_bytes);
    let static_public = x25519_dalek::PublicKey::from(&static_private);
    info!(
        "wg public key: {}",
        general_purpose::STANDARD.encode(static_public)
    );
    static_private
}

#[cfg(target_os = "linux")]
pub fn peer_static_pairs(raw_pairs: Vec<PeerPair>) -> Vec<Peer> {
    raw_pairs
        .into_iter()
        .filter_map(|pair| {
            if let Ok(peer_static_public_bytes) = decode_base64_key(&pair.public_key) {
                let peer_static_public = x25519_dalek::PublicKey::from(peer_static_public_bytes);
                let mut peer = Peer::new(Key::new(peer_static_public.to_bytes()));
                if let Ok(key) = pair.addr.parse::<IpAddr>() {
                    let peer_ip = ip_network::IpNetwork::new_truncate(key, 32u8)
                        .expect("Netmask should be correct");
                    let peer_ip_mask =
                        IpAddrMask::new(peer_ip.network_address(), peer_ip.netmask());
                    peer.set_allowed_ips(vec![peer_ip_mask]);
                    Some(peer)
                } else {
                    warn!("Not adding {:?} as IP doesn't parse", pair);
                    None
                }
            } else {
                warn!("Not adding {:?} as public key doesn't decode", pair);
                None
            }
        })
        .collect()
}
