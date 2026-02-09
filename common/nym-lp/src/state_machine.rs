// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Lewes Protocol State Machine for managing connection lifecycle.
//!
//! LP protocol flow (KKT → PSQ → Noise):
//! 1. KKTExchange: Client requests gateway's KEM public key (signed for MITM protection)
//! 2. Handshaking: Noise XKpsk3 with PSQ-derived PSK embedded in handshake messages
//!    - PSQ ciphertext piggybacked on ClientHello (no extra round-trip)
//!    - PSK = Blake3(ECDH || PSQ_secret || salt) provides hybrid classical+PQ security
//! 3. Transport: ChaCha20-Poly1305 authenticated encryption with derived keys
//!
//! State machine ensures protocol steps execute in correct order. Invalid transitions
//! return LpError, preventing protocol violations.

use crate::{
    LpError,
    message::{LpMessage, SubsessionKK1Data, SubsessionKK2Data, SubsessionReadyData},
    noise_protocol::NoiseError,
    packet::LpPacket,
    session::{LpSession, SubsessionHandshake},
};
use bytes::{Buf, Bytes};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::mem;
use tracing::debug;

/// Represents the possible states of the Lewes Protocol connection.
#[derive(Debug, Default)]
pub enum LpState {
    /// Handshake complete, ready for data transport.
    Transport { session: Box<LpSession> },

    /// Performing subsession KK handshake while parent remains active.
    /// Parent can still send/receive; subsession messages tunneled through parent.
    SubsessionHandshaking {
        session: Box<LpSession>,
        subsession: Box<SubsessionHandshake>,
    },

    /// Parent session demoted after subsession promoted.
    /// Can only receive (drain in-flight), cannot send.
    ReadOnlyTransport { session: Box<LpSession> },

    /// An error occurred, or the connection was intentionally closed.
    Closed { reason: String },
    /// Processing an input event.
    #[default]
    Processing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpStateBare {
    Transport,
    SubsessionHandshaking,
    ReadOnlyTransport,
    Closed,
    Processing,
}

impl From<&LpState> for LpStateBare {
    fn from(state: &LpState) -> Self {
        match state {
            LpState::Transport { .. } => LpStateBare::Transport,
            LpState::SubsessionHandshaking { .. } => LpStateBare::SubsessionHandshaking,
            LpState::ReadOnlyTransport { .. } => LpStateBare::ReadOnlyTransport,
            LpState::Closed { .. } => LpStateBare::Closed,
            LpState::Processing => LpStateBare::Processing,
        }
    }
}

/// Represents inputs that drive the state machine transitions.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum LpInput {
    /// Received an LP Packet from the network.
    ReceivePacket(LpPacket),
    /// Application wants to send data (only valid in Transport state).
    SendData(LpData),
    /// Close the connection.
    Close,
    /// Initiate a subsession handshake (only valid in Transport state).
    /// Creates SubsessionHandshake and sends KK1 message.
    InitiateSubsession,
}

/// Represents actions the state machine requests the environment to perform.
#[derive(Debug)]
pub enum LpAction {
    /// Send an LP Packet over the network.
    SendPacket(LpPacket),
    /// Deliver decrypted application data received from the peer.
    DeliverData(LpData),
    /// Inform the environment that the connection is closed.
    ConnectionClosed,
    /// Subsession KK handshake initiated by this side.
    /// Contains the KK1 packet to send and the subsession index for tracking.
    SubsessionInitiated {
        packet: LpPacket,
        subsession_index: u64,
    },
    /// Subsession handshake complete, ready for promotion.
    /// Contains the packet to send (Some for initiator with SubsessionReady, None for responder),
    /// the completed SubsessionHandshake for into_session(), and the new receiver_index.
    SubsessionComplete {
        packet: Option<LpPacket>,
        subsession: Box<SubsessionHandshake>,
        new_receiver_index: u32,
    },
}

/// Represent application data being sent in Transport mode
#[derive(Debug, Clone, PartialEq)]
pub struct LpData {
    pub kind: LpDataKind,
    pub content: Bytes,
}

impl AsRef<[u8]> for LpData {
    fn as_ref(&self) -> &[u8] {
        &self.content
    }
}

impl LpData {
    pub fn new(kind: LpDataKind, content: impl Into<Bytes>) -> Self {
        Self {
            kind,
            content: content.into(),
        }
    }
    pub fn new_opaque(content: impl Into<Bytes>) -> Self {
        Self::new(LpDataKind::Opaque, content)
    }

    pub fn new_registration(data: impl Into<Bytes>) -> Self {
        Self::new(LpDataKind::Registration, data)
    }

    pub fn new_forward(data: impl Into<Bytes>) -> Self {
        Self::new(LpDataKind::Forward, data)
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.into()
    }
}

