// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Lewes Protocol State Machine for managing connection lifecycle.
//! State machine ensures protocol steps execute in correct order. Invalid transitions
//! return LpError, preventing protocol violations.

use crate::packet::{EncryptedLpPacket, LpMessage};
use crate::peer_config::LpReceiverIndex;
use crate::session::SessionId;
use crate::{LpError, session::LpSession};
use bytes::{Buf, Bytes};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::mem;

#[derive(Debug)]
pub struct LpTransportState {
    /// The underlying session in the transport state
    session: Box<LpSession>,
}

/// Represents the possible states of the Lewes Protocol connection.
#[derive(Debug, Default)]
pub enum LpState {
    /// Handshake complete, ready for data transport.
    Transport(LpTransportState),

    /// An error occurred, or the connection was intentionally closed.
    Closed { reason: String },

    /// Processing an input event.
    #[default]
    Processing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpStateBare {
    Transport,
    Closed,
    Processing,
}

impl From<&LpState> for LpStateBare {
    fn from(state: &LpState) -> Self {
        match state {
            LpState::Transport { .. } => LpStateBare::Transport,
            LpState::Closed { .. } => LpStateBare::Closed,
            LpState::Processing => LpStateBare::Processing,
        }
    }
}

/// Represents inputs that drive the state machine transitions.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum LpInput {
    /// Received an encrypted LP Packet from the network.
    ReceivePacket(EncryptedLpPacket),

    /// Application wants to send data (only valid in Transport state).
    SendData(LpData),

    /// Close the connection.
    Close,
}

/// Represents actions the state machine requests the environment to perform.
#[derive(Debug)]
pub enum LpAction {
    /// Send an LP Packet over the network.
    SendPacket(EncryptedLpPacket),

    /// Deliver decrypted application data received from the peer.
    DeliverData(LpData),

