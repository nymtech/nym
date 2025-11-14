// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Lewes Protocol State Machine for managing connection lifecycle.

use crate::{
    keypair::{Keypair, PrivateKey as LpPrivateKey, PublicKey as LpPublicKey},
    make_lp_id,
    noise_protocol::NoiseError,
    packet::LpPacket,
    session::LpSession,
    LpError,
};
use bytes::BytesMut;
use nym_crypto::asymmetric::ed25519;
use std::mem;

/// Represents the possible states of the Lewes Protocol connection.
#[derive(Debug, Default)]
pub enum LpState {
    /// Initial state: Ready to start the handshake.
    /// State machine is created with keys, lp_id is derived, session is ready.
    ReadyToHandshake { session: LpSession },

    /// Performing KKT (KEM Key Transfer) exchange before Noise handshake.
    /// Initiator requests responder's KEM public key, responder provides signed key.
    KKTExchange { session: LpSession },

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
    KKTExchange,
    Handshaking,
    Transport,
    Closed,
    Processing,
}

impl From<&LpState> for LpStateBare {
    fn from(state: &LpState) -> Self {
        match state {
            LpState::ReadyToHandshake { .. } => LpStateBare::ReadyToHandshake,
            LpState::KKTExchange { .. } => LpStateBare::KKTExchange,
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
    /// Inform the environment that KKT exchange completed successfully.
    KKTComplete,
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
            | LpState::KKTExchange { session }
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
            | LpState::KKTExchange { session }
            | LpState::Handshaking { session }
            | LpState::Transport { session } => Ok(session),
            LpState::Closed { .. } => Err(LpError::LpSessionClosed),
            LpState::Processing => Err(LpError::LpSessionProcessing),
        }
    }

    pub fn id(&self) -> Result<u32, LpError> {
        Ok(self.session()?.id())
    }

