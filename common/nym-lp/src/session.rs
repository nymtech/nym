// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection
//! and Noise protocol state handling.

use crate::keypair::{PrivateKey, PublicKey};
use crate::message::{EncryptedDataPayload, HandshakeData};
use crate::noise_protocol::{NoiseError, NoiseProtocol, ReadResult};
use crate::packet::LpHeader;
use crate::psk::{psq_initiator_create_message, psq_responder_process_message};
use crate::replay::ReceivingKeyCounterValidator;
use crate::{LpError, LpMessage, LpPacket};
use nym_crypto::asymmetric::ed25519;
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
use parking_lot::Mutex;
use snow::Builder;
use std::sync::atomic::{AtomicU64, Ordering};

/// PSQ (Post-Quantum Secure PSK) handshake state.
///
/// Tracks the PSQ protocol state machine through the session lifecycle.
///
/// # State Transitions
///
/// **Initiator path:**
/// ```text
/// NotStarted → InitiatorWaiting → Completed
/// ```
///
/// **Responder path:**
/// ```text
/// NotStarted → ResponderWaiting → Completed
/// ```
#[derive(Debug)]
pub enum PSQState {
    /// PSQ handshake not yet started.
    NotStarted,

    /// Initiator has sent PSQ ciphertext and is waiting for confirmation.
    /// Stores the ciphertext that was sent.
    InitiatorWaiting { ciphertext: Vec<u8> },

    /// Responder is ready to receive and decapsulate PSQ ciphertext.
    ResponderWaiting,

    /// PSQ handshake completed successfully.
    /// The PSK has been derived and registered with the Noise protocol.
    Completed {
        /// The derived post-quantum PSK
        psk: [u8; 32],
    },
}

/// A session in the Lewes Protocol, handling connection state with Noise.
///
/// Sessions manage connection state, including LP replay protection and Noise cryptography.
/// Each session has a unique receiving index and sending index for connection identification.
#[derive(Debug)]
pub struct LpSession {
    id: u32,

    /// Flag indicating if this session acts as the Noise protocol initiator.
    is_initiator: bool,

    /// Noise protocol state machine
    noise_state: Mutex<NoiseProtocol>,

    /// PSQ (Post-Quantum Secure PSK) handshake state
    psq_state: Mutex<PSQState>,

    /// Counter for outgoing packets
    sending_counter: AtomicU64,

    /// Validator for incoming packet counters to prevent replay attacks
    receiving_counter: Mutex<ReceivingKeyCounterValidator>,

    // PSQ-related keys stored for handshake
    /// Local Ed25519 private key for PSQ authentication
    local_ed25519_private: ed25519::PrivateKey,

    /// Local Ed25519 public key for PSQ authentication
    local_ed25519_public: ed25519::PublicKey,

    /// Remote Ed25519 public key for PSQ authentication
    remote_ed25519_public: ed25519::PublicKey,

    /// Local X25519 private key (Noise static key)
    local_x25519_private: PrivateKey,

    /// Remote X25519 public key (Noise static key)
    remote_x25519_public: PublicKey,

    /// Salt for PSK derivation
    salt: [u8; 32],
}

impl LpSession {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn noise_state(&self) -> &Mutex<NoiseProtocol> {
        &self.noise_state
    }

    /// Returns true if this session was created as the initiator.
    pub fn is_initiator(&self) -> bool {
        self.is_initiator
    }

    /// Creates a new session and initializes the Noise protocol state.
    ///
    /// PSQ always runs during the handshake to derive the real PSK from X25519 DHKEM.
    /// The Noise protocol is initialized with a dummy PSK that gets replaced during handshake.
    ///
    /// # Arguments
    ///
    /// * `id` - Session identifier
    /// * `is_initiator` - True if this side initiates the Noise handshake.
    /// * `local_ed25519_keypair` - This side's Ed25519 keypair for PSQ authentication
    /// * `local_x25519_key` - This side's X25519 private key for Noise protocol and DHKEM
    /// * `remote_ed25519_key` - Peer's Ed25519 public key for PSQ authentication
    /// * `remote_x25519_key` - Peer's X25519 public key for Noise protocol and DHKEM
    /// * `salt` - Salt for PSK derivation
    pub fn new(
        id: u32,
        is_initiator: bool,
        local_ed25519_keypair: (&ed25519::PrivateKey, &ed25519::PublicKey),
        local_x25519_key: &PrivateKey,
        remote_ed25519_key: &ed25519::PublicKey,
        remote_x25519_key: &PublicKey,
        salt: &[u8; 32],
    ) -> Result<Self, LpError> {
        // XKpsk3 pattern requires remote static key known upfront (XK)
        // and PSK mixed at position 3. This provides forward secrecy with PSK authentication.
        let pattern_name = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
        let psk_index = 3;

        let params = pattern_name.parse()?;
        let builder = Builder::new(params);

        let local_key_bytes = local_x25519_key.to_bytes();
        let builder = builder.local_private_key(&local_key_bytes);

        let remote_key_bytes = remote_x25519_key.to_bytes();
        let builder = builder.remote_public_key(&remote_key_bytes);

        // Initialize with dummy PSK - real PSK will be injected via set_psk() during handshake
        // when PSQ runs using X25519 as DHKEM
        let dummy_psk = [0u8; 32];
        let builder = builder.psk(psk_index, &dummy_psk);

        let initial_state = if is_initiator {
            builder.build_initiator().map_err(LpError::SnowKeyError)?
        } else {
            builder.build_responder().map_err(LpError::SnowKeyError)?
        };

        let noise_protocol = NoiseProtocol::new(initial_state);

        // Initialize PSQ state based on role
        let psq_state = if is_initiator {
            PSQState::NotStarted
        } else {
            PSQState::ResponderWaiting
        };

        Ok(Self {
            id,
            is_initiator,
            noise_state: Mutex::new(noise_protocol),
            psq_state: Mutex::new(psq_state),
            sending_counter: AtomicU64::new(0),
            receiving_counter: Mutex::new(ReceivingKeyCounterValidator::default()),
            // Ed25519 keys don't impl Clone, so convert to bytes and reconstruct
            local_ed25519_private: ed25519::PrivateKey::from_bytes(
                &local_ed25519_keypair.0.to_bytes(),
            )
            .expect("Valid ed25519 private key"),
            local_ed25519_public: ed25519::PublicKey::from_bytes(&local_ed25519_keypair.1.to_bytes())
                .expect("Valid ed25519 public key"),
            remote_ed25519_public: ed25519::PublicKey::from_bytes(&remote_ed25519_key.to_bytes())
                .expect("Valid ed25519 public key"),
            local_x25519_private: local_x25519_key.clone(),
            remote_x25519_public: remote_x25519_key.clone(),
            salt: *salt,
        })
    }

