// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection

use crate::codec::{decrypt_lp_packet, encrypt_lp_packet};
use crate::packet::{ApplicationData, EncryptedLpPacket, LpHeader, LpMessage, LpPacket};
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::peer_config::LpReceiverIndex;
use crate::psq::{
    InitiatorData, PSQHandshakeState, PSQHandshakeStateInitiator, PSQHandshakeStateResponder,
    ResponderData,
};
use crate::replay::validator::PacketCount;
use crate::transport::LpHandshakeChannel;
use crate::{LpError, replay::ReceivingKeyCounterValidator};
use libcrux_psq::handshake::types::{Authenticator, DHPublicKey};
use libcrux_psq::session::{Session, SessionBinding};
use nym_kkt::keys::EncapsulationKey;
use nym_kkt_ciphersuite::Ciphersuite;
use std::fmt::{Debug, Formatter};

pub type SessionId = [u8; 32];

/// A session in the Lewes Protocol, handling connection state with Noise.
///
/// Sessions manage connection state, including LP replay protection.
/// Each session has a unique receiving index and sending index for connection identification.
pub struct LpSession {
    /// The underlying established session
    psq_session: Session,

    /// The public key material bound to the underlying session. Used for serialisation.
    session_binding: PersistentSessionBinding,

    /// The current active transport channel
    // In the future it might get split between UDP and TCP transports
    active_transport: libcrux_psq::session::Transport,

    /// Look-up index established during the initial KKT exchange
    receiver_index: LpReceiverIndex,

    /// Negotiated protocol version from handshake.
    protocol_version: u8,

    /// Counter for outgoing packets
    sending_counter: u64,

    /// Validator for incoming packet counters to prevent replay attacks
    receiving_counter: ReceivingKeyCounterValidator,
}

/// Wraps public key material that is bound to a session.
#[derive(Clone)]
pub struct PersistentSessionBinding {
    /// The initiator's authenticator value, i.e. a long-term DH public value or signature verification key.
    pub initiator_authenticator: Authenticator,

    /// The responder's long term DH public value.
    pub responder_ecdh_pk: DHPublicKey,

    /// The responder's long term PQ-KEM public key (if any).
    pub responder_pq_pk: Option<EncapsulationKey>,
}

impl Debug for PersistentSessionBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentSessionBinding")
            .field("initiator_authenticator", &"<initiator_authenticator>")
            .field("responder_ecdh_pk", &self.responder_ecdh_pk)
            .field("responder_pq_pk", &self.responder_pq_pk)
            .finish()
    }
}

impl<'a> From<&'a PersistentSessionBinding> for SessionBinding<'a> {
    fn from(value: &'a PersistentSessionBinding) -> Self {
        SessionBinding {
            initiator_authenticator: &value.initiator_authenticator,
            responder_ecdh_pk: &value.responder_ecdh_pk,
            responder_pq_pk: value
                .responder_pq_pk
                .as_ref()
                .map(|k| k.as_pq_encapsulation_key()),
        }
    }
}

impl Debug for LpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LpSession")
            .field("session_id", &self.psq_session.identifier())
            .field("session_binding", &self.session_binding)
            .field("active_transport_id", &self.active_transport.identifier())
            .field("protocol_version", &self.protocol_version)
            .field("sending_counter", &self.sending_counter)
            .field("receiving_counter", &self.receiving_counter)
            .finish()
    }
}

impl LpSession {
    /// Creates a new session after completed KTT/PSQ exchange
    pub fn new(
        mut psq_session: Session,
        session_binding: PersistentSessionBinding,
        receiver_index: LpReceiverIndex,
        protocol_version: u8,
    ) -> Result<Self, LpError> {
        // attempt to derive initial transport
        let transport = psq_session
            .transport_channel()
            .map_err(|inner| LpError::TransportDerivationFailure { inner })?;

        Ok(LpSession {
            psq_session,
            session_binding,
            active_transport: transport,
            receiver_index,
            protocol_version,
            sending_counter: 0,
            receiving_counter: Default::default(),
        })
    }

