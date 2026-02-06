// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::packet::LpHeader;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psq::helpers::LpTransportHandshakeExt;
use crate::{LpError, LpMessage, LpPacket};
use nym_kkt::ciphersuite::Ciphersuite;
use nym_lp_transport::traits::LpTransport;

mod helpers;
mod initiator;
mod responder;

// placeholder
pub struct LPSession {
    outer_aead_key: OuterAeadKey,
}

pub struct PSQHandshakeState<'a, S> {
    /// The underlying connection established for the handshake
    connection: &'a mut S,

    /// Ciphersuite selected for the KKT/PSQ exchange
    ciphersuite: Ciphersuite,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: LpRemotePeer,

    /// Counter for outgoing packets
    sending_counter: u64,
}

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    pub fn new(
        connection: &'a mut S,
        ciphersuite: Ciphersuite,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
    ) -> Self {
        PSQHandshakeState {
            connection,
            ciphersuite,
            local_peer,
            remote_peer,
            sending_counter: 0,
        }
    }

    /// Generates the next counter value for outgoing packets.
    pub fn next_counter(&mut self) -> u64 {
        let counter = self.sending_counter;
        self.sending_counter += 1;
        counter
    }

    pub fn next_packet(
        &mut self,
        session_id: u32,
        protocol_version: u8,
        message: LpMessage,
    ) -> LpPacket {
        let counter = self.next_counter();
        let header = LpHeader::new(session_id, counter, protocol_version);
        LpPacket::new(header, message)
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
}