    /// Creates a new state machine from Ed25519 keys, internally deriving X25519 keys.
    ///
    /// This is the primary constructor that accepts only Ed25519 keys (identity/signing keys)
    /// and internally derives the X25519 keys needed for Noise protocol and DHKEM.
    /// This simplifies the API by hiding the X25519 derivation as an implementation detail.
    ///
    /// # Arguments
    ///
    /// * `is_initiator` - Whether this side initiates the handshake
    /// * `local_ed25519_keypair` - Ed25519 keypair for PSQ authentication and X25519 derivation
    ///   (from client identity key or gateway signing key)
    /// * `remote_ed25519_key` - Peer's Ed25519 public key for PSQ authentication and X25519 derivation
    /// * `salt` - Fresh salt for PSK derivation (must be unique per session)
    ///
    /// # Errors
    ///
    /// Returns `LpError::Ed25519RecoveryError` if Ed25519→X25519 conversion fails for the remote key.
    /// Local private key conversion cannot fail.
    pub fn new(
        is_initiator: bool,
        local_ed25519_keypair: (&ed25519::PrivateKey, &ed25519::PublicKey),
        remote_ed25519_key: &ed25519::PublicKey,
        salt: &[u8; 32],
    ) -> Result<Self, LpError> {
        // AIDEV-NOTE: Ed25519→X25519 conversion for API simplification
        // We use standard RFC 7748 conversion to derive X25519 keys from Ed25519 identity keys.
        // This allows callers to provide only Ed25519 keys (which they already have for signing/identity)
        // without needing to manage separate X25519 keypairs.
        //
        // Security: Ed25519→X25519 conversion is cryptographically sound (RFC 7748).
        // The derived X25519 keys are used for:
        // - Noise protocol ephemeral DH
        // - PSQ ECDH baseline security (pre-quantum)
        // - lp_id calculation (session identifier)

        // Convert Ed25519 keys to X25519 for Noise protocol
        let local_x25519_private = local_ed25519_keypair.0.to_x25519();
        let local_x25519_public = local_ed25519_keypair
            .1
            .to_x25519()
            .map_err(LpError::Ed25519RecoveryError)?;

        let remote_x25519_public = remote_ed25519_key
            .to_x25519()
            .map_err(LpError::Ed25519RecoveryError)?;

        // Convert nym_crypto X25519 types to nym_lp keypair types
        let lp_private = LpPrivateKey::from_bytes(local_x25519_private.as_bytes());
        let lp_public = LpPublicKey::from_bytes(local_x25519_public.as_bytes())?;
        let lp_remote_public = LpPublicKey::from_bytes(remote_x25519_public.as_bytes())?;

        // Create X25519 keypair for Noise and lp_id calculation
        let local_x25519_keypair = Keypair::from_keys(lp_private, lp_public);

        // Calculate the shared lp_id using derived X25519 keys
        let lp_id = make_lp_id(local_x25519_keypair.public_key(), &lp_remote_public);

        // Create the session with both Ed25519 (for PSQ auth) and derived X25519 keys (for Noise)
        let session = LpSession::new(
            lp_id,
            is_initiator,
            local_ed25519_keypair,
            local_x25519_keypair.private_key(),
            remote_ed25519_key,
            &lp_remote_public,
            salt,
        )?;

        Ok(LpStateMachine {
            state: LpState::ReadyToHandshake { session },
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
                    // Initiator starts by requesting KEM key via KKT
                    match session.prepare_kkt_request() {
                        Some(Ok(kkt_message)) => {
                            match session.next_packet(kkt_message) {
                                Ok(kkt_packet) => {
                                    result_action = Some(Ok(LpAction::SendPacket(kkt_packet)));
                                    LpState::KKTExchange { session } // Transition to KKTExchange
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
                            // Should not happen for initiator
                            let err = LpError::Internal(
                                "prepare_kkt_request returned None for initiator".to_string(),
                            );
                            let reason = err.to_string();
                            result_action = Some(Err(err));
                            LpState::Closed { reason }
                        }
                    }
                } else {
                    // Responder waits for KKT request
                    LpState::KKTExchange { session }
                    // No action needed yet, result_action remains None.
                }
            }

            // --- KKTExchange State ---
            (LpState::KKTExchange { session }, LpInput::ReceivePacket(packet)) => {
                // Check if packet lp_id matches our session
                if packet.header.session_id() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.session_id())));
                    LpState::KKTExchange { session }
                } else {
                    use crate::message::LpMessage;

                    // Packet message is already parsed, match on it directly
                    match &packet.message {
                        LpMessage::KKTRequest(kkt_request) if !session.is_initiator() => {
                            // Responder processes KKT request
                            // Convert X25519 public key to KEM format for KKT response
                            use nym_kkt::ciphersuite::EncapsulationKey;

                            // Get local X25519 public key by deriving from private key
                            let local_x25519_public = session.local_x25519_public();

                            // Convert to libcrux KEM public key
                            match libcrux_kem::PublicKey::decode(
                                libcrux_kem::Algorithm::X25519,
                                local_x25519_public.as_bytes(),
                            ) {
                                Ok(libcrux_public_key) => {
                                    let responder_kem_pk = EncapsulationKey::X25519(libcrux_public_key);

                                    match session.process_kkt_request(&kkt_request.0, &responder_kem_pk) {
                                        Ok(kkt_response_message) => {
                                            match session.next_packet(kkt_response_message) {
                                                Ok(response_packet) => {
                                                    result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                    // After KKT exchange, move to Handshaking
                                                    LpState::Handshaking { session }
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
                                    let reason = format!("Failed to convert X25519 to KEM: {:?}", e);
                                    let err = LpError::Internal(reason.clone());
                                    result_action = Some(Err(err));
                                    LpState::Closed { reason }
                                }
                            }
                        }
                        LpMessage::KKTResponse(kkt_response) if session.is_initiator() => {
                            // Initiator processes KKT response (signature-only mode with None)
                            match session.process_kkt_response(&kkt_response.0, None) {
                                Ok(()) => {
                                    result_action = Some(Ok(LpAction::KKTComplete));
                                    // After successful KKT, move to Handshaking
                                    LpState::Handshaking { session }
                                }
                                Err(e) => {
                                    let reason = e.to_string();
                                    result_action = Some(Err(e));
                                    LpState::Closed { reason }
                                }
                            }
                        }
                        _ => {
                            // Wrong message type for KKT state
                            let err = LpError::InvalidStateTransition {
                                state: "KKTExchange".to_string(),
                                input: format!("Unexpected message type: {:?}", packet.message),
                            };
                            let reason = err.to_string();
                            result_action = Some(Err(err));
                            LpState::Closed { reason }
                        }
                    }
                }
            }

            // Reject SendData during KKT exchange
            (LpState::KKTExchange { session }, LpInput::SendData(_)) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "KKTExchange".to_string(),
                    input: "SendData".to_string(),
                }));
                LpState::KKTExchange { session }
            }

            // Reject StartHandshake if already in KKT exchange
            (LpState::KKTExchange { session }, LpInput::StartHandshake) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "KKTExchange".to_string(),
                    input: "StartHandshake".to_string(),
                }));
                LpState::KKTExchange { session }
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
                                     // 4. First check if we need to send a handshake message (before checking completion)
                                     match session.prepare_handshake_message() {
                                         Some(Ok(message)) => {
                                             match session.next_packet(message) {
                                                 Ok(response_packet) => {
                                                     result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                     // Check if handshake became complete after preparing message
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
                                             // 5. No message to send - check if handshake is complete
                                             if session.is_handshake_complete() {
                                                 result_action = Some(Ok(LpAction::HandshakeComplete));
                                                 LpState::Transport { session } // Transition to Transport
                                             } else {
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

            // --- Close Transition (applies to ReadyToHandshake, KKTExchange, Handshaking, Transport) ---
            (
                LpState::ReadyToHandshake { .. } // We consume the session here
                | LpState::KKTExchange { .. }
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
    use bytes::Bytes;
    use nym_crypto::asymmetric::ed25519;

    #[test]
    fn test_state_machine_init() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([16u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([17u8; 32], 1);

        // Test salt
        let salt = [51u8; 32];

        let initiator_sm = LpStateMachine::new(
            true,
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
            ed25519_keypair_resp.public_key(),
            &salt,
        );
        assert!(initiator_sm.is_ok());
        let initiator_sm = initiator_sm.unwrap();
        assert!(matches!(
            initiator_sm.state,
            LpState::ReadyToHandshake { .. }
        ));
        let init_session = initiator_sm.session().unwrap();
        assert!(init_session.is_initiator());

        let responder_sm = LpStateMachine::new(
            false,
            (
                ed25519_keypair_resp.private_key(),
                ed25519_keypair_resp.public_key(),
            ),
            ed25519_keypair_init.public_key(),
            &salt,
        );
        assert!(responder_sm.is_ok());
        let responder_sm = responder_sm.unwrap();
        assert!(matches!(
            responder_sm.state,
            LpState::ReadyToHandshake { .. }
        ));
        let resp_session = responder_sm.session().unwrap();
        assert!(!resp_session.is_initiator());

        // Check lp_id is the same (derived internally from Ed25519 keys)
        // Both state machines should have the same lp_id
        assert_eq!(init_session.id(), resp_session.id());
    }

    #[test]
    fn test_state_machine_simplified_flow() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([18u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([19u8; 32], 1);

        // Test salt
        let salt = [52u8; 32];

        // Create state machines (already in ReadyToHandshake)
        let mut initiator = LpStateMachine::new(
            true, // is_initiator
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        let mut responder = LpStateMachine::new(
            false, // is_initiator
            (
                ed25519_keypair_resp.private_key(),
                ed25519_keypair_resp.public_key(),
            ),
            ed25519_keypair_init.public_key(),
            &salt,
        )
        .unwrap();

        let lp_id = initiator.id().unwrap();
        assert_eq!(lp_id, responder.id().unwrap());

        // --- KKT Exchange ---
        println!("--- Step 1: Initiator starts handshake (sends KKT request) ---");
        let init_actions_1 = initiator.process_input(LpInput::StartHandshake);
        let kkt_request_packet = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_1 {
            packet.clone()
        } else {
            panic!("Initiator should send KKT request");
        };

        assert!(
            matches!(initiator.state, LpState::KKTExchange { .. }),
            "Initiator should be in KKTExchange"
        );
        assert_eq!(
            kkt_request_packet.header.session_id(),
            lp_id,
            "KKT request packet has wrong lp_id"
        );

        println!("--- Step 2: Responder starts handshake (waits for KKT) ---");
        let resp_actions_1 = responder.process_input(LpInput::StartHandshake);
        assert!(
            resp_actions_1.is_none(),
            "Responder should produce 0 actions initially"
        );
        assert!(
            matches!(responder.state, LpState::KKTExchange { .. }),
            "Responder should be in KKTExchange"
        );

        println!("--- Step 3: Responder receives KKT request, sends KKT response ---");
        let resp_actions_2 = responder.process_input(LpInput::ReceivePacket(kkt_request_packet));
        let kkt_response_packet = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_2 {
            packet.clone()
        } else {
            panic!("Responder should send KKT response");
        };
        assert!(
            matches!(responder.state, LpState::Handshaking { .. }),
            "Responder should be Handshaking after KKT"
        );

        println!("--- Step 4: Initiator receives KKT response (KKT complete) ---");
        let init_actions_2 = initiator.process_input(LpInput::ReceivePacket(kkt_response_packet));
        assert!(
            matches!(init_actions_2, Some(Ok(LpAction::KKTComplete))),
            "Initiator should signal KKT complete"
        );
        assert!(
            matches!(initiator.state, LpState::Handshaking { .. }),
            "Initiator should be Handshaking after KKT"
        );

        // --- Noise Handshake Message Exchange ---
        println!("--- Step 5: Responder receives Noise msg 1, sends Noise msg 2 ---");
        // Now both sides are in Handshaking, continue with Noise handshake
        // Initiator needs to send first Noise message
        // (In real flow, this might happen automatically or via another process_input call)
        // For this test, we'll simulate the responder receiving the first Noise message
        // Actually, let me check if initiator automatically sends the first Noise message...
        // Looking at the old test, it seems packet 1 was the first Noise message.
        // With KKT, we need the initiator to send the first Noise message now.

        // Initiator prepares and sends first Noise handshake message
        let init_noise_msg = initiator.session().unwrap().prepare_handshake_message();
        let init_packet_1 = if let Some(Ok(msg)) = init_noise_msg {
            initiator.session().unwrap().next_packet(msg).unwrap()
        } else {
            panic!("Initiator should have first Noise message");
        };

        let resp_actions_3 = responder.process_input(LpInput::ReceivePacket(init_packet_1));
        let resp_packet_2 = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_3 {
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

        println!("--- Step 6: Initiator receives Noise msg 2, sends Noise msg 3 ---");
        let init_actions_3 = initiator.process_input(LpInput::ReceivePacket(resp_packet_2));
        let init_packet_3 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_3 {
            packet.clone()
        } else {
            panic!("Initiator should send Noise packet 3");
        };
        assert!(
            matches!(initiator.state, LpState::Transport { .. }),
            "Initiator should be Transport"
        );
        assert_eq!(
            init_packet_3.header.session_id(),
            lp_id,
            "Noise packet 3 has wrong lp_id"
        );

        println!("--- Step 7: Responder receives Noise msg 3, completes handshake ---");
        let resp_actions_4 = responder.process_input(LpInput::ReceivePacket(init_packet_3));
        assert!(
            matches!(resp_actions_4, Some(Ok(LpAction::HandshakeComplete))),
            "Responder should complete handshake"
        );
        assert!(
            matches!(responder.state, LpState::Transport { .. }),
            "Responder should be Transport"
        );

        // --- Transport Phase ---
        println!("--- Step 8: Initiator sends data ---");
        let data_to_send_1 = b"hello responder";
        let init_actions_4 = initiator.process_input(LpInput::SendData(data_to_send_1.to_vec()));
        let data_packet_1 = if let Some(Ok(LpAction::SendPacket(packet))) = init_actions_4 {
            packet.clone()
        } else {
            panic!("Initiator should send data packet");
        };
        assert_eq!(data_packet_1.header.session_id(), lp_id);

        println!("--- Step 9: Responder receives data ---");
        let resp_actions_5 = responder.process_input(LpInput::ReceivePacket(data_packet_1));
        let resp_data_1 = if let Some(Ok(LpAction::DeliverData(data))) = resp_actions_5 {
            data
        } else {
            panic!("Responder should deliver data");
        };
        assert_eq!(resp_data_1, Bytes::copy_from_slice(data_to_send_1));

        println!("--- Step 10: Responder sends data ---");
        let data_to_send_2 = b"hello initiator";
        let resp_actions_6 = responder.process_input(LpInput::SendData(data_to_send_2.to_vec()));
        let data_packet_2 = if let Some(Ok(LpAction::SendPacket(packet))) = resp_actions_6 {
            packet.clone()
        } else {
            panic!("Responder should send data packet");
        };
        assert_eq!(data_packet_2.header.session_id(), lp_id);

        println!("--- Step 11: Initiator receives data ---");
        let init_actions_5 = initiator.process_input(LpInput::ReceivePacket(data_packet_2));
        if let Some(Ok(LpAction::DeliverData(data))) = init_actions_5 {
            assert_eq!(data, Bytes::copy_from_slice(data_to_send_2));
        } else {
            panic!("Initiator should deliver data");
        }

        // --- Close ---
        println!("--- Step 12: Initiator closes ---");
        let init_actions_6 = initiator.process_input(LpInput::Close);
        assert!(matches!(
            init_actions_6,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(initiator.state, LpState::Closed { .. }));

        println!("--- Step 13: Responder closes ---");
        let resp_actions_7 = responder.process_input(LpInput::Close);
        assert!(matches!(
            resp_actions_7,
            Some(Ok(LpAction::ConnectionClosed))
        ));
        assert!(matches!(responder.state, LpState::Closed { .. }));
    }

    #[test]
    fn test_kkt_exchange_initiator_flow() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([20u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([21u8; 32], 1);

        let salt = [53u8; 32];

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            true,
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        // Verify initial state
        assert!(matches!(initiator.state, LpState::ReadyToHandshake { .. }));

        // Step 1: Initiator starts handshake (should send KKT request)
        let init_action = initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(init_action, Some(Ok(LpAction::SendPacket(_)))));
        assert!(matches!(initiator.state, LpState::KKTExchange { .. }));
    }

    #[test]
    fn test_kkt_exchange_responder_flow() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([22u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([23u8; 32], 1);

        let salt = [54u8; 32];

        // Create responder state machine
        let mut responder = LpStateMachine::new(
            false,
            (
                ed25519_keypair_resp.private_key(),
                ed25519_keypair_resp.public_key(),
            ),
            ed25519_keypair_init.public_key(),
            &salt,
        )
        .unwrap();

        // Verify initial state
        assert!(matches!(responder.state, LpState::ReadyToHandshake { .. }));

        // Step 1: Responder starts handshake (should transition to KKTExchange without sending)
        let resp_action = responder.process_input(LpInput::StartHandshake);
        assert!(resp_action.is_none());
        assert!(matches!(responder.state, LpState::KKTExchange { .. }));
    }

    #[test]
    fn test_kkt_exchange_full_roundtrip() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([24u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([25u8; 32], 1);

        let salt = [55u8; 32];

        // Create both state machines
        let mut initiator = LpStateMachine::new(
            true,
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        let mut responder = LpStateMachine::new(
            false,
            (
                ed25519_keypair_resp.private_key(),
                ed25519_keypair_resp.public_key(),
            ),
            ed25519_keypair_init.public_key(),
            &salt,
        )
        .unwrap();

        // Step 1: Initiator starts handshake, sends KKT request
        let init_action = initiator.process_input(LpInput::StartHandshake);
        let kkt_request_packet = if let Some(Ok(LpAction::SendPacket(packet))) = init_action {
            packet.clone()
        } else {
            panic!("Initiator should send KKT request");
        };
        assert!(matches!(initiator.state, LpState::KKTExchange { .. }));

        // Step 2: Responder transitions to KKTExchange
        let resp_action = responder.process_input(LpInput::StartHandshake);
        assert!(resp_action.is_none());
        assert!(matches!(responder.state, LpState::KKTExchange { .. }));

        // Step 3: Responder receives KKT request, sends KKT response
        let resp_action = responder.process_input(LpInput::ReceivePacket(kkt_request_packet));
        let kkt_response_packet = if let Some(Ok(LpAction::SendPacket(packet))) = resp_action {
            packet.clone()
        } else {
            panic!("Responder should send KKT response");
        };
        // After sending KKT response, responder moves to Handshaking
        assert!(matches!(responder.state, LpState::Handshaking { .. }));

        // Step 4: Initiator receives KKT response, completes KKT
        let init_action = initiator.process_input(LpInput::ReceivePacket(kkt_response_packet));
        assert!(matches!(init_action, Some(Ok(LpAction::KKTComplete))));
        // After KKT complete, initiator moves to Handshaking
        assert!(matches!(initiator.state, LpState::Handshaking { .. }));
    }

    #[test]
    fn test_kkt_exchange_close() {
        // Ed25519 keypairs for KKT authentication
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([26u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([27u8; 32], 1);

        let salt = [56u8; 32];

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            true,
            (ed25519_keypair_init.private_key(), ed25519_keypair_init.public_key()),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        // Start handshake to enter KKTExchange state
        initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(initiator.state, LpState::KKTExchange { .. }));

        // Close during KKT exchange
        let close_action = initiator.process_input(LpInput::Close);
        assert!(matches!(close_action, Some(Ok(LpAction::ConnectionClosed))));
        assert!(matches!(initiator.state, LpState::Closed { .. }));
    }

    #[test]
    fn test_kkt_exchange_rejects_invalid_inputs() {
        // Ed25519 keypairs for KKT authentication
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([28u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([29u8; 32], 1);

        let salt = [57u8; 32];

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            true,
            (ed25519_keypair_init.private_key(), ed25519_keypair_init.public_key()),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        // Start handshake to enter KKTExchange state
        initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(initiator.state, LpState::KKTExchange { .. }));

        // Try SendData during KKT exchange (should be rejected)
        let send_action = initiator.process_input(LpInput::SendData(vec![1, 2, 3]));
        assert!(matches!(send_action, Some(Err(LpError::InvalidStateTransition { .. }))));
        assert!(matches!(initiator.state, LpState::KKTExchange { .. })); // Still in KKTExchange

        // Try StartHandshake again during KKT exchange (should be rejected)
        let start_action = initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(start_action, Some(Err(LpError::InvalidStateTransition { .. }))));
        assert!(matches!(initiator.state, LpState::KKTExchange { .. })); // Still in KKTExchange
    }
}