    /// Create an instance of `Ciphersuite` using hardcoded defaults.
    /// This is a temporary workaround until values can be properly inferred
    /// from reported version
    pub fn default_ciphersuite() -> Ciphersuite {
        Ciphersuite::default()
    }

    /// Helper function to create `PSQHandshakeState` for the handshake initiator
    pub fn psq_handshake_initiator<S>(
        connection: &'_ mut S,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        remote_protocol_version: u8,
    ) -> PSQHandshakeStateInitiator<'_, S>
    where
        S: LpHandshakeChannel + Unpin,
    {
        PSQHandshakeState::new(connection, local_peer)
            .as_initiator(InitiatorData::new(remote_protocol_version, remote_peer))
    }

    /// Helper function to create `PSQHandshakeState` for the handshake responder
    pub fn psq_handshake_responder<S>(
        connection: &'_ mut S,
        local_peer: LpLocalPeer,
    ) -> PSQHandshakeStateResponder<'_, S>
    where
        S: LpHandshakeChannel + Unpin,
    {
        PSQHandshakeState::new(connection, local_peer).as_responder(ResponderData::default())
    }

    pub fn session_binding(&self) -> &PersistentSessionBinding {
        &self.session_binding
    }

    pub fn active_transport(&mut self) -> &mut libcrux_psq::session::Transport {
        &mut self.active_transport
    }

    pub fn session_identifier(&self) -> &[u8; 32] {
        self.psq_session.identifier()
    }

    pub fn receiver_index(&self) -> LpReceiverIndex {
        self.receiver_index
    }

    /// Returns the negotiated protocol version from the handshake.
    ///
    /// Set during `LpSession` creation after sending / receiving `ClientHelloData`
    pub fn negotiated_version(&self) -> u8 {
        self.protocol_version
    }

    pub fn next_packet(&mut self, message: LpMessage) -> Result<LpPacket, LpError> {
        let counter = self.next_counter();
        let header = LpHeader::new(
            self.receiver_index(),
            counter,
            self.protocol_version,
            message.typ(),
        );
        let packet = LpPacket::new(header, message);
        Ok(packet)
    }

    /// Generates the next counter value for outgoing packets.
    pub fn next_counter(&mut self) -> u64 {
        let counter = self.sending_counter;
        self.sending_counter += 1;
        counter
    }

    /// Performs a quick validation check for an incoming packet counter.
    ///
    /// This should be called before performing any expensive operations like
    /// decryption/Noise processing to efficiently filter out potential replay attacks.
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter value to check
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the counter is likely valid
    /// * `Err(LpError::Replay)` if the counter is invalid or a potential replay
    pub fn receiving_counter_quick_check(&self, counter: u64) -> Result<(), LpError> {
        // Branchless implementation uses SIMD when available for constant-time
        // operations, preventing timing attacks. Check before crypto to save CPU cycles.
        self.receiving_counter
            .will_accept_branchless(counter)
            .map_err(LpError::Replay)
    }

    /// Marks a counter as received after successful packet processing.
    ///
    /// This should be called after a packet has been successfully decoded and processed
    /// (including Noise decryption/handshake step) to update the replay protection state.
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter value to mark as received
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the counter was successfully marked
    /// * `Err(LpError::Replay)` if the counter cannot be marked (duplicate, too old, etc.)
    pub fn receiving_counter_mark(&mut self, counter: u64) -> Result<(), LpError> {
        self.receiving_counter
            .mark_did_receive_branchless(counter)
            .map_err(LpError::Replay)
    }

    /// Returns current packet statistics for monitoring.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The next expected counter value for incoming packets
    /// * The total number of received packets
    pub fn current_packet_cnt(&self) -> PacketCount {
        self.receiving_counter.current_packet_cnt()
    }

