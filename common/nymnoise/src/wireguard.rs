// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
}

impl WireGuardStream {
    fn new(inner_stream: Arc<UdpSocket>, handshake: HandshakeState) -> WireGuardStream {
        WireGuardStream {
            inner_stream,
            handshake: Some(handshake),
            noise: None,
        }
    }

    async fn perform_handshake(mut self) -> Result<Self, NoiseError> {
        //Check if we are in the correct state
        let Some(mut handshake) = self.handshake else {
            return Err(NoiseError::IncorrectStateError);
        };
        self.handshake = None;

        while !handshake.is_handshake_finished() {
            if handshake.is_my_turn() {
                self.send_handshake_msg(&mut handshake).await?;
            } else {
                self.recv_handshake_msg(&mut handshake).await?;
            }
        }

        self.noise = Some(handshake.into_transport_mode()?);
        Ok(self)
    }

    async fn send_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
    ) -> Result<(), NoiseError> {
        // let mut buf = vec![0u8; MAXMSGLEN];
        // let len = handshake.write_message(&[], &mut buf)?;

        // self.inner_stream.write_u16(len.try_into()?).await?; //len is always < 2^16, so it shouldn't fail
        // self.inner_stream.write_all(&buf[..len]).await?;
        Ok(())
    }

    async fn recv_handshake_msg(
        &mut self,
        handshake: &mut HandshakeState,
    ) -> Result<(), NoiseError> {
        let msg = self.recv_wg_msg().await?;
        println!("Rcv: {:?}", msg);
        // let mut msg = vec![0u8; msg_len.into()];
        // self.inner_stream.read_exact(&mut msg[..]).await?;

        // let mut buf = vec![0u8; MAXMSGLEN];
        // handshake.read_message(&msg, &mut buf)?;
        Ok(())
    }

    async fn recv_wg_msg(&self) -> Result<Vec<u8>, NoiseError> {
        let mut buf = [0u8; MAXMSGLEN];
        let len = self.inner_stream.recv(&mut buf).await?;
        Ok(buf[..len].to_vec())
    }

    async fn send_wg_msg(&self, msg: &[u8], address: SocketAddr) -> Result<(), NoiseError> {
        self.inner_stream.send_to(msg, address).await?;
        Ok(())
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
) -> Result<WireGuardStream, NoiseError> {
    trace!("Perform Wireguard Handshake, responder side");

    let pattern = NoisePattern::IKpsk2;
    //If the remote_key cannot be kwnown, e.g. in a client-gateway connection
    let secret = [0u8; 32];

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
        .psk(pattern.psk_position(), &secret)
        .build_responder()?;

    let noise_stream = WireGuardStream::new(conn, handshake);

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
