// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod helpers;

use crate::codec::OuterAeadKey;
use crate::message::MessageType;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psq::helpers::LpTransportHandshakeExt;
use crate::{ClientHelloData, LpError, LpMessage, LpPacket};
use nym_lp_transport::traits::LpTransport;
use std::time::{SystemTime, UNIX_EPOCH};

// placeholder
pub struct LPSession;

pub struct PSQHandshakeState<'a, S> {
    /// The underlying connection established for the handshake
    connection: &'a mut S,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: LpRemotePeer,
}

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    pub fn new(connection: &'a mut S, local_peer: LpLocalPeer, remote_peer: LpRemotePeer) -> Self {
        PSQHandshakeState {
            connection,
            local_peer,
            remote_peer,
        }
    }

    /// Attempt to send an Lp packet on the persistent stream
    /// and attempt to immediately read a response.
    ///
    /// Both packets are going to be optionally encrypted/decrypted based on the availability of keys
    /// within the internal `LpStateMachine`
    ///
    /// # Arguments
    /// * `packet` - The LP packet to send
    ///
    /// # Errors
    /// Returns an error if not connected or if send or receive fails.
    async fn send_and_receive_packet(
        &mut self,
        packet: LpPacket,
        outer_aead_key: Option<&OuterAeadKey>,
    ) -> Result<LpPacket, LpError> {
        self.connection.send_packet(packet, outer_aead_key).await?;
        self.connection.receive_packet(outer_aead_key).await
    }

    pub async fn psq_handshake_initiator(
        mut self,
        remote_protocol: u8,
    ) -> Result<LPSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        // 1. Generate and send ClientHelloData with fresh salt and both public keys
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LpError::Internal("System time before UNIX epoch".into()))?
            .as_secs();

        let client_hello_data = self.local_peer.build_client_hello_data(timestamp);
        let salt = client_hello_data.salt;
        let receiver_index = client_hello_data.receiver_index;

        // 2. receive ack
        match self
            .send_and_receive_packet(client_hello_data.into_lp_packet(remote_protocol), None)
            .await?
            .message
        {
            LpMessage::Ack => (),
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Ack,
                ));
            }
        }

        // 3. send KKT request

        // 4. receive KKT response

        // 5. send PSQ msg1

        // 6. receive PSQ msg2

        // 7. send PSQ msg3

        // 8. receive ACK
        todo!()
    }

    pub async fn psq_handshake_responder(self) -> Result<LPSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        // 1. receive ClientHello

        // 2. send ack

        // 3. receive KKT request

        // 4. send KKT response

        // 5. receive PSQ msg1

        // 6. send PSQ msg2

        // 7. receive PSQ msg3

        // 8. send ACK
        todo!()
    }
}
