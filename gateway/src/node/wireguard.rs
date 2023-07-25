use std::sync::Arc;

use base64::engine::general_purpose;
use base64::Engine as _;
use boringtun::noise::{rate_limiter::RateLimiter, Tunn, TunnResult};
use log::{debug, error, info};
use tokio::{net::UdpSocket, sync::Mutex};
use x25519_dalek::{PublicKey, StaticSecret};

pub async fn wireguard() {
    let wg_address = "127.0.0.1:51820";
    let sock = UdpSocket::bind(wg_address).await.unwrap();
    info!("wg listening on {wg_address}");

    // Secret key ofthe gateway, we'll need a way to generate this from the IdentityKey, might be enough to do some base58 -> base64 conversion
    let secret_bytes: [u8; 32] = general_purpose::STANDARD
        .decode("MBbPChSpmC/FXwIWNROltjd6cOywC81GNEgH9jMOOFk=")
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
        .decode("JpJzoO1DY6HZbn2h33GQJg0GLnxfdpOeV9C/rvdZ5Cs=")
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
    // Rate limiter is global for the gateway
    let rate_limiter = Arc::new(RateLimiter::new(&public, 1024));

    let tun = Arc::new(Mutex::new(
        Tunn::new(secret, peer_public, None, None, 0, Some(rate_limiter)).unwrap(),
    ));
    // Here we have a pretty suboptimal implementation of the UDP communication, for one client
    loop {
        let mut buf = [0; 1024];
        let mut dst = vec![0; 1024];
        let (len, addr) = sock.recv_from(&mut buf).await.unwrap();
        let packet = Tunn::parse_incoming_packet(&buf[..len]).unwrap();
        info!("packet: {:?}", packet);
        let dst_addr = Tunn::dst_address(&buf[..len]);
        debug!("dst_addr: {:?}", dst_addr);
        let result = {
            let mut t = tun.lock().await;
            t.decapsulate(dst_addr, &buf[..len], &mut dst)
        };

        loop {
            let tun = Arc::clone(&tun);
            debug!("result: {:?}", result);
            match result {
                TunnResult::Done => break,
                // We'll get here during the handshake process, if the reponse is WriteToNetwork we should call decapsulate again with an
                // empty datagram until we get a Done response
                TunnResult::WriteToNetwork(p) => {
                    let len = sock.send_to(p, addr).await.unwrap();
                    debug!("{} bytes sent to {}", len, addr);
                    let mut t = tun.lock().await;
                    t.decapsulate(dst_addr, &[], p);
                    break;
                }
                TunnResult::Err(e) => {
                    error!("error: {:?}", e);
                    break;
                }
                // We've recieved some DataPackets we need to forward and send response back to the initiating client
                // if no data packets are available we should send an empty packet as an ack.
                // For now this just logs that it received the packet, and send and ack back to the client.
                TunnResult::WriteToTunnelV4(ref _r, _addy) => {
                    // These are very spammy
                    debug!("WriteToTunnelV4");
                    let mut t = tun.lock().await;
                    sock.send_to(&empty_packet(&mut t), addr).await.unwrap();
                    break;
                }
                TunnResult::WriteToTunnelV6(ref _r, _addy) => {
                    // These are very spammy
                    debug!("WriteToTunnelV6");
                    let mut t = tun.lock().await;
                    sock.send_to(&empty_packet(&mut t), addr).await.unwrap();
                    break;
                }
            }
        }
    }
}

fn empty_packet(tun: &mut Tunn) -> [u8; 128] {
    let mut dst = [0; 128];
    tun.encapsulate(&[], &mut dst);
    dst
}