    /// Inform the environment that the connection is closed.
    ConnectionClosed,
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
            LpState::Transport(transport) => Ok(&mut transport.session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn session(&self) -> Result<&LpSession, LpError> {
        match &self.state {
            LpState::Transport(transport) => Ok(&transport.session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    /// Consume the state machine and return the session with ownership.
    /// This is useful when the handshake is complete and you want to transfer
    /// ownership of the session to the caller.
    pub fn into_session(self) -> Result<LpSession, LpError> {
        match self.state {
            LpState::Transport(transport) => Ok(*transport.session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn session_identifier(&self) -> Result<SessionId, LpError> {
        Ok(*self.session()?.session_identifier())
    }

    pub fn receiver_index(&self) -> Result<LpReceiverIndex, LpError> {
        Ok(self.session()?.receiver_index())
    }

    /// Creates a new state machine in `Transport` state post-KKT/PSQ handshake
    pub fn new(session: LpSession) -> Self {
        LpStateMachine {
            state: LpState::Transport(LpTransportState {
                session: Box::new(session),
            }),
        }
    }

    fn process_input_transport(
        &mut self,
        mut state: LpTransportState,
        input: LpInput,
    ) -> (LpState, Option<Result<LpAction, LpError>>) {
        let session = &mut state.session;
        match input {
            LpInput::ReceivePacket(packet) => {
                // Check if packet lp_id matches our session
                if packet.outer_header().receiver_idx != session.receiver_index() {
                    let result_action = Some(Err(LpError::UnknownSessionId(
                        packet.outer_header().receiver_idx,
                    )));
                    return (LpState::Transport(state), result_action);
                }

                let ctr = packet.outer_header().counter;

                // 1. Check replay protection
                if let Err(e) = session.receiving_counter_quick_check(ctr) {
                    return (LpState::Transport(state), Some(Err(e)));
                }

                // 2. decrypt the packet and attempt to deliver data
                let packet = match session.decrypt_packet(packet) {
                    Ok(packet) => packet,
                    Err(e) => return (LpState::Transport(state), Some(Err(e))),
                };

                // 3. Mark counter as received
                if let Err(e) = session.receiving_counter_mark(ctr) {
                    return (LpState::Transport(state), Some(Err(e)));
                }

                // Check message type
                match packet.into_message() {
                    // Normal encrypted data
                    LpMessage::ApplicationData(payload) => {
                        //  Deliver data
                        match payload.0.try_into() {
                            Ok(data) => {
                                let result_action = Some(Ok(LpAction::DeliverData(data)));
                                (LpState::Transport(state), result_action)
                            }
                            Err(e) => {
                                let reason = e.to_string();
                                (LpState::Closed { reason }, Some(Err(e)))
                            }
                        }
                    }
                    other => {
                        // Unexpected message type in Transport state
                        let err = LpError::InvalidStateTransition {
                            state: "Transport".to_string(),
                            input: format!("Unexpected message type: {other}"),
                        };
                        (LpState::Transport(state), Some(Err(err)))
                    }
                }
            }
            LpInput::SendData(data) => {
                // Encrypt and send application data
                let result_action = match self.prepare_data_packet(session, data) {
                    Ok(packet) => Some(Ok(LpAction::SendPacket(packet))),
                    Err(e) => {
                        // If prepare fails, should we close? Let's report error and stay Transport for now.
                        // Alternative: transition to Closed state.
                        Some(Err(e))
                    }
                };
                // Remain in transport state
                (LpState::Transport(state), result_action)
            }

            // --- Close Transition ---
            LpInput::Close => {
                // Transition to Closed state
                (
                    LpState::Closed {
                        reason: "Closed by user".to_string(),
                    },
                    Some(Ok(LpAction::ConnectionClosed)),
                )
            }
        }
    }

    /// Processes an input event and returns a list of actions to perform.
    pub fn process_input(&mut self, input: LpInput) -> Option<Result<LpAction, LpError>> {
        // 1. Replace current state with a placeholder, taking ownership of the real current state.
        let current_state = mem::take(&mut self.state);

        let mut result_action: Option<Result<LpAction, LpError>> = None;

        // 2. Match on the owned current_state. Each arm calculates and returns the NEXT state.
        let next_state = match (current_state, input) {
            // --- Transport State ---
            (LpState::Transport(transport), input) => {
                let (next_state, action) = self.process_input_transport(transport, input);
                result_action = action;
                next_state
            }
            // Ignore Close if already Closed
            (closed_state @ LpState::Closed { .. }, LpInput::Close) => {
                // result_action remains None
                // Return the original closed state
                closed_state
            }
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
    ) -> Result<EncryptedLpPacket, LpError> {
        session.encrypt_application_data(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionsMock;
    use nym_kkt_ciphersuite::{IntoEnumIterator, KEM};

    #[test]
    fn test_state_machine_init() {
        for kem in KEM::iter() {
            let mock_sessions = SessionsMock::mock_post_handshake(kem);

            let initiator_sm = LpStateMachine::new(mock_sessions.initiator);
            assert!(matches!(initiator_sm.state, LpState::Transport { .. }));
            let init_session = initiator_sm.session().unwrap();

            let responder_sm = LpStateMachine::new(mock_sessions.responder);
            assert!(matches!(responder_sm.state, LpState::Transport { .. }));
            let resp_session = responder_sm.session().unwrap();

            // Check both state machines use the same receiver_index
            assert_eq!(init_session.receiver_index(), resp_session.receiver_index());
        }
    }

    #[test]
    fn test_state_machine_simplified_flow() {
        for kem in KEM::iter() {
            let mock_sessions = SessionsMock::mock_post_handshake(kem);
            let receiver_index = mock_sessions.responder.receiver_index();

            // Create state machines (already in Transport)
            let mut initiator = LpStateMachine::new(mock_sessions.initiator);
            let mut responder = LpStateMachine::new(mock_sessions.responder);

            assert_eq!(
                initiator.session_identifier().unwrap(),
                responder.session_identifier().unwrap()
            );

            // --- Transport Phase ---
            println!("--- Step 1: Initiator sends data ---");
            let data_to_send_1 = LpData::new_opaque(b"hello responder".to_vec());
            let init_actions_4 = initiator.process_input(LpInput::SendData(data_to_send_1.clone()));
            let data_packet_1 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_4 {
                packet.clone()
            } else {
                panic!("Initiator should send data packet");
            };
            assert_eq!(data_packet_1.outer_header().receiver_idx, receiver_index);

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
            assert_eq!(data_packet_2.outer_header().receiver_idx, receiver_index);

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
    }
}