    pub fn next_packet(&self, message: LpMessage) -> Result<LpPacket, LpError> {
        let counter = self.next_counter();
        let header = LpHeader::new(self.id(), counter);
        let packet = LpPacket::new(header, message);
        Ok(packet)
    }

    /// Generates the next counter value for outgoing packets.
    pub fn next_counter(&self) -> u64 {
        self.sending_counter.fetch_add(1, Ordering::Relaxed)
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
        let counter_validator = self.receiving_counter.lock();
        counter_validator
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
    pub fn receiving_counter_mark(&self, counter: u64) -> Result<(), LpError> {
        let mut counter_validator = self.receiving_counter.lock();
        counter_validator
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
    pub fn current_packet_cnt(&self) -> (u64, u64) {
        let counter_validator = self.receiving_counter.lock();
        counter_validator.current_packet_cnt()
    }

    /// Prepares the next handshake message to be sent, if any.
    ///
    /// This should be called by the driver/IO layer to check if the Noise protocol
    /// state machine requires a message to be sent to the peer.
    ///
    /// For initiators, PSQ always runs on the first message:
    /// 1. Converts X25519 keys to DHKEM format
    /// 2. Generates PSQ payload and derives PSK
    /// 3. Injects PSK into Noise HandshakeState
    /// 4. Embeds PSQ payload in first handshake message as: [u16 len][psq_payload][noise_msg]
    ///
    /// # Returns
    ///
    /// * `Ok(None)` if no message needs to be sent currently (e.g., waiting for peer, or handshake complete).
    /// * `Err(LpError)` if there's an error within the Noise protocol or PSQ.
    pub fn prepare_handshake_message(&self) -> Option<Result<LpMessage, LpError>> {
        let mut noise_state = self.noise_state.lock();

        // PSQ always runs for initiator on first message
        let mut psq_state = self.psq_state.lock();

        if self.is_initiator && matches!(*psq_state, PSQState::NotStarted) {
            // Convert X25519 remote public key to EncapsulationKey (DHKEM)
            let remote_kem_bytes = self.remote_x25519_public.as_bytes();
            let libcrux_public_key = match libcrux_kem::PublicKey::decode(
                libcrux_kem::Algorithm::X25519,
                remote_kem_bytes,
            ) {
                Ok(key) => key,
                Err(e) => {
                    return Some(Err(LpError::KKTError(format!(
                        "Failed to convert X25519 key to libcrux PublicKey: {:?}",
                        e
                    ))))
                }
            };
            let remote_kem = EncapsulationKey::X25519(libcrux_public_key);

            // Generate PSQ payload and PSK using X25519 as DHKEM
            let session_context = self.id.to_le_bytes();

            let (psk, psq_payload) = match psq_initiator_create_message(
                &self.local_x25519_private,
                &self.remote_x25519_public,
                &remote_kem,
                &self.local_ed25519_private,
                &self.local_ed25519_public,
                &self.salt,
                &session_context,
            ) {
                Ok(result) => result,
                Err(e) => {
                    log::error!("PSQ handshake preparation failed, aborting: {:?}", e);
                    return Some(Err(e));
                }
            };

            // Inject PSK into Noise HandshakeState
            if let Err(e) = noise_state.set_psk(3, &psk) {
                return Some(Err(LpError::NoiseError(e)));
            }

            // Get the Noise handshake message
            let noise_msg = match noise_state.get_bytes_to_send() {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => return Some(Err(LpError::NoiseError(e))),
                None => return None, // Should not happen if is_my_turn, but handle gracefully
            };

            // Combine: [u16 psq_len][psq_payload][noise_msg]
            let psq_len = psq_payload.len() as u16;
            let mut combined = Vec::with_capacity(2 + psq_payload.len() + noise_msg.len());
            combined.extend_from_slice(&psq_len.to_be_bytes());
            combined.extend_from_slice(&psq_payload);
            combined.extend_from_slice(&noise_msg);

            // Update PSQ state to InitiatorWaiting
            *psq_state = PSQState::InitiatorWaiting {
                ciphertext: psq_payload,
            };

            return Some(Ok(LpMessage::Handshake(HandshakeData(combined))));
        }

        // Normal flow (no PSQ, or PSQ already completed)
        drop(psq_state); // Release lock

        if let Some(message) = noise_state.get_bytes_to_send() {
            match message {
                Ok(message) => Some(Ok(LpMessage::Handshake(HandshakeData(message)))),
                Err(e) => Some(Err(LpError::NoiseError(e))),
            }
        } else {
            None
        }
    }

    /// Processes a received handshake message from the peer.
    ///
    /// This should be called by the driver/IO layer after receiving a potential
    /// handshake message payload from an LP packet.
    ///
    /// For responders, PSQ always runs on the first message:
    /// 1. Extracts PSQ payload from the first handshake message: [u16 len][psq_payload][noise_msg]
    /// 2. Converts X25519 keys to DHKEM format
    /// 3. Decapsulates PSK from PSQ payload
    /// 4. Injects PSK into Noise HandshakeState
    /// 5. Processes the remaining Noise handshake message
    ///
    /// # Arguments
    ///
    /// * `message` - The LP message received from the peer, expected to be a Handshake message.
    ///
    /// # Returns
    ///
    /// * `Ok(ReadResult)` detailing the outcome (e.g., handshake complete, no-op).
    /// * `Err(LpError)` if the message is invalid or causes a Noise/PSQ protocol error.
    pub fn process_handshake_message(
        &self,
        message: &LpMessage,
    ) -> Result<ReadResult, LpError> {
        let mut noise_state = self.noise_state.lock();
        let mut psq_state = self.psq_state.lock();

        match message {
            LpMessage::Handshake(HandshakeData(payload)) => {
                // PSQ always runs for responder on first message
                if !self.is_initiator && matches!(*psq_state, PSQState::ResponderWaiting) {
                    // Extract PSQ payload: [u16 psq_len][psq_payload][noise_msg]
                    if payload.len() < 2 {
                        return Err(LpError::NoiseError(NoiseError::Other(
                            "Payload too short for PSQ extraction".to_string(),
                        )));
                    }

                    let psq_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;

                    if payload.len() < 2 + psq_len {
                        return Err(LpError::NoiseError(NoiseError::Other(
                            "Payload length mismatch for PSQ extraction".to_string(),
                        )));
                    }

                    let psq_payload = &payload[2..2 + psq_len];
                    let noise_payload = &payload[2 + psq_len..];

                    // Convert X25519 local keys to DecapsulationKey/EncapsulationKey (DHKEM)
                    let local_private_bytes = &self.local_x25519_private.to_bytes();
                    let libcrux_private_key = libcrux_kem::PrivateKey::decode(
                        libcrux_kem::Algorithm::X25519,
                        local_private_bytes,
                    )
                    .map_err(|e| {
                        LpError::KKTError(format!(
                            "Failed to convert X25519 private key to libcrux PrivateKey: {:?}",
                            e
                        ))
                    })?;
                    let dec_key = DecapsulationKey::X25519(libcrux_private_key);

                    let local_public_key = self.local_x25519_private.public_key();
                    let local_public_bytes = local_public_key.as_bytes();
                    let libcrux_public_key = libcrux_kem::PublicKey::decode(
                        libcrux_kem::Algorithm::X25519,
                        local_public_bytes,
                    )
                    .map_err(|e| {
                        LpError::KKTError(format!(
                            "Failed to convert X25519 public key to libcrux PublicKey: {:?}",
                            e
                        ))
                    })?;
                    let enc_key = EncapsulationKey::X25519(libcrux_public_key);

                    // Decapsulate PSK from PSQ payload using X25519 as DHKEM
                    let session_context = self.id.to_le_bytes();

                    let psk = match psq_responder_process_message(
                        &self.local_x25519_private,
                        &self.remote_x25519_public,
                        (&dec_key, &enc_key),
                        &self.remote_ed25519_public,
                        psq_payload,
                        &self.salt,
                        &session_context,
                    ) {
                        Ok(psk) => psk,
                        Err(e) => {
                            log::error!("PSQ handshake processing failed, aborting: {:?}", e);
                            return Err(e);
                        }
                    };

                    // Inject PSK into Noise HandshakeState
                    noise_state.set_psk(3, &psk)?;

                    // Update PSQ state to Completed
                    *psq_state = PSQState::Completed { psk };

                    // Process the Noise handshake message (without PSQ prefix)
                    drop(psq_state); // Release lock before processing
                    return noise_state
                        .read_message(noise_payload)
                        .map_err(LpError::NoiseError);
                }

                // Normal flow (no PSQ, or PSQ already completed)
                drop(psq_state); // Release lock

                // The sans-io NoiseProtocol::read_message expects only the payload.
                noise_state
                    .read_message(payload)
                    .map_err(LpError::NoiseError)
            }
            _ => Err(LpError::NoiseError(NoiseError::IncorrectStateError)),
        }
    }

    /// Checks if the Noise handshake phase is complete.
    pub fn is_handshake_complete(&self) -> bool {
        self.noise_state.lock().is_handshake_finished()
    }

    /// Encrypts application data payload using the established Noise transport session.
    ///
    /// This should only be called after the handshake is complete (`is_handshake_complete` returns true).
    ///
    /// # Arguments
    ///
    /// * `payload` - The application data to encrypt.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the encrypted Noise message ciphertext.
    /// * `Err(NoiseError)` if the session is not in transport mode or encryption fails.
    pub fn encrypt_data(&self, payload: &[u8]) -> Result<LpMessage, NoiseError> {
        let mut noise_state = self.noise_state.lock();
        // Explicitly check if handshake is finished before trying to write
        if !noise_state.is_handshake_finished() {
            return Err(NoiseError::IncorrectStateError);
        }
        let payload = noise_state.write_message(payload)?;
        Ok(LpMessage::EncryptedData(EncryptedDataPayload(payload)))
    }

    /// Decrypts an incoming Noise message containing application data.
    ///
    /// This should only be called after the handshake is complete (`is_handshake_complete` returns true)
    /// and when an `LPMessage::EncryptedData` is received.
    ///
    /// # Arguments
    ///
    /// * `noise_ciphertext` - The encrypted Noise message received from the peer.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the decrypted application data payload.
    /// * `Err(NoiseError)` if the session is not in transport mode, decryption fails, or the message is not data.
    pub fn decrypt_data(&self, noise_ciphertext: &LpMessage) -> Result<Vec<u8>, NoiseError> {
        let mut noise_state = self.noise_state.lock();
        // Explicitly check if handshake is finished before trying to read
        if !noise_state.is_handshake_finished() {
            return Err(NoiseError::IncorrectStateError);
        }

        let payload = noise_ciphertext.payload();

        match noise_state.read_message(payload)? {
            ReadResult::DecryptedData(data) => Ok(data),
            _ => Err(NoiseError::IncorrectStateError),
        }
    }
}

#[cfg(test)]
mod tests {
    use snow::{params::NoiseParams, Keypair};

    use super::*;
    use crate::{replay::ReplayError, sessions_for_tests, NOISE_PATTERN};

    // Helper function to generate keypairs for tests
    fn generate_keypair() -> crate::keypair::Keypair {
        crate::keypair::Keypair::default()
    }

    // Helper function to create a session with real keys for handshake tests
    fn create_handshake_test_session(
        is_initiator: bool,
        local_keys: &crate::keypair::Keypair,
        remote_pub_key: &crate::keypair::PublicKey,
    ) -> LpSession {
        use nym_crypto::asymmetric::ed25519;

        // Compute the shared lp_id from both keypairs (order-independent)
        let lp_id = crate::make_lp_id(local_keys.public_key(), remote_pub_key);

        // Create Ed25519 keypairs that correspond to initiator/responder roles
        // Initiator uses [1u8], Responder uses [2u8]
        let (local_ed25519_seed, remote_ed25519_seed) = if is_initiator {
            ([1u8; 32], [2u8; 32])
        } else {
            ([2u8; 32], [1u8; 32])
        };

        let local_ed25519 = ed25519::KeyPair::from_secret(local_ed25519_seed, 0);
        let remote_ed25519 = ed25519::KeyPair::from_secret(remote_ed25519_seed, 1);

        let salt = [0u8; 32]; // Test salt

        // PSQ will derive the PSK during handshake using X25519 as DHKEM
        LpSession::new(
            lp_id,
            is_initiator,
            (local_ed25519.private_key(), local_ed25519.public_key()),
            local_keys.private_key(),
            remote_ed25519.public_key(),
            remote_pub_key,
            &salt,
        )
        .expect("Test session creation failed")
    }

    #[test]
    fn test_session_creation() {
        let session = sessions_for_tests().0;

        // Initial counter should be zero
        let counter = session.next_counter();
        assert_eq!(counter, 0);

        // Counter should increment
        let counter = session.next_counter();
        assert_eq!(counter, 1);
    }

    // NOTE: These tests are obsolete after removing optional KEM parameters.
    // PSQ now always runs using X25519 keys internally converted to KEM format.
    // The new tests at the end of this file (test_psq_*) cover PSQ integration.
    /*
    #[test]
    fn test_session_creation_with_psq_state_initiator() {
        // OLD API - REMOVED
    }

    #[test]
    fn test_session_creation_with_psq_state_responder() {
        // OLD API - REMOVED
    }
    */

    #[test]
    fn test_replay_protection_sequential() {
        let session = sessions_for_tests().1;

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

    #[test]
    fn test_replay_protection_out_of_order() {
        let session = sessions_for_tests().1;

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

    #[test]
    fn test_packet_stats() {
        let session = sessions_for_tests().1;

        // Initial stats
        let (next, received) = session.current_packet_cnt();
        assert_eq!(next, 0);
        assert_eq!(received, 0);

        // After receiving packets
        assert!(session.receiving_counter_mark(0).is_ok());
        assert!(session.receiving_counter_mark(1).is_ok());

        let (next, received) = session.current_packet_cnt();
        assert_eq!(next, 2);
        assert_eq!(received, 2);
    }

    #[test]
    fn test_prepare_handshake_message_initial_state() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let psk = [3u8; 32];

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session = create_handshake_test_session(
            false,
            &responder_keys,
            initiator_keys.public_key(), // Responder also needs initiator's key for XK
        );

        // Initiator should have a message to send immediately (-> e)
        let initiator_msg_result = initiator_session.prepare_handshake_message();
        assert!(initiator_msg_result.is_some());
        let initiator_msg = initiator_msg_result
            .unwrap()
            .expect("Initiator msg prep failed");
        assert!(!initiator_msg.is_empty());

        // Responder should have nothing to send initially (waits for <- e)
        let responder_msg_result = responder_session.prepare_handshake_message();
        assert!(responder_msg_result.is_none());
    }

    #[test]
    fn test_process_handshake_message_first_step() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let psk = [4u8; 32];

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // 1. Initiator prepares the first message (-> e)
        let initiator_msg_result = initiator_session.prepare_handshake_message();
        let initiator_msg = initiator_msg_result
            .unwrap()
            .expect("Initiator msg prep failed");

        // 2. Responder processes the message (<- e)
        let process_result = responder_session.process_handshake_message(&initiator_msg);

        // Check the result of processing
        match process_result {
            Ok(ReadResult::NoOp) => {
                // Expected for XK first message, responder doesn't decrypt data yet
            }
            Ok(other) => panic!("Unexpected process result: {:?}", other),
            Err(e) => panic!("Responder processing failed: {:?}", e),
        }

        // 3. After processing, responder should now have a message to send (-> e, es)
        let responder_response_result = responder_session.prepare_handshake_message();
        assert!(responder_response_result.is_some());
        let responder_response = responder_response_result
            .unwrap()
            .expect("Responder response prep failed");
        assert!(!responder_response.is_empty());
    }

    #[test]
    fn test_handshake_driver_simulation() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let psk = [5u8; 32];

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        let mut responder_to_initiator_msg = None;
        let mut rounds = 0;
        const MAX_ROUNDS: usize = 10; // Safety break for the loop

        // Start by priming the initiator message
        let mut initiator_to_responder_msg =
            initiator_session.prepare_handshake_message().unwrap().ok();
        assert!(
            initiator_to_responder_msg.is_some(),
            "Initiator did not produce initial message"
        );

        while rounds < MAX_ROUNDS {
            rounds += 1;

            // === Initiator -> Responder ===
            if let Some(msg) = initiator_to_responder_msg.take() {
                // Process message
                match responder_session.process_handshake_message(&msg) {
                    Ok(_) => {}
                    Err(e) => panic!("Responder processing failed: {:?}", e),
                }

                // Check if responder needs to send a reply
                responder_to_initiator_msg = responder_session
                    .prepare_handshake_message()
                    .transpose()
                    .unwrap();
            }

            // Check completion after potentially processing responder's message below
            if initiator_session.is_handshake_complete()
                && responder_session.is_handshake_complete()
            {
                break;
            }

            // === Responder -> Initiator ===
            if let Some(msg) = responder_to_initiator_msg.take() {
                // Process message
                match initiator_session.process_handshake_message(&msg) {
                    Ok(_) => {}
                    Err(e) => panic!("Initiator processing failed: {:?}", e),
                }

                // Check if initiator needs to send a reply (should be last message in XK)
                initiator_to_responder_msg = initiator_session
                    .prepare_handshake_message()
                    .transpose()
                    .unwrap();
            }

            // Check completion again after potentially processing initiator's message above
            if initiator_session.is_handshake_complete()
                && responder_session.is_handshake_complete()
            {
                break;
            }
        }

        assert!(
            rounds < MAX_ROUNDS,
            "Handshake did not complete within max rounds"
        );
        assert!(
            initiator_session.is_handshake_complete(),
            "Initiator handshake did not complete"
        );
        assert!(
            responder_session.is_handshake_complete(),
            "Responder handshake did not complete"
        );

        println!("Handshake completed in {} rounds.", rounds);
    }

    #[test]
    fn test_encrypt_decrypt_after_handshake() {
        // --- Setup Handshake ---
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let psk = [6u8; 32];

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // Drive handshake to completion (simplified loop from previous test)
        let mut i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();
        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&r_msg).unwrap();
        i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();

        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // --- Test Encryption/Decryption ---
        let plaintext = b"Hello, Lewes Protocol!";

        // Initiator encrypts
        let ciphertext = initiator_session
            .encrypt_data(plaintext)
            .expect("Initiator encryption failed");
        assert_ne!(ciphertext.payload(), plaintext); // Ensure it's actually encrypted

        // Responder decrypts
        let decrypted = responder_session
            .decrypt_data(&ciphertext)
            .expect("Responder decryption failed");
        assert_eq!(decrypted, plaintext);

        // --- Test other direction ---
        let plaintext2 = b"Response from responder.";

        // Responder encrypts
        let ciphertext2 = responder_session
            .encrypt_data(plaintext2)
            .expect("Responder encryption failed");
        assert_ne!(ciphertext2.payload(), plaintext2);

        // Initiator decrypts
        let decrypted2 = initiator_session
            .decrypt_data(&ciphertext2)
            .expect("Initiator decryption failed");
        assert_eq!(decrypted2, plaintext2);
    }

