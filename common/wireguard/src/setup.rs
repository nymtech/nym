use std::net::IpAddr;

use base64::{engine::general_purpose, Engine as _};
use log::info;

// The wireguard UDP listener
pub const WG_ADDRESS: &str = "0.0.0.0";

// The private key of the listener
// Corresponding public key: "WM8s8bYegwMa0TJ+xIwhk+dImk2IpDUKslDBCZPizlE="
pub(crate) const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

// The AllowedIPs for the connected peer, which is one a single IP and the same as the IP that the
// peer has configured on their side.
const ALLOWED_IPS: &str = "10.1.0.2";

fn decode_base64_key(base64_key: &str) -> [u8; 32] {
    general_purpose::STANDARD
        .decode(base64_key)
        .unwrap()
        .try_into()
        .unwrap()
}

pub fn server_static_private_key() -> x25519_dalek::StaticSecret {
    // TODO: this is a temporary solution for development
    let static_private_bytes: [u8; 32] = decode_base64_key(PRIVATE_KEY);
    let static_private = x25519_dalek::StaticSecret::from(static_private_bytes);
    let static_public = x25519_dalek::PublicKey::from(&static_private);
    info!(
        "wg public key: {}",
        general_purpose::STANDARD.encode(static_public)
    );
    static_private
}

pub fn peer_static_public_key() -> x25519_dalek::PublicKey {
    // A single static public key is used during development

    // Read from NYM_PEER_PUBLIC_KEY env variable
    let peer = std::env::var("NYM_PEER_PUBLIC_KEY").expect("NYM_PEER_PUBLIC_KEY must be set");

    let peer_static_public_bytes: [u8; 32] = decode_base64_key(&peer);
    let peer_static_public = x25519_dalek::PublicKey::from(peer_static_public_bytes);
    info!(
        "Adding wg peer public key: {}",
        general_purpose::STANDARD.encode(peer_static_public)
    );
    peer_static_public
}

pub fn peer_allowed_ips() -> ip_network::IpNetwork {
    let key: IpAddr = ALLOWED_IPS.parse().unwrap();
    let cidr = 32u8;
    ip_network::IpNetwork::new_truncate(key, cidr).unwrap()
}
