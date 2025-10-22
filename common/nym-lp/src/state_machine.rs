// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Lewes Protocol State Machine for managing connection lifecycle.

use crate::{
    keypair::{Keypair, PublicKey},
    make_lp_id,
    noise_protocol::NoiseError,
    packet::LpPacket,
    session::LpSession,
    LpError,
};
use bytes::BytesMut;
use std::mem;

/// Represents the possible states of the Lewes Protocol connection.
#[derive(Debug, Default)]
pub enum LpState {
    /// Initial state: Ready to start the handshake.
    /// State machine is created with keys, lp_id is derived, session is ready.
    ReadyToHandshake { session: LpSession },

    /// Actively performing the Noise handshake.
    /// (We might be able to merge this with ReadyToHandshake if the first step always happens)
    Handshaking { session: LpSession }, // Kept for now, logic might merge later

    /// Handshake complete, ready for data transport.
    Transport { session: LpSession },
    /// An error occurred, or the connection was intentionally closed.
    Closed { reason: String },
    /// Processing an input event.
    #[default]
    Processing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpStateBare {
    ReadyToHandshake,
    Handshaking,
    Transport,
    Closed,
    Processing,
}

impl From<&LpState> for LpStateBare {
    fn from(state: &LpState) -> Self {
        match state {
            LpState::ReadyToHandshake { .. } => LpStateBare::ReadyToHandshake,
            LpState::Handshaking { .. } => LpStateBare::Handshaking,
            LpState::Transport { .. } => LpStateBare::Transport,
            LpState::Closed { .. } => LpStateBare::Closed,
            LpState::Processing => LpStateBare::Processing,
        }
    }
}

/// Represents inputs that drive the state machine transitions.
#[derive(Debug)]
pub enum LpInput {
    /// Explicitly trigger the start of the handshake (optional, could be implicit on creation)
    StartHandshake,
    /// Received an LP Packet from the network.
    ReceivePacket(LpPacket),
    /// Application wants to send data (only valid in Transport state).
    SendData(Vec<u8>), // Using Bytes for efficiency
    /// Close the connection.
    Close,
}

/// Represents actions the state machine requests the environment to perform.
#[derive(Debug)]
pub enum LpAction {
    /// Send an LP Packet over the network.
    SendPacket(LpPacket),
    /// Deliver decrypted application data received from the peer.
    DeliverData(BytesMut),
    /// Inform the environment that the handshake is complete.
    HandshakeComplete,
    /// Inform the environment that the connection is closed.
    ConnectionClosed,
}

/// The Lewes Protocol State Machine.
pub struct LpStateMachine {
    pub state: LpState,
}

impl LpStateMachine {
    pub fn bare_state(&self) -> LpStateBare {
        LpStateBare::from(&self.state)
    }

