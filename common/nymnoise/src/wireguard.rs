// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use blake2::digest::FixedOutput;
use blake2::digest::KeyInit;
use blake2::Blake2s256;
use blake2::Blake2sMac;
use log::*;
use snow::Builder;
use snow::HandshakeState;
use snow::TransportState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

use crate::*;

/// Wrapper around a UdpSocket
pub struct WireGuardStream {
    inner_stream: Arc<UdpSocket>,
    handshake: Option<HandshakeState>,
    noise: Option<TransportState>,
    peer_public_key: [u8; 32],
}

impl WireGuardStream {
    fn new(
        inner_stream: Arc<UdpSocket>,
        handshake: HandshakeState,
        peer_public_key: [u8; 32],
    ) -> WireGuardStream {
        WireGuardStream {
            inner_stream,
            handshake: Some(handshake),
            noise: None,
            peer_public_key,
        }
    }

    async fn perform_handshake(mut self) -> Result<Self, NoiseError> {
        //Check if we are in the correct state
        let Some(mut handshake) = self.handshake else {
            return Err(NoiseError::IncorrectStateError);
        };
        self.handshake = None;
        let mut id_i = [0u8; 4];
        let mut address: SocketAddr = "0.0.0.0:12345".parse().unwrap();

        while !handshake.is_handshake_finished() {
            if handshake.is_my_turn() {
                self.send_handshake_msg(&mut handshake, id_i.clone(), address)
                    .await?;
            } else {
                let res = self.recv_handshake_msg(&mut handshake).await?;
                id_i = res.0;
                address = res.1;
            }
        }

        self.noise = Some(handshake.into_transport_mode()?);
        Ok(self)
    }

    async fn send_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
        id_initiator: [u8; 4],
        address: SocketAddr,
    ) -> Result<(), NoiseError> {
        let mut buf = vec![0u8; MAXMSGLEN];
        let len = handshake.write_message(&[], &mut buf)?;
        let msg = [&2u32.to_be_bytes(), &id_initiator, &[42u8; 4], &buf[..len]].concat();
        //mac1 key
        let mut k_mac1 = Blake2s256::new();
        k_mac1.update(b"mac1----");
        k_mac1.update(self.peer_public_key);
        let k_mac1_bytes: [u8; 32] = k_mac1.finalize().into();

        //mac1
        let mut hmac = Blake2sMac::new_from_slice(&k_mac1_bytes).unwrap();
        blake2::digest::Update::update(&mut hmac, &msg);
        let mac1_bytes: [u8; 16] = hmac.finalize_fixed().into();

        let final_msg = [msg, mac1_bytes.to_vec(), vec![0u8; 16]].concat();
        self.send_wg_msg(&final_msg, address).await?;

        // self.inner_stream.write_u16(len.try_into()?).await?; //len is always < 2^16, so it shouldn't fail
        // self.inner_stream.write_all(&buf[..len]).await?;
        Ok(())
    }

    async fn recv_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
    ) -> Result<([u8; 4], SocketAddr), NoiseError> {
        println!("Hash 1 : {:?}", handshake.get_handshake_hash());
        let (msg, address) = self.recv_wg_msg().await?;
        println!("Rcv: {:?}", msg);
        println!("Will read : {:?}", &msg[8..88]);

        let mut buf = vec![0u8; MAXMSGLEN];
        handshake.read_message(&msg[8..88], &mut buf)?;
        Ok((msg[4..8].try_into().unwrap(), address))
    }

    async fn recv_wg_msg(&self) -> Result<(Vec<u8>, SocketAddr), NoiseError> {
        let mut buf = [0u8; MAXMSGLEN];
        let (len, address) = self.inner_stream.recv_from(&mut buf).await?;
        Ok((buf[..len].to_vec(), address))
    }

    async fn send_wg_msg(&self, msg: &[u8], address: SocketAddr) -> Result<(), NoiseError> {
        self.inner_stream.send_to(msg, address).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Vec<u8>, NoiseError> {
        let (msg, _) = self.recv_wg_msg().await?;
        println!("Rcv: {:?}", msg);

        let mut buf = vec![0u8; MAXMSGLEN];
        if let Some(noise) = &mut self.noise {
            let len = noise.read_message(&msg[16..], &mut buf)?;
            return Ok(buf[..len].to_vec());
        }
        Err(NoiseError::IncorrectStateError)
    }
}

