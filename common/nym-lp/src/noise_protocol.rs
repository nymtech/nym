// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sans-IO Noise protocol state machine, adapted from noise-psq.

use snow::{TransportState, params::NoiseParams};
use thiserror::Error;

// --- Error Definition ---

/// Errors related to the Noise protocol state machine.
#[derive(Error, Debug)]
pub enum NoiseError {
    #[error("encountered a Noise decryption error")]
    DecryptionError,

    #[error("encountered a Noise Protocol error - {0}")]
    ProtocolError(snow::Error),

    #[error("operation is invalid in the current protocol state")]
    IncorrectStateError,

    #[error("attempted transport mode operation without real PSK injection")]
    PskNotInjected,

    #[error("Other Noise-related error: {0}")]
    Other(String),

    #[error("session is read-only after demotion")]
    SessionReadOnly,
}

impl From<snow::Error> for NoiseError {
    fn from(err: snow::Error) -> Self {
        match err {
            snow::Error::Decrypt => NoiseError::DecryptionError,
            err => NoiseError::ProtocolError(err),
        }
    }
}

// --- Protocol State and Structs ---

/// Represents the possible states of the Noise protocol machine.
#[derive(Debug)]
pub enum NoiseProtocolState {
    /// The protocol is currently performing the handshake.
    /// Contains the Snow handshake state.
    Handshaking(Box<snow::HandshakeState>),

    /// The handshake is complete, and the protocol is in transport mode.
    /// Contains the Snow transport state.
    Transport(TransportState),

    /// The protocol has encountered an unrecoverable error.
    /// Stores the error description.
    Failed(String),
}

/// The core sans-io Noise protocol state machine.
#[derive(Debug)]
pub struct NoiseProtocol {
    state: NoiseProtocolState,
    // We might need buffers for incoming/outgoing data later if we add internal buffering
    // read_buffer: Vec<u8>,
    // write_buffer: Vec<u8>,
}

/// Represents the outcome of processing received bytes via `read_message`.
#[derive(Debug, PartialEq)]
pub enum ReadResult {
    /// A handshake or transport message was successfully processed, but yielded no application data
    /// and did not complete the handshake.
    NoOp,
    /// A complete application data message was decrypted.
    DecryptedData(Vec<u8>),
    /// The handshake successfully completed during this read operation.
    HandshakeComplete,
    // NOTE: NeedMoreBytes variant removed as read_message expects full frames.
}

// --- Implementation ---

impl NoiseProtocol {
    /// Creates a new `NoiseProtocol` instance in the Handshaking state.
    ///
    /// Takes an initialized `snow::HandshakeState` (e.g., from `snow::Builder`).
    pub fn new(initial_state: snow::HandshakeState) -> Self {
        NoiseProtocol {
            state: NoiseProtocolState::Handshaking(Box::new(initial_state)),
        }
    }

    /// Processes a single, complete incoming Noise message frame.
    ///
    /// Assumes the caller handles buffering and framing to provide one full message.
    /// Returns the result of processing the message.
    pub fn read_message(&mut self, input: &[u8]) -> Result<ReadResult, NoiseError> {
        // Allocate a buffer large enough for the maximum possible Noise message size.
        // TODO: Consider reusing a buffer for efficiency.
        let mut buffer = vec![0u8; 65535]; // Max Noise message size

        match &mut self.state {
            NoiseProtocolState::Handshaking(handshake_state) => {
                match handshake_state.read_message(input, &mut buffer) {
                    Ok(_) => {
                        if handshake_state.is_handshake_finished() {
                            // Transition to Transport state.
                            let current_state = std::mem::replace(
                                &mut self.state,
                                // Temporary placeholder needed for mem::replace
                                NoiseProtocolState::Failed(
                                    NoiseError::IncorrectStateError.to_string(),
                                ),
                            );
                            if let NoiseProtocolState::Handshaking(state_to_convert) = current_state
                            {
                                match state_to_convert.into_transport_mode() {
                                    Ok(transport_state) => {
                                        self.state = NoiseProtocolState::Transport(transport_state);
                                        Ok(ReadResult::HandshakeComplete)
                                    }
                                    Err(e) => {
                                        let err = NoiseError::from(e);
                                        self.state = NoiseProtocolState::Failed(err.to_string());
                                        Err(err)
                                    }
                                }
                            } else {
                                // Should be unreachable
                                let err = NoiseError::IncorrectStateError;
                                self.state = NoiseProtocolState::Failed(err.to_string());
                                Err(err)
                            }
                        } else {
                            // Handshake continues
                            Ok(ReadResult::NoOp)
                        }
                    }
                    Err(e) => {
                        let err = NoiseError::from(e);
                        self.state = NoiseProtocolState::Failed(err.to_string());
                        Err(err)
                    }
                }
            }
            NoiseProtocolState::Transport(transport_state) => {
                match transport_state.read_message(input, &mut buffer) {
                    Ok(len) => Ok(ReadResult::DecryptedData(buffer[..len].to_vec())),
                    Err(e) => {
                        let err = NoiseError::from(e);
                        self.state = NoiseProtocolState::Failed(err.to_string());
                        Err(err)
                    }
                }
            }
            NoiseProtocolState::Failed(_) => Err(NoiseError::IncorrectStateError),
        }
    }

