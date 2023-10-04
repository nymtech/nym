use base64::{engine::general_purpose, Engine as _};
use boringtun::x25519;
use log::info;

// The wireguard UDP listener
pub const WG_ADDRESS: &str = "0.0.0.0";
pub const WG_PORT: u16 = 51822;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

// The private key of the listener
// Corresponding public key: "WM8s8bYegwMa0TJ+xIwhk+dImk2IpDUKslDBCZPizlE="
const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

// The public keys of the registered peers (clients)
const PEERS: &[&str; 1] = &[
    // Corresponding private key: "ILeN6gEh6vJ3Ju8RJ3HVswz+sPgkcKtAYTqzQRhTtlo="
    "NCIhkgiqxFx1ckKl3Zuh595DzIFl8mxju1Vg995EZhI=",
    // Another key
    // "mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs=",
];

pub fn init_static_dev_keys() -> (x25519::StaticSecret, x25519::PublicKey) {
    // TODO: this is a temporary solution for development
    let static_private_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(PRIVATE_KEY)
        .unwrap()
        .try_into()
        .unwrap();
    let static_private = x25519::StaticSecret::try_from(static_private_bytes).unwrap();
    let static_public = x25519::PublicKey::from(&static_private);
    info!(
        "wg public key: {}",
        general_purpose::STANDARD.encode(static_public)
    );

    // TODO: A single static public key is used for all peers during development
    let peer_static_public_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(PEERS[0])
        .unwrap()
        .try_into()
        .unwrap();
    let peer_static_public = x25519::PublicKey::try_from(peer_static_public_bytes).unwrap();

    (static_private, peer_static_public)
}