// pub async fn upgrade_noise_initiator(
//     conn: UdpSocket,
//     pattern: NoisePattern,
//     local_public_key: Option<&[u8]>,
//     local_private_key: &[u8],
//     remote_pub_key: &[u8],
//     epoch: u32,
// ) -> Result<WireGuardStream, NoiseError> {
//     trace!("Perform Noise Handshake, initiator side");

//     //In case the local key cannot be known by the remote party, e.g. in a client-gateway connection
//     let secret = [
//         local_public_key.unwrap_or(&[]),
//         remote_pub_key,
//         &epoch.to_be_bytes(),
//     ]
//     .concat();
//     let secret_hash = Sha256::digest(secret);

//     let handshake = Builder::new(pattern.as_str().parse()?)
//         .local_private_key(local_private_key)
//         .remote_public_key(remote_pub_key)
//         .psk(pattern.psk_position(), &secret_hash)
//         .build_initiator()?;

//     let noise_stream = WireGuardStream::new(conn, handshake);

//     noise_stream.perform_handshake().await
// }

// pub async fn upgrade_noise_initiator_with_topology(
//     conn: UdpSocket,
//     pattern: NoisePattern,
//     topology: &NymTopology,
//     epoch: u32,
//     local_public_key: &[u8],
//     local_private_key: &[u8],
// ) -> Result<WireGuardStream, NoiseError> {
//     //Get init material
//     let responder_addr = match conn.peer_addr() {
//         Ok(addr) => addr,
//         Err(err) => {
//             error!("Unable to extract peer address from connection - {err}");
//             return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
//         }
//     };
//     let remote_pub_key = match topology.find_node_key_by_mix_host(responder_addr) {
//         Some(pub_key) => pub_key.to_bytes(),
//         None => {
//             error!(
//                 "Cannot find public key for node with address {:?}",
//                 responder_addr
//             );
//             return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
//         }
//     };

//     upgrade_noise_initiator(
//         conn,
//         pattern,
//         Some(local_public_key),
//         local_private_key,
//         &remote_pub_key,
//         epoch,
//     )
//     .await
// }

pub async fn upgrade_noise_responder(
    conn: Arc<UdpSocket>,
    local_private_key: &[u8],
    peer_public_key: [u8; 32],
) -> Result<WireGuardStream, NoiseError> {
    trace!("Perform Wireguard Handshake, responder side");

    let pattern = NoisePattern::IKpsk2;
    //If the remote_key cannot be kwnown, e.g. in a client-gateway connection
    let secret = [0u8; 32];

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
        .psk(pattern.psk_position(), &secret)
        .prologue(b"WireGuard v1 zx2c4 Jason@zx2c4.com")
        .build_responder()?;

    let noise_stream = WireGuardStream::new(conn, handshake, peer_public_key);

    noise_stream.perform_handshake().await
}

// pub async fn upgrade_noise_responder_with_topology(
//     conn: UdpSocket,
//     pattern: NoisePattern,
//     topology: &NymTopology,
//     epoch: u32,
//     local_public_key: &[u8],
//     local_private_key: &[u8],
// ) -> Result<WireGuardStream, NoiseError> {
//     //Get init material
//     let initiator_addr = match conn.peer_addr() {
//         Ok(addr) => addr,
//         Err(err) => {
//             error!("Unable to extract peer address from connection - {err}");
//             return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
//         }
//     };
//     let remote_pub_key = match topology.find_node_key_by_mix_host(initiator_addr) {
//         Some(pub_key) => pub_key.to_bytes(),
//         None => {
//             error!(
//                 "Cannot find public key for node with address {:?}",
//                 initiator_addr
//             );
//             return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
//         }
//     };

//     upgrade_noise_responder(
//         conn,
//         pattern,
//         local_public_key,
//         local_private_key,
//         Some(&remote_pub_key),
//         epoch,
//     )
//     .await
// }