    /// Checks if there are pending handshake messages to send.
    ///
    /// If in Handshaking state and it's our turn, generates the message.
    /// Transitions state to Transport if the handshake completes after this message.
    /// Returns `None` if not in Handshaking state or not our turn.
    pub fn get_bytes_to_send(&mut self) -> Option<Result<Vec<u8>, NoiseError>> {
        match &mut self.state {
            NoiseProtocolState::Handshaking(handshake_state) => {
                if handshake_state.is_my_turn() {
                    let mut buffer = vec![0u8; 65535];
                    match handshake_state.write_message(&[], &mut buffer) {
                        // Empty payload for handshake msg
                        Ok(len) => {
                            if handshake_state.is_handshake_finished() {
                                // Transition to Transport state.
                                let current_state = std::mem::replace(
                                    &mut self.state,
                                    NoiseProtocolState::Failed(
                                        NoiseError::IncorrectStateError.to_string(),
                                    ),
                                );
                                if let NoiseProtocolState::Handshaking(state_to_convert) =
                                    current_state
                                {
                                    match state_to_convert.into_transport_mode() {
                                        Ok(transport_state) => {
                                            self.state =
                                                NoiseProtocolState::Transport(transport_state);
                                            Some(Ok(buffer[..len].to_vec())) // Return final handshake msg
                                        }
                                        Err(e) => {
                                            let err = NoiseError::from(e);
                                            self.state =
                                                NoiseProtocolState::Failed(err.to_string());
                                            Some(Err(err))
                                        }
                                    }
                                } else {
                                    // Should be unreachable
                                    let err = NoiseError::IncorrectStateError;
                                    self.state = NoiseProtocolState::Failed(err.to_string());
                                    Some(Err(err))
                                }
                            } else {
                                // Handshake continues
                                Some(Ok(buffer[..len].to_vec()))
                            }
                        }
                        Err(e) => {
                            let err = NoiseError::from(e);
                            self.state = NoiseProtocolState::Failed(err.to_string());
                            Some(Err(err))
                        }
                    }
                } else {
                    // Not our turn
                    None
                }
            }
            NoiseProtocolState::Transport(_) | NoiseProtocolState::Failed(_) => {
                // No handshake messages to send in these states
                None
            }
        }
    }

    /// Encrypts an application data payload for sending during the Transport phase.
    ///
    /// Returns the ciphertext (payload + 16-byte tag).
    /// Errors if not in Transport state or encryption fails.
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, NoiseError> {
        match &mut self.state {
            NoiseProtocolState::Transport(transport_state) => {
                let mut buffer = vec![0u8; payload.len() + 16]; // Payload + tag
                match transport_state.write_message(payload, &mut buffer) {
                    Ok(len) => Ok(buffer[..len].to_vec()),
                    Err(e) => {
                        let err = NoiseError::from(e);
                        self.state = NoiseProtocolState::Failed(err.to_string());
                        Err(err)
                    }
                }
            }
            NoiseProtocolState::Handshaking(_) | NoiseProtocolState::Failed(_) => {
                Err(NoiseError::IncorrectStateError)
            }
        }
    }

    /// Returns true if the protocol is in the transport phase (handshake complete).
    pub fn is_transport(&self) -> bool {
        matches!(self.state, NoiseProtocolState::Transport(_))
    }

    /// Returns true if the protocol has failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.state, NoiseProtocolState::Failed(_))
    }

    /// Check if the handshake has finished and the protocol is in transport mode.
    pub fn is_handshake_finished(&self) -> bool {
        matches!(self.state, NoiseProtocolState::Transport(_))
    }

    /// Inject a PSK into the Noise HandshakeState.
    ///
    /// This allows dynamic PSK injection after HandshakeState construction,
    /// which is required for PSQ (Post-Quantum Secure PSK) integration where
    /// the PSK is derived during the handshake process.
    ///
    /// # Arguments
    /// * `index` - PSK index (typically 3 for XKpsk3 pattern)
    /// * `psk` - The pre-shared key bytes to inject
    ///
    /// # Errors
    /// Returns an error if:
    /// - Not in handshake state
    /// - The underlying snow library rejects the PSK
    pub fn set_psk(&mut self, index: u8, psk: &[u8]) -> Result<(), NoiseError> {
        match &mut self.state {
            NoiseProtocolState::Handshaking(handshake_state) => {
                handshake_state
                    .set_psk(index as usize, psk)
                    .map_err(NoiseError::ProtocolError)?;
                Ok(())
            }
            _ => Err(NoiseError::IncorrectStateError),
        }
    }
}

pub fn create_noise_state(
    local_private_key: &[u8],
    remote_public_key: &[u8],
    psk: &[u8],
) -> Result<NoiseProtocol, NoiseError> {
    let pattern_name = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
    let psk_index = 3;
    let noise_params: NoiseParams = pattern_name.parse().unwrap();

    let builder = snow::Builder::new(noise_params.clone());
    // Using dummy remote key as it's not needed for state creation itself
    // In a real scenario, the key would depend on initiator/responder role
    let handshake_state = builder
        .local_private_key(local_private_key)
        .remote_public_key(remote_public_key) // Use own public as dummy remote
        .psk(psk_index, psk)
        .build_initiator()?;
    Ok(NoiseProtocol::new(handshake_state))
}

pub fn create_noise_state_responder(
    local_private_key: &[u8],
    remote_public_key: &[u8],
    psk: &[u8],
) -> Result<NoiseProtocol, NoiseError> {
    let pattern_name = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
    let psk_index = 3;
    let noise_params: NoiseParams = pattern_name.parse().unwrap();

    let builder = snow::Builder::new(noise_params.clone());
    // Using dummy remote key as it's not needed for state creation itself
    // In a real scenario, the key would depend on initiator/responder role
    let handshake_state = builder
        .local_private_key(local_private_key)
        .remote_public_key(remote_public_key) // Use own public as dummy remote
        .psk(psk_index, psk)
        .build_responder()?;
    Ok(NoiseProtocol::new(handshake_state))
}