    /// Encrypts a produced application using the established transport session
    /// and produce an `EncryptedLpPacket`
    ///
    /// # Arguments
    ///
    /// * `data` - plaintext data to encrypt
    ///
    /// # Returns
    ///
    /// * `Ok(EncryptedLpPacket)` containing the encrypted message ciphertext.
    /// * `Err(LpError)` if the session is not in transport mode or encryption fails.
    pub(crate) fn encrypt_application_data(
        &mut self,
        data: Vec<u8>,
    ) -> Result<EncryptedLpPacket, LpError> {
        let packet = self.next_packet(LpMessage::ApplicationData(ApplicationData::new(data)))?;
        encrypt_lp_packet(packet, &mut self.active_transport)
    }

    /// Decrypts an incoming LpPacket
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted packet
    ///
    /// # Returns
    ///
    /// * `Ok(LpPacket)` containing the decrypted application data payload.
    /// * `Err(LpError)` if the session is not in transport mode, decryption fails, or the message is not data.
    pub(crate) fn decrypt_packet(
        &mut self,
        packet: EncryptedLpPacket,
    ) -> Result<LpPacket, LpError> {
        decrypt_lp_packet(packet, &mut self.active_transport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ReplayError, SessionsMock};
    use nym_kkt_ciphersuite::{IntoEnumIterator, KEM};

    #[test]
    fn test_session_creation() {
        for kem in KEM::iter() {
            let mut session = SessionsMock::mock_post_handshake(kem).responder;

            // Initial counter should be zero
            let counter = session.next_counter();
            assert_eq!(counter, 0);

            // Counter should increment
            let counter = session.next_counter();
            assert_eq!(counter, 1);
        }
    }

    #[test]
    fn test_replay_protection_sequential() {
        for kem in KEM::iter() {
            let mut session = SessionsMock::mock_post_handshake(kem).responder;

            // Sequential counters should be accepted
            assert!(session.receiving_counter_quick_check(0).is_ok());
            assert!(session.receiving_counter_mark(0).is_ok());

            assert!(session.receiving_counter_quick_check(1).is_ok());
            assert!(session.receiving_counter_mark(1).is_ok());

            // Duplicates should be rejected
            assert!(session.receiving_counter_quick_check(0).is_err());
            let err = session.receiving_counter_mark(0).unwrap_err();
            match err {
                LpError::Replay(replay_error) => {
                    assert!(matches!(replay_error, ReplayError::DuplicateCounter));
                }
                _ => panic!("Expected replay error"),
            }
        }
    }

    #[test]
    fn test_replay_protection_out_of_order() {
        for kem in KEM::iter() {
            let mut session = SessionsMock::mock_post_handshake(kem).responder;

            // Receive packets in order
            assert!(session.receiving_counter_mark(0).is_ok());
            assert!(session.receiving_counter_mark(1).is_ok());
            assert!(session.receiving_counter_mark(2).is_ok());

            // Skip ahead
            assert!(session.receiving_counter_mark(10).is_ok());

            // Can still receive out-of-order packets within window
            assert!(session.receiving_counter_quick_check(5).is_ok());
            assert!(session.receiving_counter_mark(5).is_ok());

            // But duplicates are still rejected
            assert!(session.receiving_counter_quick_check(5).is_err());
            assert!(session.receiving_counter_mark(5).is_err());
        }
    }

    #[test]
    fn test_packet_stats() {
        for kem in KEM::iter() {
            let mut session = SessionsMock::mock_post_handshake(kem).responder;

            // Initial stats
            let packet_count = session.current_packet_cnt();
            assert_eq!(packet_count.next, 0);
            assert_eq!(packet_count.received, 0);

            // After receiving packets
            assert!(session.receiving_counter_mark(0).is_ok());
            assert!(session.receiving_counter_mark(1).is_ok());

            let packet_count = session.current_packet_cnt();
            assert_eq!(packet_count.next, 2);
            assert_eq!(packet_count.received, 2);
        }
    }
}