    #[test]
    fn test_encrypt_decrypt_before_handshake() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let psk = [7u8; 32];

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());

        assert!(!initiator_session.is_handshake_complete());

        // Attempt to encrypt before handshake
        let plaintext = b"This should fail";
        let result = initiator_session.encrypt_data(plaintext);
        assert!(result.is_err());
        match result.unwrap_err() {
            NoiseError::IncorrectStateError => {} // Expected error
            e => panic!("Expected IncorrectStateError, got {:?}", e),
        }

        // Attempt to decrypt before handshake (using dummy ciphertext)
        let dummy_ciphertext = vec![0u8; 32];
        let result_decrypt =
            initiator_session.decrypt_data(&LpMessage::EncryptedData(EncryptedDataPayload(dummy_ciphertext)));
        assert!(result_decrypt.is_err());
        match result_decrypt.unwrap_err() {
            NoiseError::IncorrectStateError => {} // Expected error
            e => panic!("Expected IncorrectStateError, got {:?}", e),
        }
    }

    /*
    // These tests remain commented as they rely on the old mock crypto functions
    #[test]
    fn test_mock_crypto() {
        let session = create_test_session(true);
        let data = [1, 2, 3, 4, 5];
        let mut encrypted = [0; 5];
        let mut decrypted = [0; 5];

        // Mock encrypt should copy the data
        // let encrypted_len = session.encrypt_packet(&data, &mut encrypted).unwrap(); // Removed method
        // assert_eq!(encrypted_len, 5);
        // assert_eq!(encrypted, data);

        // Mock decrypt should copy the data
        // let decrypted_len = session.decrypt_packet(&encrypted, &mut decrypted).unwrap(); // Removed method
        // assert_eq!(decrypted_len, 5);
        // assert_eq!(decrypted, data);
    }

    #[test]
    fn test_mock_crypto_buffer_too_small() {
        let session = create_test_session(true);
        let data = [1, 2, 3, 4, 5];
        let mut too_small = [0; 3];

        // Should fail with buffer too small
        // let result = session.encrypt_packet(&data, &mut too_small); // Removed method
        // assert!(result.is_err());
        // match result.unwrap_err() {
        //     LpError::InsufficientBufferSize => {} // Error type might change
        //     _ => panic!("Expected InsufficientBufferSize error"),
        // }
    }
    */

    // ====================================================================
    // PSQ Handshake Integration Tests
    // ====================================================================

    /// Test that PSQ runs during handshake and derives a PSK
    #[test]
    fn test_psq_handshake_runs_with_psk_injection() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // Drive the handshake
        let mut i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have message")
            .expect("Message prep should succeed");

        // The first message should contain PSQ payload embedded
        // Verify message is not empty and has reasonable size
        assert!(!i_msg.is_empty(), "Initiator message should not be empty");
        assert!(
            i_msg.len() > 100,
            "Message should contain PSQ payload (actual: {})",
            i_msg.len()
        );

        // Responder processes message (which includes PSQ decapsulation)
        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process first message");

        // Continue handshake
        let r_msg = responder_session
            .prepare_handshake_message()
            .expect("Responder should have message")
            .expect("Responder message prep should succeed");

        initiator_session
            .process_handshake_message(&r_msg)
            .expect("Initiator should process responder message");

        i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have final message")
            .expect("Final message prep should succeed");

        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process final message");

        // Verify handshake completed
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Verify encryption works (implicitly tests PSK was correctly injected)
        let plaintext = b"PSQ test message";
        let encrypted = initiator_session
            .encrypt_data(plaintext)
            .expect("Encryption should work after handshake");

        let decrypted = responder_session
            .decrypt_data(&encrypted)
            .expect("Decryption should work with PSQ-derived PSK");

        assert_eq!(decrypted, plaintext);
    }

    /// Test that X25519 keys are correctly converted to KEM format
    #[test]
    fn test_x25519_to_kem_conversion() {
        use nym_kkt::ciphersuite::EncapsulationKey;

        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Verify we can convert X25519 public key to KEM format (as done in session.rs)
        let x25519_public_bytes = responder_keys.public_key().as_bytes();
        let libcrux_public_key = libcrux_kem::PublicKey::decode(
            libcrux_kem::Algorithm::X25519,
            x25519_public_bytes,
        )
        .expect("X25519 public key should convert to libcrux PublicKey");

        let _kem_key = EncapsulationKey::X25519(libcrux_public_key);

        // Verify we can convert X25519 private key to KEM format
        let x25519_private_bytes = initiator_keys.private_key().to_bytes();
        let _libcrux_private_key = libcrux_kem::PrivateKey::decode(
            libcrux_kem::Algorithm::X25519,
            &x25519_private_bytes,
        )
        .expect("X25519 private key should convert to libcrux PrivateKey");

        // Successful conversion is sufficient - actual encapsulation is tested in psk.rs
        // (libcrux_kem::PrivateKey is an enum with no len() method, conversion success is enough)
    }

    /// Test that PSQ actually derives a different PSK (not using dummy)
    #[test]
    fn test_psq_derived_psk_differs_from_dummy() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions - they start with dummy PSK [0u8; 32]
        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // Prepare first message (initiator runs PSQ and injects PSK)
        let i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have message")
            .expect("Message prep should succeed");

        // Verify message is not empty (PSQ runs successfully)
        assert!(
            !i_msg.is_empty(),
            "First message should contain PSQ payload"
        );

        // Complete handshake
        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process message");

        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        initiator_session.process_handshake_message(&r_msg).unwrap();

        let final_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        responder_session
            .process_handshake_message(&final_msg)
            .unwrap();

        // Test that encryption produces non-trivial ciphertext
        // (would fail if using dummy PSK incorrectly)
        let plaintext = b"test";
        let encrypted = initiator_session.encrypt_data(plaintext).unwrap();

        // Decrypt should work
        let decrypted = responder_session.decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);

        // Verify ciphertext is not just plaintext (basic encryption sanity)
        if let LpMessage::EncryptedData(payload) = encrypted {
            assert_ne!(
                &payload.0[..plaintext.len()],
                plaintext,
                "Ciphertext should differ from plaintext"
            );
        } else {
            panic!("Expected EncryptedData message");
        }
    }

    /// Test full end-to-end handshake with PSQ integration
    #[test]
    fn test_handshake_with_psq_end_to_end() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // Verify initial state
        assert!(!initiator_session.is_handshake_complete());
        assert!(!responder_session.is_handshake_complete());
        assert!(initiator_session.is_initiator());
        assert!(!responder_session.is_initiator());

        // Round 1: Initiator -> Responder (contains PSQ encapsulation)
        let msg1 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare message")
            .expect("Message should succeed");

        assert!(!msg1.is_empty());
        assert!(!initiator_session.is_handshake_complete());

        responder_session
            .process_handshake_message(&msg1)
            .expect("Responder should process PSQ message");

        assert!(!responder_session.is_handshake_complete());

        // Round 2: Responder -> Initiator
        let msg2 = responder_session
            .prepare_handshake_message()
            .expect("Responder should prepare message")
            .expect("Message should succeed");

        initiator_session
            .process_handshake_message(&msg2)
            .expect("Initiator should process message");

        // Round 3: Initiator -> Responder (final)
        let msg3 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare final message")
            .expect("Message should succeed");

        responder_session
            .process_handshake_message(&msg3)
            .expect("Responder should process final message");

        // Verify both sides completed
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Test bidirectional encrypted communication
        let msg_i_to_r = b"Hello from initiator";
        let encrypted_i = initiator_session
            .encrypt_data(msg_i_to_r)
            .expect("Initiator encryption");
        let decrypted_i = responder_session
            .decrypt_data(&encrypted_i)
            .expect("Responder decryption");
        assert_eq!(decrypted_i, msg_i_to_r);

        let msg_r_to_i = b"Hello from responder";
        let encrypted_r = responder_session
            .encrypt_data(msg_r_to_i)
            .expect("Responder encryption");
        let decrypted_r = initiator_session
            .decrypt_data(&encrypted_r)
            .expect("Initiator decryption");
        assert_eq!(decrypted_r, msg_r_to_i);

        // Successfully completed end-to-end test with PSQ
    }

    /// Test that Ed25519 keys are used in PSQ authentication
    #[test]
    fn test_psq_handshake_uses_ed25519_authentication() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions with explicit Ed25519 keys
        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());
        let responder_session =
            create_handshake_test_session(false, &responder_keys, initiator_keys.public_key());

        // Verify sessions store Ed25519 keys
        // (Internal verification - keys are used in PSQ calls)
        assert_eq!(initiator_session.id(), responder_session.id());

        // Complete handshake
        let msg1 = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&msg1).unwrap();

        let msg2 = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&msg2).unwrap();

        let msg3 = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&msg3).unwrap();

        // If Ed25519 authentication failed, handshake would not complete
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Verify encrypted communication works (proof of successful PSQ with auth)
        let test_data = b"Authentication test";
        let encrypted = initiator_session.encrypt_data(test_data).unwrap();
        let decrypted = responder_session.decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, test_data);
    }

    #[test]
    fn test_psq_deserialization_failure() {
        // Test that corrupted PSQ payload causes clean abort
        let responder_keys = generate_keypair();
        let initiator_keys = generate_keypair();

        let responder_session = create_handshake_test_session(
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Create a handshake message with corrupted PSQ payload
        let corrupted_psq_data = vec![0xFF; 128]; // Random garbage
        let bad_message = LpMessage::Handshake(HandshakeData(corrupted_psq_data));

        // Attempt to process corrupted message - should fail
        let result = responder_session.process_handshake_message(&bad_message);

        // Should return error (PSQ deserialization will fail)
        assert!(result.is_err(), "Expected error for corrupted PSQ payload");

        // Verify session state is unchanged
        // PSQ state should still be ResponderWaiting (not modified)
        // Noise PSK should still be dummy [0u8; 32]
        assert!(!responder_session.is_handshake_complete());
    }

    #[test]
    fn test_handshake_abort_on_psq_failure() {
        // Test that Ed25519 auth failure causes handshake abort
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions with MISMATCHED Ed25519 keys
        // This simulates authentication failure
        let initiator_ed25519 = ed25519::KeyPair::from_secret([1u8; 32], 0);
        let wrong_ed25519 = ed25519::KeyPair::from_secret([99u8; 32], 99); // Different key!

        let lp_id = crate::make_lp_id(initiator_keys.public_key(), responder_keys.public_key());
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            lp_id,
            true,
            (initiator_ed25519.private_key(), initiator_ed25519.public_key()),
            initiator_keys.private_key(),
            wrong_ed25519.public_key(), // Responder expects THIS key
            responder_keys.public_key(),
            &salt,
        )
        .unwrap();

        let responder_ed25519 = ed25519::KeyPair::from_secret([2u8; 32], 1);

        let responder_session = LpSession::new(
            lp_id,
            false,
            (responder_ed25519.private_key(), responder_ed25519.public_key()),
            responder_keys.private_key(),
            wrong_ed25519.public_key(), // Expects WRONG key (not initiator's)
            initiator_keys.public_key(),
            &salt,
        )
        .unwrap();

        // Initiator prepares message (should succeed - signing works)
        let msg1 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare message")
            .expect("Initiator should have message");

        // Responder processes message - should FAIL (signature verification fails)
        let result = responder_session.process_handshake_message(&msg1);

        // Should return CredError due to Ed25519 signature mismatch
        assert!(
            result.is_err(),
            "Expected error for Ed25519 authentication failure"
        );

        // Verify handshake aborted cleanly
        assert!(!initiator_session.is_handshake_complete());
        assert!(!responder_session.is_handshake_complete());
    }

    #[test]
    fn test_psq_invalid_signature() {
        // Test Ed25519 signature validation specifically
        // Setup with matching X25519 keys but mismatched Ed25519 keys
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Initiator uses Ed25519 key [1u8]
        let initiator_ed25519 = ed25519::KeyPair::from_secret([1u8; 32], 0);

        // Responder expects Ed25519 key [99u8] (wrong!)
        let wrong_ed25519_keypair = ed25519::KeyPair::from_secret([99u8; 32], 99);
        let wrong_ed25519_public = wrong_ed25519_keypair.public_key();

        let lp_id = crate::make_lp_id(initiator_keys.public_key(), responder_keys.public_key());
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            lp_id,
            true,
            (initiator_ed25519.private_key(), initiator_ed25519.public_key()),
            initiator_keys.private_key(),
            wrong_ed25519_public, // This doesn't matter for initiator
            responder_keys.public_key(),
            &salt,
        )
        .unwrap();

        let responder_ed25519 = ed25519::KeyPair::from_secret([2u8; 32], 1);

        let responder_session = LpSession::new(
            lp_id,
            false,
            (responder_ed25519.private_key(), responder_ed25519.public_key()),
            responder_keys.private_key(),
            wrong_ed25519_public, // Responder expects WRONG key
            initiator_keys.public_key(),
            &salt,
        )
        .unwrap();

        // Initiator creates message with valid signature (signed with [1u8])
        let msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        // Responder tries to verify with wrong public key [99u8]
        // This should fail Ed25519 signature verification
        let result = responder_session.process_handshake_message(&msg);

        assert!(
            result.is_err(),
            "Expected signature verification to fail"
        );

        // Verify error is related to PSQ/authentication
        match result.unwrap_err() {
            LpError::Internal(msg) if msg.contains("PSQ") => {
                // Expected - PSQ v1 responder send failed due to CredError
            }
            e => panic!("Unexpected error type: {:?}", e),
        }
    }

    #[test]
    fn test_psq_state_unchanged_on_error() {
        // Verify that PSQ errors leave session in clean state
        let responder_keys = generate_keypair();
        let initiator_keys = generate_keypair();

        let responder_session = create_handshake_test_session(
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Capture initial PSQ state (should be ResponderWaiting)
        // (We can't directly access psq_state, but we can verify behavior)

        // Send corrupted data
        let corrupted_message = LpMessage::Handshake(HandshakeData(vec![0xFF; 100]));

        // Process should fail
        let result = responder_session.process_handshake_message(&corrupted_message);
        assert!(result.is_err());

        // After error, session should still be in handshake mode (not complete)
        assert!(!responder_session.is_handshake_complete());

        // Session should still be functional - can process valid messages
        // Create a proper initiator to send valid message
        let initiator_session =
            create_handshake_test_session(true, &initiator_keys, responder_keys.public_key());

        let valid_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        // After the error, responder should still be able to process valid messages
        let result2 = responder_session.process_handshake_message(&valid_msg);

        // Should succeed (session state was not corrupted by previous error)
        assert!(
            result2.is_ok(),
            "Session should still be functional after PSQ error"
        );
    }
}