impl From<LpData> for Vec<u8> {
    fn from(data: LpData) -> Self {
        let mut out = Vec::with_capacity(data.content.len() + 1);
        out.push(data.kind as u8);
        out.extend_from_slice(data.content.as_ref());
        out
    }
}

impl TryFrom<Vec<u8>> for LpData {
    type Error = LpError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let kind = LpDataKind::try_from(value[0]).map_err(|_| {
            LpError::DeserializationError(format!("unknown data type: {}", value[0]))
        })?;
        let mut content = Bytes::from(value);
        content.advance(1);

        Ok(LpData::new(kind, content))
    }
}

/// Represent kind of application data being sent in Transport mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum LpDataKind {
    Opaque = 0,
    Registration = 1,
    Forward = 2,
}

/// The Lewes Protocol State Machine.
pub struct LpStateMachine {
    pub state: LpState,
}

impl LpStateMachine {
    pub fn bare_state(&self) -> LpStateBare {
        LpStateBare::from(&self.state)
    }

    pub fn session_mut(&mut self) -> Result<&mut LpSession, LpError> {
        match &mut self.state {
            LpState::Transport { session }
            | LpState::SubsessionHandshaking { session, .. }
            | LpState::ReadOnlyTransport { session } => Ok(session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn session(&self) -> Result<&LpSession, LpError> {
        match &self.state {
            LpState::Transport { session }
            | LpState::SubsessionHandshaking { session, .. }
            | LpState::ReadOnlyTransport { session } => Ok(session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    /// Consume the state machine and return the session with ownership.
    /// This is useful when the handshake is complete and you want to transfer
    /// ownership of the session to the caller.
    pub fn into_session(self) -> Result<LpSession, LpError> {
        match self.state {
            LpState::Transport { session }
            | LpState::SubsessionHandshaking { session, .. }
            | LpState::ReadOnlyTransport { session } => Ok(*session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn id(&self) -> Result<u32, LpError> {
        Ok(self.session()?.id())
    }

    /// Creates a new state machine in `Transport` state post-KKT/PSQ handshake
    pub fn new(session: LpSession) -> Self {
        LpStateMachine {
            state: LpState::Transport {
                session: Box::new(session),
            },
        }
    }

    /// Creates a state machine in Transport state from a completed subsession handshake.
    ///
    /// This is used when a subsession (rekeying) completes and we need a new state machine
    /// for the promoted session that can handle further subsession initiations (chained rekeying).
    ///
    /// # Arguments
    ///
    /// * `subsession` - The completed subsession handshake
    /// * `receiver_index` - The new session's receiver index
    ///
    /// # Errors
    ///
    /// Returns error if the subsession handshake is not complete.
    pub fn from_subsession(
        subsession: SubsessionHandshake,
        receiver_index: u32,
    ) -> Result<Self, LpError> {
        let session = subsession.into_session(receiver_index)?;
        Ok(LpStateMachine {
            state: LpState::Transport {
                session: Box::new(session),
            },
        })
    }

    /// Processes an input event and returns a list of actions to perform.
    pub fn process_input(&mut self, input: LpInput) -> Option<Result<LpAction, LpError>> {
        // 1. Replace current state with a placeholder, taking ownership of the real current state.
        let current_state = mem::take(&mut self.state);

        let mut result_action: Option<Result<LpAction, LpError>> = None;

        // 2. Match on the owned current_state. Each arm calculates and returns the NEXT state.
        let next_state = match (current_state, input) {
            // --- Transport State ---
            (LpState::Transport { mut session }, LpInput::ReceivePacket(packet)) => {
                // Check if packet lp_id matches our session
                if packet.header.receiver_idx() != session.id() {
                    result_action =
                        Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
                    LpState::Transport { session }
                } else {
                    // Check message type - handle subsession initiation from peer
                    match &packet.message {
                        // Peer initiated subsession - we become responder
                        LpMessage::SubsessionKK1(kk1_data) => {
                            // Create subsession as responder
                            let subsession_index = session.next_subsession_index();
                            match session.create_subsession(subsession_index, false) {
                                Ok(subsession) => {
                                    // Process KK1
                                    match subsession.process_message(&kk1_data.payload) {
                                        Ok(_) => {
                                            // Prepare KK2 response
                                            match subsession.prepare_message() {
                                                Ok(kk2_payload) => {
                                                    let kk2_msg = LpMessage::SubsessionKK2(
                                                        SubsessionKK2Data {
                                                            payload: kk2_payload,
                                                        },
                                                    );
                                                    match session.next_packet(kk2_msg) {
                                                        Ok(response_packet) => {
                                                            result_action =
                                                                Some(Ok(LpAction::SendPacket(
                                                                    response_packet,
                                                                )));
                                                            // Stay in SubsessionHandshaking, wait for SubsessionReady
                                                            LpState::SubsessionHandshaking {
                                                                session,
                                                                subsession: Box::new(subsession),
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let reason = e.to_string();
                                                            result_action = Some(Err(e));
                                                            LpState::Closed { reason }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let reason = e.to_string();
                                                    result_action = Some(Err(e));
                                                    LpState::Closed { reason }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let reason = e.to_string();
                                            result_action = Some(Err(e));
                                            LpState::Closed { reason }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let reason = e.to_string();
                                    result_action = Some(Err(e));
                                    LpState::Closed { reason }
                                }
                            }
                        }
                        // Normal encrypted data
                        LpMessage::EncryptedData(_) => {
                            // 1. Check replay protection
                            if let Err(e) =
                                session.receiving_counter_quick_check(packet.header.counter)
                            {
                                result_action = Some(Err(e));
                                LpState::Transport { session }
                            } else {
                                // 2. Decrypt data
                                match session.decrypt_data(&packet.message) {
                                    Ok(plaintext) => {
                                        // 3. Mark counter as received
                                        if let Err(e) =
                                            session.receiving_counter_mark(packet.header.counter)
                                        {
                                            result_action = Some(Err(e));
                                            LpState::Transport { session }
                                        } else {
                                            // 4. Deliver data
                                            match plaintext.try_into() {
                                                Ok(data) => {
                                                    result_action =
                                                        Some(Ok(LpAction::DeliverData(data)));
                                                    LpState::Transport { session }
                                                }
                                                Err(e) => {
                                                    let reason = e.to_string();
                                                    result_action = Some(Err(e));
                                                    LpState::Closed { reason }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let reason = e.to_string();
                                        result_action = Some(Err(e.into()));
                                        LpState::Closed { reason }
                                    }
                                }
                            }
                        }
                        // Stale abort in Transport state - race already resolved.
                        // This can happen if abort arrives after loser already returned to Transport
                        // via KK1 processing (loser detected local < remote and became responder).
                        // The winner's abort message arrived late. Silently ignore.
                        LpMessage::SubsessionAbort => {
                            debug!("Ignoring stale SubsessionAbort in Transport state");
                            result_action = None;
                            LpState::Transport { session }
                        }
                        _ => {
                            // Unexpected message type in Transport state
                            let err = LpError::InvalidStateTransition {
                                state: "Transport".to_string(),
                                input: format!("Unexpected message type: {}", packet.message),
                            };
                            result_action = Some(Err(err));
                            LpState::Transport { session }
                        }
                    }
                }
            }
            (LpState::Transport { mut session }, LpInput::SendData(data)) => {
                // Encrypt and send application data
                match self.prepare_data_packet(&mut session, data) {
                    Ok(packet) => result_action = Some(Ok(LpAction::SendPacket(packet))),
                    Err(e) => {
                        // If prepare fails, should we close? Let's report error and stay Transport for now.
                        // Alternative: transition to Closed state.
                        result_action = Some(Err(e.into()));
                    }
                }
                // Remain in transport state
                LpState::Transport { session }
            }

            // --- Transport + InitiateSubsession → SubsessionHandshaking ---
            (LpState::Transport { mut session }, LpInput::InitiateSubsession) => {
                // Get next subsession index
                let subsession_index = session.next_subsession_index();

                // Create subsession handshake (this side is initiator)
                match session.create_subsession(subsession_index, true) {
                    Ok(subsession) => {
                        // Prepare KK1 message
                        match subsession.prepare_message() {
                            Ok(kk1_payload) => {
                                let kk1_msg = LpMessage::SubsessionKK1(SubsessionKK1Data {
                                    payload: kk1_payload,
                                });
                                match session.next_packet(kk1_msg) {
                                    Ok(packet) => {
                                        // Emit SubsessionInitiated with packet and index
                                        result_action = Some(Ok(LpAction::SubsessionInitiated {
                                            packet,
                                            subsession_index,
                                        }));
                                        LpState::SubsessionHandshaking {
                                            session,
                                            subsession: Box::new(subsession),
                                        }
                                    }
                                    Err(e) => {
                                        let reason = e.to_string();
                                        result_action = Some(Err(e));
                                        LpState::Closed { reason }
                                    }
                                }
                            }
                            Err(e) => {
                                let reason = e.to_string();
                                result_action = Some(Err(e));
                                LpState::Closed { reason }
                            }
                        }
                    }
                    Err(e) => {
                        let reason = e.to_string();
                        result_action = Some(Err(e));
                        LpState::Closed { reason }
                    }
                }
            }

            // --- SubsessionHandshaking State ---
            (
                LpState::SubsessionHandshaking {
                    mut session,
                    subsession,
                },
                LpInput::ReceivePacket(packet),
            ) => {
                // Check if packet receiver_idx matches our session
                if packet.header.receiver_idx() != session.id() {
                    result_action =
                        Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
                    LpState::SubsessionHandshaking {
                        session,
                        subsession,
                    }
                } else {
                    match &packet.message {
                        LpMessage::SubsessionKK1(kk1_data) if !subsession.is_initiator() => {
                            // Responder processes KK1, prepares KK2
                            // Responder stays in SubsessionHandshaking after sending KK2,
                            // waiting for SubsessionReady from initiator before completing
                            match subsession.process_message(&kk1_data.payload) {
                                Ok(_) => {
                                    match subsession.prepare_message() {
                                        Ok(kk2_payload) => {
                                            let kk2_msg =
                                                LpMessage::SubsessionKK2(SubsessionKK2Data {
                                                    payload: kk2_payload,
                                                });
                                            match session.next_packet(kk2_msg) {
                                                Ok(response_packet) => {
                                                    result_action = Some(Ok(LpAction::SendPacket(
                                                        response_packet,
                                                    )));
                                                    // Stay in SubsessionHandshaking, wait for SubsessionReady
                                                    LpState::SubsessionHandshaking {
                                                        session,
                                                        subsession,
                                                    }
                                                }
                                                Err(e) => {
                                                    let reason = e.to_string();
                                                    result_action = Some(Err(e));
                                                    LpState::Closed { reason }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let reason = e.to_string();
                                            result_action = Some(Err(e));
                                            LpState::Closed { reason }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let reason = e.to_string();
                                    result_action = Some(Err(e));
                                    LpState::Closed { reason }
                                }
                            }
                        }
                        LpMessage::SubsessionKK1(kk1_data) if subsession.is_initiator() => {
                            // Simultaneous initiation race detected.
                            // Both sides called InitiateSubsession and sent KK1 to each other.
                            // Use X25519 public key comparison as deterministic tie-breaker.
                            // Lower key loses and becomes responder.
                            let local_key = session.local_x25519_public();
                            let remote_key = session.remote_x25519_public();

                            if local_key.as_bytes() < remote_key.as_bytes() {
                                // We LOSE - become responder
                                // Use the same index as our initiator subsession, which should
                                // match the winner's index if subsession counters are in sync.
                                // This works because both sides independently picked the same index when
                                // they initiated simultaneously (both counters were at the same value).
                                let subsession_index = subsession.index;
                                match session.create_subsession(subsession_index, false) {
                                    Ok(new_subsession) => {
                                        match new_subsession.process_message(&kk1_data.payload) {
                                            Ok(_) => {
                                                match new_subsession.prepare_message() {
                                                    Ok(kk2_payload) => {
                                                        let kk2_msg = LpMessage::SubsessionKK2(
                                                            SubsessionKK2Data {
                                                                payload: kk2_payload,
                                                            },
                                                        );
                                                        match session.next_packet(kk2_msg) {
                                                            Ok(response_packet) => {
                                                                result_action =
                                                                    Some(Ok(LpAction::SendPacket(
                                                                        response_packet,
                                                                    )));
                                                                // Replace old initiator subsession with new responder subsession
                                                                LpState::SubsessionHandshaking {
                                                                    session,
                                                                    subsession: Box::new(
                                                                        new_subsession,
                                                                    ),
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let reason = e.to_string();
                                                                result_action = Some(Err(e));
                                                                LpState::Closed { reason }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let reason = e.to_string();
                                                        result_action = Some(Err(e));
                                                        LpState::Closed { reason }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let reason = e.to_string();
                                                result_action = Some(Err(e));
                                                LpState::Closed { reason }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let reason = e.to_string();
                                        result_action = Some(Err(e));
                                        LpState::Closed { reason }
                                    }
                                }
                            } else {
                                // We WIN - stay initiator, notify peer they lost
                                // Send SubsessionAbort to explicitly tell peer to become responder
                                let abort_msg = LpMessage::SubsessionAbort;
                                match session.next_packet(abort_msg) {
                                    Ok(abort_packet) => {
                                        result_action =
                                            Some(Ok(LpAction::SendPacket(abort_packet)));
                                        LpState::SubsessionHandshaking {
                                            session,
                                            subsession,
                                        }
                                    }
                                    Err(e) => {
                                        let reason = e.to_string();
                                        result_action = Some(Err(e));
                                        LpState::Closed { reason }
                                    }
                                }
                            }
                        }
                        LpMessage::SubsessionKK2(kk2_data) if subsession.is_initiator() => {
                            // Initiator processes KK2, completes handshake
                            // Initiator emits SubsessionComplete with SubsessionReady packet
                            // and the subsession for caller to promote via into_session()
                            match subsession.process_message(&kk2_data.payload) {
                                Ok(_) if subsession.is_complete() => {
                                    // Generate new receiver_index for subsession
                                    let new_receiver_index: u32 = rand::random();
                                    session.demote(new_receiver_index);

                                    // Send SubsessionReady with new index
                                    let ready_msg =
                                        LpMessage::SubsessionReady(SubsessionReadyData {
                                            receiver_index: new_receiver_index,
                                        });
                                    match session.next_packet(ready_msg) {
                                        Ok(ready_packet) => {
                                            result_action =
                                                Some(Ok(LpAction::SubsessionComplete {
                                                    packet: Some(ready_packet),
                                                    subsession,
                                                    new_receiver_index,
                                                }));
                                            LpState::ReadOnlyTransport { session }
                                        }
                                        Err(e) => {
                                            let reason = e.to_string();
                                            result_action = Some(Err(e));
                                            LpState::Closed { reason }
                                        }
                                    }
                                }
                                Ok(_) => {
                                    // Handshake not complete yet, shouldn't happen for KK
                                    let err = LpError::Internal(
                                        "Subsession handshake incomplete after KK2".to_string(),
                                    );
                                    let reason = err.to_string();
                                    result_action = Some(Err(err));
                                    LpState::Closed { reason }
                                }
                                Err(e) => {
                                    let reason = e.to_string();
                                    result_action = Some(Err(e));
                                    LpState::Closed { reason }
                                }
                            }
                        }
                        LpMessage::EncryptedData(_) => {
                            // Parent still processes normal traffic during subsession handshake
                            // Same as Transport state handling
                            if let Err(e) =
                                session.receiving_counter_quick_check(packet.header.counter)
                            {
                                result_action = Some(Err(e));
                                LpState::SubsessionHandshaking {
                                    session,
                                    subsession,
                                }
                            } else {
                                match session.decrypt_data(&packet.message) {
                                    Ok(plaintext) => {
                                        if let Err(e) =
                                            session.receiving_counter_mark(packet.header.counter)
                                        {
                                            result_action = Some(Err(e));
                                            LpState::SubsessionHandshaking {
                                                session,
                                                subsession,
                                            }
                                        } else {
                                            match plaintext.try_into() {
                                                Ok(data) => {
                                                    result_action =
                                                        Some(Ok(LpAction::DeliverData(data)));
                                                    LpState::SubsessionHandshaking {
                                                        session,
                                                        subsession,
                                                    }
                                                }
                                                Err(err) => {
                                                    result_action = Some(Err(err));
                                                    LpState::SubsessionHandshaking {
                                                        session,
                                                        subsession,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let reason = e.to_string();
                                        result_action = Some(Err(e.into()));
                                        LpState::Closed { reason }
                                    }
                                }
                            }
                        }
                        LpMessage::SubsessionReady(ready_data) if !subsession.is_initiator() => {
                            // Responder receives SubsessionReady from initiator
                            // Responder completes handshake here, uses initiator's receiver_index
                            // The subsession handshake should already be complete (after KK2)
                            if subsession.is_complete() {
                                let new_receiver_index = ready_data.receiver_index;
                                session.demote(new_receiver_index);
                                result_action = Some(Ok(LpAction::SubsessionComplete {
                                    packet: None, // Responder has no packet to send
                                    subsession,
                                    new_receiver_index,
                                }));
                                LpState::ReadOnlyTransport { session }
                            } else {
                                // Shouldn't happen - handshake should be complete after KK2
                                let err = LpError::Internal(
                                    "Received SubsessionReady but handshake not complete"
                                        .to_string(),
                                );
                                let reason = err.to_string();
                                result_action = Some(Err(err));
                                LpState::Closed { reason }
                            }
                        }
                        LpMessage::SubsessionAbort if subsession.is_initiator() => {
                            // We received abort from peer - we lost the simultaneous initiation race.
                            // Peer has higher X25519 key and is staying as initiator.
                            // Discard our initiator subsession and return to Transport to receive peer's KK1.
                            // Peer's KK1 should already be in flight or queued.
                            result_action = None;
                            LpState::Transport { session }
                        }
                        LpMessage::SubsessionAbort if !subsession.is_initiator() => {
                            // Race was already resolved via KK1 - this abort is stale.
                            // We already became responder when we received KK1 and detected local < remote.
                            // The winner's abort message arrived after we processed their KK1.
                            // Silently ignore it - we're in the correct state.
                            result_action = None;
                            LpState::SubsessionHandshaking {
                                session,
                                subsession,
                            }
                        }
                        _ => {
                            // Wrong message type for subsession handshake
                            let err = LpError::InvalidStateTransition {
                                state: "SubsessionHandshaking".to_string(),
                                input: format!("Unexpected message type: {:?}", packet.message),
                            };
                            let reason = err.to_string();
                            result_action = Some(Err(err));
                            LpState::Closed { reason }
                        }
                    }
                }
            }

            // Parent can still send data during subsession handshake
            (
                LpState::SubsessionHandshaking {
                    mut session,
                    subsession,
                },
                LpInput::SendData(data),
            ) => {
                match self.prepare_data_packet(&mut session, data) {
                    Ok(packet) => result_action = Some(Ok(LpAction::SendPacket(packet))),
                    Err(e) => {
                        result_action = Some(Err(e.into()));
                    }
                }
                LpState::SubsessionHandshaking {
                    session,
                    subsession,
                }
            }

            // Reject other inputs during subsession handshake
            (
                LpState::SubsessionHandshaking {
                    session,
                    subsession,
                },
                LpInput::InitiateSubsession,
            ) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "SubsessionHandshaking".to_string(),
                    input: "InitiateSubsession".to_string(),
                }));
                LpState::SubsessionHandshaking {
                    session,
                    subsession,
                }
            }

            // --- ReadOnlyTransport State ---
            (LpState::ReadOnlyTransport { mut session }, LpInput::ReceivePacket(packet)) => {
                // Can still receive and decrypt, but state stays ReadOnlyTransport
                if packet.header.receiver_idx() != session.id() {
                    result_action =
                        Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
                    LpState::ReadOnlyTransport { session }
                } else if let Err(e) = session.receiving_counter_quick_check(packet.header.counter)
                {
                    result_action = Some(Err(e));
                    LpState::ReadOnlyTransport { session }
                } else {
                    match session.decrypt_data(&packet.message) {
                        Ok(plaintext) => {
                            if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                result_action = Some(Err(e));
                                LpState::ReadOnlyTransport { session }
                            } else {
                                match plaintext.try_into() {
                                    Ok(data) => {
                                        result_action = Some(Ok(LpAction::DeliverData(data)));
                                        LpState::ReadOnlyTransport { session }
                                    }
                                    Err(err) => {
                                        result_action = Some(Err(err));
                                        LpState::ReadOnlyTransport { session }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let reason = e.to_string();
                            result_action = Some(Err(e.into()));
                            LpState::Closed { reason }
                        }
                    }
                }
            }

            // Reject SendData in read-only mode
            (LpState::ReadOnlyTransport { session }, LpInput::SendData(_)) => {
                result_action = Some(Err(LpError::NoiseError(NoiseError::SessionReadOnly)));
                LpState::ReadOnlyTransport { session }
            }

            // Reject other inputs in read-only mode
            (LpState::ReadOnlyTransport { session }, LpInput::InitiateSubsession) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "ReadOnlyTransport".to_string(),
                    input: "InitiateSubsession".to_string(),
                }));
                LpState::ReadOnlyTransport { session }
            }

            // --- Close Transition (applies to ReadyToHandshake, KKTExchange, Handshaking, Transport, SubsessionHandshaking, ReadOnlyTransport) ---
            (
                LpState::Transport { .. }
                | LpState::SubsessionHandshaking { .. }
                | LpState::ReadOnlyTransport { .. },
                LpInput::Close,
            ) => {
                result_action = Some(Ok(LpAction::ConnectionClosed));
                // Transition to Closed state
                LpState::Closed {
                    reason: "Closed by user".to_string(),
                }
            }
            // Ignore Close if already Closed
            (closed_state @ LpState::Closed { .. }, LpInput::Close) => {
                // result_action remains None
                // Return the original closed state
                closed_state
            }
            // Ignore StartHandshake if Closed
            // (closed_state @ LpState::Closed { .. }, LpInput::StartHandshake) => {
            //      result_action = Some(Err(LpError::LpSessionClosed));
            //      closed_state
            // }
            // Ignore ReceivePacket if Closed
            (closed_state @ LpState::Closed { .. }, LpInput::ReceivePacket(_)) => {
                result_action = Some(Err(LpError::LpSessionClosed));
                closed_state
            }
            // Ignore SendData if Closed
            (closed_state @ LpState::Closed { .. }, LpInput::SendData(_)) => {
                result_action = Some(Err(LpError::LpSessionClosed));
                closed_state
            }
            // Processing state should not be matched directly if using replace
            (LpState::Processing, _) => {
                // This case should ideally be unreachable if placeholder logic is correct
                let err = LpError::Internal("Reached Processing state unexpectedly".to_string());
                let reason = err.to_string();
                result_action = Some(Err(err));
                LpState::Closed { reason }
            }

            // --- Default: Invalid input for current state (if any combinations missed) ---
            // Consider if this should transition to Closed state. For now, just report error
            // and transition to Closed as a safety measure.
            (invalid_state, input) => {
                let err = LpError::InvalidStateTransition {
                    state: format!("{:?}", invalid_state), // Use owned state for debug info
                    input: format!("{:?}", input),
                };
                let reason = err.to_string();
                result_action = Some(Err(err));
                LpState::Closed { reason }
            }
        };

        // 3. Put the calculated next state back into the machine.
        self.state = next_state;

        result_action // Return the determined action (or None)
    }

    // Helper to prepare an outgoing data packet
    // Kept as it doesn't mutate self.state
    fn prepare_data_packet(
        &self,
        session: &mut LpSession,
        data: LpData,
    ) -> Result<LpPacket, NoiseError> {
        let encrypted_message = session.encrypt_data(Vec::<u8>::from(data).as_ref())?;
        session
            .next_packet(encrypted_message)
            .map_err(|e| NoiseError::Other(e.to_string())) // Improve error conversion?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionsMock;

    #[test]
    fn test_state_machine_init() {
        let mock_sessions = SessionsMock::mock_post_handshake(123).unwrap();

        let initiator_sm = LpStateMachine::new(mock_sessions.initiator);
        assert!(matches!(initiator_sm.state, LpState::Transport { .. }));
        let init_session = initiator_sm.session().unwrap();

        let responder_sm = LpStateMachine::new(mock_sessions.responder);
        assert!(matches!(responder_sm.state, LpState::Transport { .. }));
        let resp_session = responder_sm.session().unwrap();

        // Check both state machines use the same receiver_index
        assert_eq!(init_session.id(), resp_session.id());
    }

    #[test]
    fn test_state_machine_simplified_flow() {
        let receiver_index: u32 = 123;
        let mock_sessions = SessionsMock::mock_post_handshake(123).unwrap();

        // Create state machines (already in Transport)
        let mut initiator = LpStateMachine::new(mock_sessions.initiator);
        let mut responder = LpStateMachine::new(mock_sessions.responder);

        assert_eq!(initiator.id().unwrap(), responder.id().unwrap());

        // --- Transport Phase ---
        println!("--- Step 1: Initiator sends data ---");
        let data_to_send_1 = LpData::new_opaque(b"hello responder".to_vec());
        let init_actions_4 = initiator.process_input(LpInput::SendData(data_to_send_1.clone()));
        let data_packet_1 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_4 {
            packet.clone()
        } else {
            panic!("Initiator should send data packet");
        };
        assert_eq!(data_packet_1.header.receiver_idx(), receiver_index);

        println!("--- Step 2: Responder receives data ---");
        let resp_actions_5 = responder.process_input(LpInput::ReceivePacket(data_packet_1));
        let resp_data_1 = if let Some(Ok(LpAction::DeliverData(data))) = resp_actions_5 {
            data
        } else {
            panic!("Responder should deliver data");
        };
        assert_eq!(resp_data_1, data_to_send_1);

        println!("--- Step 3: Responder sends data ---");
        let data_to_send_2 = LpData::new_opaque(b"hello initiator".to_vec());
        let resp_actions_6 = responder.process_input(LpInput::SendData(data_to_send_2.clone()));
        let data_packet_2 = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_6 {
            packet.clone()
        } else {
            panic!("Responder should send data packet");
        };
        assert_eq!(data_packet_2.header.receiver_idx(), receiver_index);

        println!("--- Step 4: Initiator receives data ---");
        let init_actions_5 = initiator.process_input(LpInput::ReceivePacket(data_packet_2));
        if let Some(Ok(LpAction::DeliverData(data))) = init_actions_5 {
            assert_eq!(data, data_to_send_2);
        } else {
            panic!("Initiator should deliver data");
        }

        // --- Close ---
        println!("--- Step 5: Initiator closes ---");
        let init_actions_6 = initiator.process_input(LpInput::Close);
        assert!(matches!(
            init_actions_6,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(initiator.state, LpState::Closed { .. }));

        println!("--- Step 6: Responder closes ---");
        let resp_actions_7 = responder.process_input(LpInput::Close);
        assert!(matches!(
            resp_actions_7,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(responder.state, LpState::Closed { .. }));
    }

    /// Helper function to complete a full handshake between initiator and responder,
    /// returning both in Transport state ready for subsession testing.
    fn setup_transport_sessions() -> (LpStateMachine, LpStateMachine) {
        let sessions = SessionsMock::mock_post_handshake(12345).unwrap();
        (
            LpStateMachine::new(sessions.initiator),
            LpStateMachine::new(sessions.responder),
        )
    }

    #[test]
    fn test_simultaneous_subsession_initiation() {
        // Test for simultaneous subsession initiation race condition.
        // Both sides call InitiateSubsession at the same time, sending KK1 to each other.
        // The tie-breaker uses X25519 public key comparison: lower key becomes responder.

        let (mut alice, mut bob) = setup_transport_sessions();

        // Get X25519 public keys to determine expected winner
        let alice_x25519 = alice.session().unwrap().local_x25519_public();
        let bob_x25519 = bob.session().unwrap().local_x25519_public();

        // Determine who should win (higher key stays initiator)
        let alice_wins = alice_x25519.as_bytes() > bob_x25519.as_bytes();

        // --- Both sides initiate subsession simultaneously ---
        // Alice initiates subsession
        let alice_kk1_packet = if let Some(Ok(LpAction::SubsessionInitiated { packet, .. })) =
            alice.process_input(LpInput::InitiateSubsession)
        {
            packet
        } else {
            panic!("Alice should initiate subsession with KK1");
        };
        assert!(matches!(alice.state, LpState::SubsessionHandshaking { .. }));

        // Bob initiates subsession (simultaneously)
        let bob_kk1_packet = if let Some(Ok(LpAction::SubsessionInitiated { packet, .. })) =
            bob.process_input(LpInput::InitiateSubsession)
        {
            packet
        } else {
            panic!("Bob should initiate subsession with KK1");
        };
        assert!(matches!(bob.state, LpState::SubsessionHandshaking { .. }));

        // --- Cross-delivery of KK1 packets (race resolution) ---
        // Alice receives Bob's KK1
        let alice_response = alice.process_input(LpInput::ReceivePacket(bob_kk1_packet));

        // Bob receives Alice's KK1
        let bob_response = bob.process_input(LpInput::ReceivePacket(alice_kk1_packet));

        // --- Verify tie-breaker worked correctly ---
        if alice_wins {
            // Alice has higher key - she stays initiator, sends SubsessionAbort
            assert!(
                matches!(alice_response, Some(Ok(LpAction::SendPacket(_)))),
                "Alice (winner) should send SubsessionAbort"
            );
            assert!(
                matches!(alice.state, LpState::SubsessionHandshaking { .. }),
                "Alice should still be SubsessionHandshaking as initiator"
            );

            // Bob has lower key - he becomes responder, sends KK2
            let bob_kk2_packet = if let Some(Ok(LpAction::SendPacket(p))) = bob_response {
                p
            } else {
                panic!("Bob (loser) should send KK2 as new responder");
            };
            assert!(
                matches!(bob.state, LpState::SubsessionHandshaking { .. }),
                "Bob should be SubsessionHandshaking as responder"
            );

            // Complete the handshake: Alice receives KK2
            let alice_completion = alice.process_input(LpInput::ReceivePacket(bob_kk2_packet));
            match alice_completion {
                Some(Ok(LpAction::SubsessionComplete {
                    packet: Some(ready_packet),
                    ..
                })) => {
                    assert!(
                        matches!(alice.state, LpState::ReadOnlyTransport { .. }),
                        "Alice should be ReadOnlyTransport after SubsessionComplete"
                    );

                    // Bob receives SubsessionReady
                    let bob_final = bob.process_input(LpInput::ReceivePacket(ready_packet));
                    assert!(
                        matches!(bob_final, Some(Ok(LpAction::SubsessionComplete { .. }))),
                        "Bob should complete with SubsessionComplete"
                    );
                    assert!(
                        matches!(bob.state, LpState::ReadOnlyTransport { .. }),
                        "Bob should be ReadOnlyTransport"
                    );
                }
                other => panic!("Alice should complete subsession, got: {:?}", other),
            }
        } else {
            // Bob has higher key - he stays initiator, sends SubsessionAbort
            assert!(
                matches!(bob_response, Some(Ok(LpAction::SendPacket(_)))),
                "Bob (winner) should send SubsessionAbort"
            );
            assert!(
                matches!(bob.state, LpState::SubsessionHandshaking { .. }),
                "Bob should still be SubsessionHandshaking as initiator"
            );

            // Alice has lower key - she becomes responder, sends KK2
            let alice_kk2_packet = if let Some(Ok(LpAction::SendPacket(p))) = alice_response {
                p
            } else {
                panic!("Alice (loser) should send KK2 as new responder");
            };
            assert!(
                matches!(alice.state, LpState::SubsessionHandshaking { .. }),
                "Alice should be SubsessionHandshaking as responder"
            );

            // Complete the handshake: Bob receives KK2
            let bob_completion = bob.process_input(LpInput::ReceivePacket(alice_kk2_packet));
            match bob_completion {
                Some(Ok(LpAction::SubsessionComplete {
                    packet: Some(ready_packet),
                    ..
                })) => {
                    assert!(
                        matches!(bob.state, LpState::ReadOnlyTransport { .. }),
                        "Bob should be ReadOnlyTransport after SubsessionComplete"
                    );

                    // Alice receives SubsessionReady
                    let alice_final = alice.process_input(LpInput::ReceivePacket(ready_packet));
                    assert!(
                        matches!(alice_final, Some(Ok(LpAction::SubsessionComplete { .. }))),
                        "Alice should complete with SubsessionComplete"
                    );
                    assert!(
                        matches!(alice.state, LpState::ReadOnlyTransport { .. }),
                        "Alice should be ReadOnlyTransport"
                    );
                }
                other => panic!("Bob should complete subsession, got: {:?}", other),
            }
        }
    }
}
