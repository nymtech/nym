// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Lewes Protocol State Machine for managing connection lifecycle.

use crate::{
    LpError,
    keypair::{Keypair, PrivateKey as LpPrivateKey, PublicKey as LpPublicKey},
    message::{LpMessage, SubsessionKK1Data, SubsessionKK2Data, SubsessionReadyData},
    noise_protocol::NoiseError,
    packet::LpPacket,
    session::{LpSession, SubsessionHandshake},
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

    /// Performing subsession KK handshake while parent remains active.
    /// Parent can still send/receive; subsession messages tunneled through parent.
    SubsessionHandshaking {
        session: LpSession,
        subsession: SubsessionHandshake,
    },

    /// Parent session demoted after subsession promoted.
    /// Can only receive (drain in-flight), cannot send.
    ReadOnlyTransport { session: LpSession },

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
    SubsessionHandshaking,
    ReadOnlyTransport,
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
            LpState::SubsessionHandshaking { .. } => LpStateBare::SubsessionHandshaking,
            LpState::ReadOnlyTransport { .. } => LpStateBare::ReadOnlyTransport,
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
    DeliverData(BytesMut),
    /// Inform the environment that KKT exchange completed successfully.
    KKTComplete,
    /// Inform the environment that the handshake is complete.
    HandshakeComplete,
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
        subsession: SubsessionHandshake,
        new_receiver_index: u32,
    },
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
            | LpState::Transport { session }
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
            LpState::ReadyToHandshake { session }
            | LpState::KKTExchange { session }
            | LpState::Handshaking { session }
            | LpState::Transport { session }
            | LpState::SubsessionHandshaking { session, .. }
            | LpState::ReadOnlyTransport { session } => Ok(session),
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
    /// * `receiver_index` - Client-proposed session identifier (random 4 bytes)
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
        receiver_index: u32,
        is_initiator: bool,
        local_ed25519_keypair: (&ed25519::PrivateKey, &ed25519::PublicKey),
        remote_ed25519_key: &ed25519::PublicKey,
        salt: &[u8; 32],
    ) -> Result<Self, LpError> {
        // We use standard RFC 7748 conversion to derive X25519 keys from Ed25519 identity keys.
        // This allows callers to provide only Ed25519 keys (which they already have for signing/identity)
        // without needing to manage separate X25519 keypairs.
        //
        // Security: Ed25519→X25519 conversion is cryptographically sound (RFC 7748).
        // The derived X25519 keys are used for:
        // - Noise protocol ephemeral DH
        // - PSQ ECDH baseline security (pre-quantum)

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

        // Create X25519 keypair for Noise
        let local_x25519_keypair = Keypair::from_keys(lp_private, lp_public);

        // Create the session with both Ed25519 (for PSQ auth) and derived X25519 keys (for Noise)
        // receiver_index is client-proposed, passed through directly
        let session = LpSession::new(
            receiver_index,
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
                if packet.header.receiver_idx() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
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
                if packet.header.receiver_idx() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
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
                                 result_action = Some(Err(e));
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
            (LpState::Transport { session }, LpInput::ReceivePacket(packet)) => {
                 // Check if packet lp_id matches our session
                 if packet.header.receiver_idx() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
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
                                                     let kk2_msg = LpMessage::SubsessionKK2(SubsessionKK2Data { payload: kk2_payload });
                                                     match session.next_packet(kk2_msg) {
                                                         Ok(response_packet) => {
                                                             result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                             // Stay in SubsessionHandshaking, wait for SubsessionReady
                                                             LpState::SubsessionHandshaking { session, subsession }
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
                             if let Err(e) = session.receiving_counter_quick_check(packet.header.counter) {
                                 result_action = Some(Err(e));
                                 LpState::Transport { session }
                             } else {
                                 // 2. Decrypt data
                                 match session.decrypt_data(&packet.message) {
                                     Ok(plaintext) => {
                                         // 3. Mark counter as received
                                         if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                             result_action = Some(Err(e));
                                             LpState::Transport { session }
                                         } else {
                                             // 4. Deliver data
                                             result_action = Some(Ok(LpAction::DeliverData(BytesMut::from(plaintext.as_slice()))));
                                             LpState::Transport { session }
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

            // --- Transport + InitiateSubsession → SubsessionHandshaking ---
            (LpState::Transport { session }, LpInput::InitiateSubsession) => {
                // Get next subsession index
                let subsession_index = session.next_subsession_index();

                // Create subsession handshake (this side is initiator)
                match session.create_subsession(subsession_index, true) {
                    Ok(subsession) => {
                        // Prepare KK1 message
                        match subsession.prepare_message() {
                            Ok(kk1_payload) => {
                                let kk1_msg = LpMessage::SubsessionKK1(SubsessionKK1Data { payload: kk1_payload });
                                match session.next_packet(kk1_msg) {
                                    Ok(packet) => {
                                        // Emit SubsessionInitiated with packet and index
                                        result_action = Some(Ok(LpAction::SubsessionInitiated {
                                            packet,
                                            subsession_index,
                                        }));
                                        LpState::SubsessionHandshaking { session, subsession }
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
            (LpState::SubsessionHandshaking { session, subsession }, LpInput::ReceivePacket(packet)) => {
                // Check if packet receiver_idx matches our session
                if packet.header.receiver_idx() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
                    LpState::SubsessionHandshaking { session, subsession }
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
                                            let kk2_msg = LpMessage::SubsessionKK2(SubsessionKK2Data { payload: kk2_payload });
                                            match session.next_packet(kk2_msg) {
                                                Ok(response_packet) => {
                                                    result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                    // Stay in SubsessionHandshaking, wait for SubsessionReady
                                                    LpState::SubsessionHandshaking { session, subsession }
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
                                                        let kk2_msg = LpMessage::SubsessionKK2(SubsessionKK2Data { payload: kk2_payload });
                                                        match session.next_packet(kk2_msg) {
                                                            Ok(response_packet) => {
                                                                result_action = Some(Ok(LpAction::SendPacket(response_packet)));
                                                                // Replace old initiator subsession with new responder subsession
                                                                LpState::SubsessionHandshaking { session, subsession: new_subsession }
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
                                        result_action = Some(Ok(LpAction::SendPacket(abort_packet)));
                                        LpState::SubsessionHandshaking { session, subsession }
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
                                    let ready_msg = LpMessage::SubsessionReady(SubsessionReadyData {
                                        receiver_index: new_receiver_index,
                                    });
                                    match session.next_packet(ready_msg) {
                                        Ok(ready_packet) => {
                                            result_action = Some(Ok(LpAction::SubsessionComplete {
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
                                    let err = LpError::Internal("Subsession handshake incomplete after KK2".to_string());
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
                            if let Err(e) = session.receiving_counter_quick_check(packet.header.counter) {
                                result_action = Some(Err(e));
                                LpState::SubsessionHandshaking { session, subsession }
                            } else {
                                match session.decrypt_data(&packet.message) {
                                    Ok(plaintext) => {
                                        if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                            result_action = Some(Err(e));
                                            LpState::SubsessionHandshaking { session, subsession }
                                        } else {
                                            result_action = Some(Ok(LpAction::DeliverData(BytesMut::from(plaintext.as_slice()))));
                                            LpState::SubsessionHandshaking { session, subsession }
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
                                    "Received SubsessionReady but handshake not complete".to_string(),
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
                            LpState::SubsessionHandshaking { session, subsession }
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
            (LpState::SubsessionHandshaking { session, subsession }, LpInput::SendData(data)) => {
                match self.prepare_data_packet(&session, &data) {
                    Ok(packet) => result_action = Some(Ok(LpAction::SendPacket(packet))),
                    Err(e) => {
                        result_action = Some(Err(e.into()));
                    }
                }
                LpState::SubsessionHandshaking { session, subsession }
            }

            // Reject other inputs during subsession handshake
            (LpState::SubsessionHandshaking { session, subsession }, LpInput::StartHandshake) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "SubsessionHandshaking".to_string(),
                    input: "StartHandshake".to_string(),
                }));
                LpState::SubsessionHandshaking { session, subsession }
            }

            (LpState::SubsessionHandshaking { session, subsession }, LpInput::InitiateSubsession) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "SubsessionHandshaking".to_string(),
                    input: "InitiateSubsession".to_string(),
                }));
                LpState::SubsessionHandshaking { session, subsession }
            }

            // --- ReadOnlyTransport State ---
            (LpState::ReadOnlyTransport { session }, LpInput::ReceivePacket(packet)) => {
                // Can still receive and decrypt, but state stays ReadOnlyTransport
                if packet.header.receiver_idx() != session.id() {
                    result_action = Some(Err(LpError::UnknownSessionId(packet.header.receiver_idx())));
                    LpState::ReadOnlyTransport { session }
                } else {
                    if let Err(e) = session.receiving_counter_quick_check(packet.header.counter) {
                        result_action = Some(Err(e));
                        LpState::ReadOnlyTransport { session }
                    } else {
                        match session.decrypt_data(&packet.message) {
                            Ok(plaintext) => {
                                if let Err(e) = session.receiving_counter_mark(packet.header.counter) {
                                    result_action = Some(Err(e));
                                    LpState::ReadOnlyTransport { session }
                                } else {
                                    result_action = Some(Ok(LpAction::DeliverData(BytesMut::from(plaintext.as_slice()))));
                                    LpState::ReadOnlyTransport { session }
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
            }

            // Reject SendData in read-only mode
            (LpState::ReadOnlyTransport { session }, LpInput::SendData(_)) => {
                result_action = Some(Err(LpError::NoiseError(NoiseError::SessionReadOnly)));
                LpState::ReadOnlyTransport { session }
            }

            // Reject other inputs in read-only mode
            (LpState::ReadOnlyTransport { session }, LpInput::StartHandshake) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "ReadOnlyTransport".to_string(),
                    input: "StartHandshake".to_string(),
                }));
                LpState::ReadOnlyTransport { session }
            }

            (LpState::ReadOnlyTransport { session }, LpInput::InitiateSubsession) => {
                result_action = Some(Err(LpError::InvalidStateTransition {
                    state: "ReadOnlyTransport".to_string(),
                    input: "InitiateSubsession".to_string(),
                }));
                LpState::ReadOnlyTransport { session }
            }

            // --- Close Transition (applies to ReadyToHandshake, KKTExchange, Handshaking, Transport, SubsessionHandshaking, ReadOnlyTransport) ---
            (
                LpState::ReadyToHandshake { .. } // We consume the session here
                | LpState::KKTExchange { .. }
                | LpState::Handshaking { .. }
                | LpState::Transport { .. }
                | LpState::SubsessionHandshaking { .. }
                | LpState::ReadOnlyTransport { .. },
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

        let receiver_index: u32 = 77777;

        let initiator_sm = LpStateMachine::new(
            receiver_index,
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
            receiver_index,
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

        // Check both state machines use the same receiver_index
        assert_eq!(init_session.id(), resp_session.id());
    }

    #[test]
    fn test_state_machine_simplified_flow() {
        // Ed25519 keypairs for PSQ authentication and X25519 derivation
        let ed25519_keypair_init = ed25519::KeyPair::from_secret([18u8; 32], 0);
        let ed25519_keypair_resp = ed25519::KeyPair::from_secret([19u8; 32], 1);

        // Test salt
        let salt = [52u8; 32];
        let receiver_index: u32 = 88888;

        // Create state machines (already in ReadyToHandshake)
        let mut initiator = LpStateMachine::new(
            receiver_index,
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
            receiver_index,
            false, // is_initiator
            (
                ed25519_keypair_resp.private_key(),
                ed25519_keypair_resp.public_key(),
            ),
            ed25519_keypair_init.public_key(),
            &salt,
        )
        .unwrap();

        assert_eq!(initiator.id().unwrap(), responder.id().unwrap());

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
            kkt_request_packet.header.receiver_idx(),
            receiver_index,
            "KKT request packet has wrong receiver_index"
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
            resp_packet_2.header.receiver_idx(),
            receiver_index,
            "Packet 2 has wrong receiver_index"
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
            init_packet_3.header.receiver_idx(),
            receiver_index,
            "Noise packet 3 has wrong receiver_index"
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
        assert_eq!(data_packet_1.header.receiver_idx(), receiver_index);

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
        assert_eq!(data_packet_2.header.receiver_idx(), receiver_index);

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
        let receiver_index: u32 = 99901;

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            receiver_index,
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
        let receiver_index: u32 = 99902;

        // Create responder state machine
        let mut responder = LpStateMachine::new(
            receiver_index,
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
        let receiver_index: u32 = 99903;

        // Create both state machines
        let mut initiator = LpStateMachine::new(
            receiver_index,
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
            receiver_index,
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
        let receiver_index: u32 = 99904;

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            receiver_index,
            true,
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
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
        let receiver_index: u32 = 99905;

        // Create initiator state machine
        let mut initiator = LpStateMachine::new(
            receiver_index,
            true,
            (
                ed25519_keypair_init.private_key(),
                ed25519_keypair_init.public_key(),
            ),
            ed25519_keypair_resp.public_key(),
            &salt,
        )
        .unwrap();

        // Start handshake to enter KKTExchange state
        initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(initiator.state, LpState::KKTExchange { .. }));

        // Try SendData during KKT exchange (should be rejected)
        let send_action = initiator.process_input(LpInput::SendData(vec![1, 2, 3]));
        assert!(matches!(
            send_action,
            Some(Err(LpError::InvalidStateTransition { .. }))
        ));
        assert!(matches!(initiator.state, LpState::KKTExchange { .. })); // Still in KKTExchange

        // Try StartHandshake again during KKT exchange (should be rejected)
        let start_action = initiator.process_input(LpInput::StartHandshake);
        assert!(matches!(
            start_action,
            Some(Err(LpError::InvalidStateTransition { .. }))
        ));
        assert!(matches!(initiator.state, LpState::KKTExchange { .. })); // Still in KKTExchange
    }

    /// Helper function to complete a full handshake between initiator and responder,
    /// returning both in Transport state ready for subsession testing.
    fn setup_transport_sessions() -> (LpStateMachine, LpStateMachine) {
        // Use different seeds to get different X25519 keys.
        // The tie-breaker compares X25519 public keys.
        let ed25519_keypair_a = ed25519::KeyPair::from_secret([30u8; 32], 0);
        let ed25519_keypair_b = ed25519::KeyPair::from_secret([31u8; 32], 1);

        let salt = [60u8; 32];
        let receiver_index: u32 = 111111;

        // Create state machines - Alice is initiator, Bob is responder
        let mut alice = LpStateMachine::new(
            receiver_index,
            true,
            (
                ed25519_keypair_a.private_key(),
                ed25519_keypair_a.public_key(),
            ),
            ed25519_keypair_b.public_key(),
            &salt,
        )
        .unwrap();

        let mut bob = LpStateMachine::new(
            receiver_index,
            false,
            (
                ed25519_keypair_b.private_key(),
                ed25519_keypair_b.public_key(),
            ),
            ed25519_keypair_a.public_key(),
            &salt,
        )
        .unwrap();

        // --- Complete KKT Exchange ---
        // Alice starts handshake
        let kkt_request = if let Some(Ok(LpAction::SendPacket(p))) =
            alice.process_input(LpInput::StartHandshake)
        {
            p
        } else {
            panic!("Alice should send KKT request");
        };

        // Bob starts handshake
        let _ = bob.process_input(LpInput::StartHandshake);

        // Bob receives KKT request, sends response
        let kkt_response = if let Some(Ok(LpAction::SendPacket(p))) =
            bob.process_input(LpInput::ReceivePacket(kkt_request))
        {
            p
        } else {
            panic!("Bob should send KKT response");
        };

        // Alice receives KKT response
        let _ = alice.process_input(LpInput::ReceivePacket(kkt_response));

        // --- Complete Noise Handshake ---
        // Alice prepares first Noise message
        let noise1_msg = alice.session().unwrap().prepare_handshake_message().unwrap().unwrap();
        let noise1_packet = alice.session().unwrap().next_packet(noise1_msg).unwrap();

        // Bob receives noise1, sends noise2
        let noise2_packet = if let Some(Ok(LpAction::SendPacket(p))) =
            bob.process_input(LpInput::ReceivePacket(noise1_packet))
        {
            p
        } else {
            panic!("Bob should send Noise packet 2");
        };

        // Alice receives noise2, sends noise3
        let noise3_packet = if let Some(Ok(LpAction::SendPacket(p))) =
            alice.process_input(LpInput::ReceivePacket(noise2_packet))
        {
            p
        } else {
            panic!("Alice should send Noise packet 3");
        };
        assert!(matches!(alice.state, LpState::Transport { .. }));

        // Bob receives noise3, completes handshake
        let _ = bob.process_input(LpInput::ReceivePacket(noise3_packet));
        assert!(matches!(bob.state, LpState::Transport { .. }));

        (alice, bob)
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
        assert!(matches!(
            alice.state,
            LpState::SubsessionHandshaking { .. }
        ));

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