    pub fn session(&self) -> Result<&LpSession, LpError> {
        match &self.state {
            LpState::ReadyToHandshake { session }
            | LpState::Handshaking { session }
            | LpState::Transport { session } => Ok(session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    /// Consume the state machine and return the session with ownership.
    /// This is useful when the handshake is complete and you want to transfer
    /// ownership of the session to the caller.
    pub fn into_session(self) -> Result<LpSession, LpError> {
        match self.state {
            LpState::ReadyToHandshake { session }
            | LpState::Handshaking { session }
            | LpState::Transport { session } => Ok(session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn id(&self) -> Result<u32, LpError> {
        Ok(self.session()?.id())
    }

    /// Creates a new state machine, calculates the lp_id, creates the session,
    /// and sets the initial state to ReadyToHandshake.
    ///
    /// Requires the local *full* keypair to get the public key for lp_id calculation.
    pub fn new(
        is_initiator: bool,
        local_keypair: &Keypair, // Use Keypair
        remote_public_key: &PublicKey,
        psk: &[u8],
        // session_manager: Arc<SessionManager> // Optional
    ) -> Result<Self, LpError> {
        // Calculate the shared lp_id// Calculate the shared lp_id
        let lp_id = make_lp_id(local_keypair.public_key(), remote_public_key);

        let local_private_key = local_keypair.private_key().to_bytes();
        let remote_public_key = remote_public_key.as_bytes();

        // Create the session immediately
        let session = LpSession::new(
            lp_id,
            is_initiator,
            &local_private_key,
            remote_public_key,
            psk,
        )?;

        // TODO: Register the session with the SessionManager if applicable
        // if let Some(manager) = session_manager {
        //     manager.insert_session(lp_id, session.clone())?; // Assuming insert_session exists
        // }

        Ok(LpStateMachine {
            state: LpState::ReadyToHandshake { session },
            // Store necessary info if needed for recreation, otherwise remove
            // is_initiator,
            // local_private_key: local_private_key.to_vec(),
            // remote_public_key: remote_public_key.to_vec(),
            // psk: psk.to_vec(),
        })
    }
    /// Processes an input event and returns a list of actions to perform.
    pub fn process_input(&mut self, input: LpInput) -> Option<Result<LpAction, LpError>> {
        // 1. Replace current state with a placeholder, taking ownership of the real current state.
        let current_state = mem::take(&mut self.state);

        let mut result_action: Option<Result<LpAction, LpError>> = None;

        // 2. Match on the owned current_state. Each arm calculates and returns the NEXT state.
        let next_state = match (current_state, input) {
            // --- ReadyToHandshake State ---
            (LpState::ReadyToHandshake { session }, LpInput::StartHandshake) => {
                if session.is_initiator() {
                    // Initiator sends the first message
                    match self.start_handshake(&session) {
                        Some(Ok(action)) => {
                            result_action = Some(Ok(action));
                            LpState::Handshaking { session } // Transition state
                        }
                        Some(Err(e)) => {
                            // Error occurred, move to Closed state
                            let reason = e.to_string();
                            result_action = Some(Err(e));
                            LpState::Closed { reason }
                        }
                        None => {
                            // Should not happen, treat as internal error
                            let err = LpError::Internal(
                                "start_handshake returned None unexpectedly".to_string(),
                            );
                            let reason = err.to_string();
                            result_action = Some(Err(err));
                            LpState::Closed { reason }
                        }
                    }
                } else {
                    // Responder waits for the first message, transition to Handshaking to wait.
                    LpState::Handshaking { session }
                    // No action needed yet, result_action remains None.
                }
            }

            // --- Handshaking State ---
            (LpState::Handshaking { session }, LpInput::ReceivePacket(packet)) => {
                // Check if packet lp_id matches our session
                if packet.header.session_id() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.session_id())));
                    // Don't change state, return the original state variant
                    LpState::Handshaking { session }
                } else {
                    // --- Inline handle_handshake_packet logic ---
                    // 1. Check replay protection *before* processing
                    if let Err(e) = session.receiving_counter_quick_check(packet.header.counter) {
                         let _reason = e.to_string();
                         result_action = Some(Err(e));
                         LpState::Handshaking { session }
                        //  LpState::Closed { reason }
                    } else {
                         // 2. Process the handshake message
                         match session.process_handshake_message(&packet.message) {
                             Ok(_) => {
                                 // 3. Mark counter as received *after* successful processing
                                 if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                     let _reason = e.to_string();
                                     result_action = Some(Err(e));
                                    //  LpState::Closed { reason }
                                    LpState::Handshaking { session }
                                 } else {
                                     // 4. Check if handshake is now complete
                                     if session.is_handshake_complete() {
                                         result_action = Some(Ok(LpAction::HandshakeComplete));
                                         LpState::Transport { session } // Transition to Transport
                                     } else {
                                         // 5. Check if we need to send the next handshake message
                                         match session.prepare_handshake_message() {
                                             Some(Ok(message)) => {
                                                 match session.next_packet(message) {
                                                     Ok(response_packet) => {
                                                         result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                         // Check AGAIN if handshake became complete *after preparing*
                                                         if session.is_handshake_complete() {
                                                             LpState::Transport { session } // Transition to Transport
                                                         } else {
                                                             LpState::Handshaking { session } // Remain Handshaking
                                                         }
                                                     }
                                                     Err(e) => {
                                                         let reason = e.to_string();
                                                         result_action = Some(Err(e));
                                                         LpState::Closed { reason }
                                                     }
                                                 }
                                             }
                                             Some(Err(e)) => {
                                                 let reason = e.to_string();
                                                 result_action = Some(Err(e));
                                                 LpState::Closed { reason }
                                             }
                                             None => {
                                                 // Handshake stalled unexpectedly
                                                 let err = LpError::NoiseError(NoiseError::Other(
                                                     "Handshake stalled unexpectedly".to_string(),
                                                 ));
                                                 let reason = err.to_string();
                                                 result_action = Some(Err(err));
                                                 LpState::Closed { reason }
                                             }
                                         }
                                     }
                                 }
                             }
                             Err(e) => { // Error from process_handshake_message
                                 let reason = e.to_string();
                                 result_action = Some(Err(e.into()));
                                 LpState::Closed { reason }
                             }
                         }
                    }
                    // --- End inline handle_handshake_packet logic ---
                }
            }
             // Reject SendData during handshake
            (LpState::Handshaking { session }, LpInput::SendData(_)) => { // Keep session if returning to this state
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "Handshaking".to_string(),
                    input: "SendData".to_string(),
                }));
                // Invalid input, remain in Handshaking state
                LpState::Handshaking { session }
            }
            // Reject StartHandshake if already handshaking
            (LpState::Handshaking { session }, LpInput::StartHandshake) => { // Keep session
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "Handshaking".to_string(),
                    input: "StartHandshake".to_string(),
                }));
                 // Invalid input, remain in Handshaking state
                 LpState::Handshaking { session }
            }

            // --- Transport State ---
            (LpState::Transport { session }, LpInput::ReceivePacket(packet)) => { // Needs mut session for marking counter
                 // Check if packet lp_id matches our session
                 if packet.header.session_id() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.session_id())));
                    // Remain in transport state
                    LpState::Transport { session }
                 } else {
                     // --- Inline handle_data_packet logic ---
                     // 1. Check replay protection
                     if let Err(e) = session.receiving_counter_quick_check(packet.header.counter) {
                         let _reason = e.to_string();
                         result_action = Some(Err(e));
                         LpState::Transport { session }
                     } else {
                         // 2. Decrypt data
                         match session.decrypt_data(&packet.message) {
                             Ok(plaintext) => {
                                 // 3. Mark counter as received
                                 if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                     let _reason = e.to_string();
                                     result_action = Some(Err(e));
                                     LpState::Transport{ session }
                                 } else {
                                     // 4. Deliver data
                                     result_action = Some(Ok(LpAction::DeliverData(BytesMut::from(plaintext.as_slice()))));
                                     // Remain in transport state
                                     LpState::Transport { session }
                                 }
                             }
                             Err(e) => { // Error decrypting data
                                 let reason = e.to_string();
                                 result_action = Some(Err(e.into()));
                                 LpState::Closed { reason }
                             }
                         }
                     }
                     // --- End inline handle_data_packet logic ---
                 }
            }
            (LpState::Transport { session }, LpInput::SendData(data)) => {
                // Encrypt and send application data
                match self.prepare_data_packet(&session, &data) {
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
             // Reject StartHandshake if already in transport
            (LpState::Transport { session }, LpInput::StartHandshake) => { // Keep session
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "Transport".to_string(),
                    input: "StartHandshake".to_string(),
                }));
                 // Invalid input, remain in Transport state
                 LpState::Transport { session }
            }

            // --- Close Transition (applies to ReadyToHandshake, Handshaking, Transport) ---
            (
                LpState::ReadyToHandshake { .. } // We consume the session here
                | LpState::Handshaking { .. }
                | LpState::Transport { .. },
                LpInput::Close,
            ) => {
                result_action = Some(Ok(LpAction::ConnectionClosed));
                 // Transition to Closed state
                 LpState::Closed { reason: "Closed by user".to_string() }
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

    // Helper to start the handshake (sends first message if initiator)
    // Kept as it doesn't mutate self.state
    fn start_handshake(&self, session: &LpSession) -> Option<Result<LpAction, LpError>> {
        session
            .prepare_handshake_message()
            .map(|result| match result {
                Ok(message) => match session.next_packet(message) {
                    Ok(packet) => Ok(LpAction::SendPacket(packet)),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            })
    }

    // Helper to prepare an outgoing data packet
    // Kept as it doesn't mutate self.state
    fn prepare_data_packet(
        &self,
        session: &LpSession,
        data: &[u8],
    ) -> Result<LpPacket, NoiseError> {
        let encrypted_message = session.encrypt_data(data)?;
        session
            .next_packet(encrypted_message)
            .map_err(|e| NoiseError::Other(e.to_string())) // Improve error conversion?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;
    use bytes::Bytes;

    #[test]
    fn test_state_machine_init() {
        let init_key = Keypair::new();
        let resp_key = Keypair::new();
        let psk = vec![0u8; 32];
        let remote_pub_key = resp_key.public_key();

        let initiator_sm = LpStateMachine::new(true, &init_key, &remote_pub_key, &psk);
        assert!(initiator_sm.is_ok());
        let initiator_sm = initiator_sm.unwrap();
        assert!(matches!(
            initiator_sm.state,
            LpState::ReadyToHandshake { .. }
        ));
        let init_session = initiator_sm.session().unwrap();
        assert!(init_session.is_initiator());

        let responder_sm = LpStateMachine::new(false, &resp_key, &init_key.public_key(), &psk);
        assert!(responder_sm.is_ok());
        let responder_sm = responder_sm.unwrap();
        assert!(matches!(
            responder_sm.state,
            LpState::ReadyToHandshake { .. }
        ));
        let resp_session = responder_sm.session().unwrap();
        assert!(!resp_session.is_initiator());

        // Check lp_id is the same
        let expected_lp_id = make_lp_id(&init_key.public_key(), remote_pub_key);
        assert_eq!(init_session.id(), expected_lp_id);
        assert_eq!(resp_session.id(), expected_lp_id);
    }

    #[test]
    fn test_state_machine_simplified_flow() {
        // Create test keys
        let init_key = Keypair::new();
        let resp_key = Keypair::new();
        let psk = vec![0u8; 32];

        // Create state machines (already in ReadyToHandshake)
        let mut initiator = LpStateMachine::new(
            true, // is_initiator
            &init_key,
            &resp_key.public_key(),
            &psk.clone(),
        )
        .unwrap();

        let mut responder = LpStateMachine::new(
            false, // is_initiator
            &resp_key,
            &init_key.public_key(),
            &psk,
        )
        .unwrap();

        let lp_id = initiator.id().unwrap();
        assert_eq!(lp_id, responder.id().unwrap());

        // --- Start Handshake --- (No index exchange needed)
        println!("--- Step 1: Initiator starts handshake ---");
        let init_actions_1 = initiator.process_input(LpInput::StartHandshake);
        let init_packet_1 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_1 {
            packet.clone()
        } else {
            panic!("Initiator should produce 1 action");
        };

        assert!(
            matches!(initiator.state, LpState::Handshaking { .. }),
            "Initiator should be Handshaking"
        );
        assert_eq!(
            init_packet_1.header.session_id(),
            lp_id,
            "Packet 1 has wrong lp_id"
        );

        println!("--- Step 2: Responder starts handshake (waits) ---");
        let resp_actions_1 = responder.process_input(LpInput::StartHandshake);
        assert!(
            resp_actions_1.is_none(),
            "Responder should produce 0 actions initially"
        );
        assert!(
            matches!(responder.state, LpState::Handshaking { .. }),
            "Responder should be Handshaking"
        );

        // --- Handshake Message Exchange ---
        println!("--- Step 3: Responder receives packet 1, sends packet 2 ---");
        let resp_actions_2 = responder.process_input(LpInput::ReceivePacket(init_packet_1));
        let resp_packet_2 = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_2 {
            packet.clone()
        } else {
            panic!("Responder should send packet 2");
        };
        assert!(
            matches!(responder.state, LpState::Handshaking { .. }),
            "Responder still Handshaking"
        );
        assert_eq!(
            resp_packet_2.header.session_id(),
            lp_id,
            "Packet 2 has wrong lp_id"
        );

        println!("--- Step 4: Initiator receives packet 2, sends packet 3 ---");
        let init_actions_2 = initiator.process_input(LpInput::ReceivePacket(resp_packet_2));
        let init_packet_3 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_2 {
            packet.clone()
        } else {
            panic!("Initiator should send packet 3");
        };
        assert!(
            matches!(initiator.state, LpState::Transport { .. }),
            "Initiator should be Transport"
        );
        assert_eq!(
            init_packet_3.header.session_id(),
            lp_id,
            "Packet 3 has wrong lp_id"
        );

        println!("--- Step 5: Responder receives packet 3, completes handshake ---");
        let resp_actions_3 = responder.process_input(LpInput::ReceivePacket(init_packet_3));
        assert!(
            matches!(resp_actions_3, Some(Ok(LpAction::HandshakeComplete))),
            "Responder should complete handshake"
        );
        assert!(
            matches!(responder.state, LpState::Transport { .. }),
            "Responder should be Transport"
        );

        // --- Transport Phase ---
        println!("--- Step 6: Initiator sends data ---");
        let data_to_send_1 = b"hello responder";
        let init_actions_3 = initiator.process_input(LpInput::SendData(data_to_send_1.to_vec()));
        let data_packet_1 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_3 {
            packet.clone()
        } else {
            panic!("Initiator should send data packet");
        };
        assert_eq!(data_packet_1.header.session_id(), lp_id);

        println!("--- Step 7: Responder receives data ---");
        let resp_actions_4 = responder.process_input(LpInput::ReceivePacket(data_packet_1));
        let resp_data_1 = if let Some(Ok(LpAction::DeliverData(data))) = resp_actions_4 {
            data
        } else {
            panic!("Responder should deliver data");
        };
        assert_eq!(resp_data_1, Bytes::copy_from_slice(data_to_send_1));

        println!("--- Step 8: Responder sends data ---");
        let data_to_send_2 = b"hello initiator";
        let resp_actions_5 = responder.process_input(LpInput::SendData(data_to_send_2.to_vec()));
        let data_packet_2 = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_5 {
            packet.clone()
        } else {
            panic!("Responder should send data packet");
        };
        assert_eq!(data_packet_2.header.session_id(), lp_id);

        println!("--- Step 9: Initiator receives data ---");
        let init_actions_4 = initiator.process_input(LpInput::ReceivePacket(data_packet_2));
        if let Some(Ok(LpAction::DeliverData(data))) = init_actions_4 {
            assert_eq!(data, Bytes::copy_from_slice(data_to_send_2));
        } else {
            panic!("Initiator should deliver data");
        }

        // --- Close ---
        println!("--- Step 10: Initiator closes ---");
        let init_actions_5 = initiator.process_input(LpInput::Close);
        assert!(matches!(
            init_actions_5,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(initiator.state, LpState::Closed { .. }));

        println!("--- Step 11: Responder closes ---");
        let resp_actions_6 = responder.process_input(LpInput::Close);
        assert!(matches!(
            resp_actions_6,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(responder.state, LpState::Closed { .. }));
    }
}
