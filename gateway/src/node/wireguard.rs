use base64::engine::general_purpose;
use base64::Engine as _;
use log::{error, info};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::node::wg::events::Event;
use crate::node::wg::WireGuardTunnel;

pub async fn wireguard() {
    let wg_address = "0.0.0.0:51820";
    let sock = Arc::new(UdpSocket::bind(wg_address).await.unwrap());
    info!("wg listening on {wg_address}");

    // Secret key ofthe gateway, we'll need a way to generate this from the IdentityKey, might be enough to do some base58 -> base64 conversion
    let secret_bytes: [u8; 32] = general_purpose::STANDARD
        .decode("AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=")
        .unwrap()
        .try_into()
        .unwrap();

    // Hardcoded peer public key, we'll need a way to register those, private key for that one is `aMUcuAgTiFCHQ/fHqEQRvpLWBxh8sKA7f7lSyWymrGE=`
    // Wireguard configuration that works with this setup is below, this needs to be put into the wireguard client of choice.
    // Working in this case means that they go through the handshake, and client
    // starts sending data packets to the gateway.
    //
    // [Interface]
    // PrivateKey = aMUcuAgTiFCHQ/fHqEQRvpLWBxh8sKA7f7lSyWymrGE=
    // Address = 10.8.0.0/24
    // DNS = 1.1.1.1
    //
    // [Peer]
    // PublicKey = y6/iGYraJjON6pw9fcBa5vLRbGsQqprFLfWKyJQnlWs=
    // AllowedIPs = 0.0.0.0/0
    // Endpoint = 127.0.0.1:51820
    let peer_public_bytes: [u8; 32] = general_purpose::STANDARD
        .decode("mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs=")
        .unwrap()
        .try_into()
        .unwrap();
    let peer_public = PublicKey::from(peer_public_bytes);
    let secret = StaticSecret::try_from(secret_bytes).unwrap();
    let public = PublicKey::from(&secret);
    info!(
        "wg public key: {}",
        general_purpose::STANDARD.encode(public)
    );

    let mut buf = [0; 1024];
    let mut peers = HashSet::new();

    let (bus_tx, _) = broadcast::channel(128);

    while let Ok((len, addr)) = sock.recv_from(&mut buf).await {
        info!("Received {} bytes from {}", len, addr);
        if peers.contains(&addr) {
            bus_tx
                .send(Event::WgPacket(buf[..len].to_vec().into()))
                .map_err(|e| error!("{e}"))
                .unwrap();
        } else {
            info!("New peer with endpoint {addr}");
            let tun =
                WireGuardTunnel::new(peer_public, Arc::clone(&sock), addr, bus_tx.clone()).await;
            peers.insert(addr);
            tokio::spawn(tun.spin_off());
            bus_tx
                .send(Event::WgPacket(buf[..len].to_vec().into()))
                .map_err(|e| error!("{e}"))
                .unwrap();
        }
    }
    panic!("Not OK");
}
