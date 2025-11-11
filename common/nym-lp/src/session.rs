// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection
//! and Noise protocol state handling.

use crate::message::{EncryptedDataPayload, HandshakeData};
use crate::noise_protocol::{NoiseError, NoiseProtocol, ReadResult};
use crate::packet::LpHeader;
use crate::replay::ReceivingKeyCounterValidator;
use crate::{LpError, LpMessage, LpPacket};
use parking_lot::Mutex;
use snow::Builder;
use std::sync::atomic::{AtomicU64, Ordering};

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

    /// Counter for outgoing packets
    sending_counter: AtomicU64,

    /// Validator for incoming packet counters to prevent replay attacks
    receiving_counter: Mutex<ReceivingKeyCounterValidator>,
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
    /// # Arguments
    ///
    /// * `receiving_index` - Index used for receiving packets (becomes session ID).
    /// * `sending_index` - Index used for sending packets to the peer.
    /// * `is_initiator` - True if this side initiates the Noise handshake.
    /// * `local_static_key` - This side's static private key (e.g., X25519).
    /// * `remote_static_key` - The peer's static public key (required for initiator in some patterns like XK).
    /// * `psk` - The pre-shared key established out-of-band.
    /// * `pattern_name` - The Noise protocol pattern string (e.g., "Noise_XKpsk3_25519_ChaChaPoly_SHA256").
    /// * `psk_index` - The index/position where the PSK is mixed in according to the pattern.
    pub fn new(
        id: u32,
        is_initiator: bool,
        local_private_key: &[u8],
        remote_public_key: &[u8],
        psk: &[u8],
    ) -> Result<Self, LpError> {
        // XKpsk3 pattern requires remote static key known upfront (XK)
        // and PSK mixed at position 3. This provides forward secrecy with PSK authentication.
        let pattern_name = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
        let psk_index = 3;

        let params = pattern_name.parse()?;
        let builder = Builder::new(params);

        let builder = builder.local_private_key(local_private_key);

        let builder = builder.remote_public_key(remote_public_key);

        let builder = builder.psk(psk_index, psk);

        let initial_state = if is_initiator {
            builder.build_initiator().map_err(LpError::SnowKeyError)?
        } else {
            builder.build_responder().map_err(LpError::SnowKeyError)?
        };

        let noise_protocol = NoiseProtocol::new(initial_state);

        Ok(Self {
            id,
            is_initiator,
            noise_state: Mutex::new(noise_protocol),
            sending_counter: AtomicU64::new(0),
            receiving_counter: Mutex::new(ReceivingKeyCounterValidator::default()),
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
    /// # Returns
    ///
    /// * `Ok(None)` if no message needs to be sent currently (e.g., waiting for peer, or handshake complete).
    /// * `Err(NoiseError)` if there's an error within the Noise protocol state.
    pub fn prepare_handshake_message(&self) -> Option<Result<LpMessage, LpError>> {
        let mut noise_state = self.noise_state.lock();
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
    /// # Arguments
    ///
    /// * `noise_payload` - The raw bytes received from the peer, purported to be a Noise handshake message.
    ///
    /// # Returns
    ///
    /// * `Ok(ReadResult)` detailing the outcome (e.g., handshake complete, no-op).
    /// * `Err(NoiseError)` if the message is invalid or causes a Noise protocol error.
    pub fn process_handshake_message(&self, message: &LpMessage) -> Result<ReadResult, NoiseError> {
        let mut noise_state = self.noise_state.lock();

        match message {
            LpMessage::Handshake(HandshakeData(payload)) => {
                // The sans-io NoiseProtocol::read_message expects only the payload.
                noise_state.read_message(payload)
            }
            _ => Err(NoiseError::IncorrectStateError),
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
    fn generate_keypair() -> Keypair {
        let params: NoiseParams = NOISE_PATTERN.parse().unwrap();
        snow::Builder::new(params).generate_keypair().unwrap()
    }

    // Helper function to create a session with real keys for handshake tests
    fn create_handshake_test_session(
        is_initiator: bool,
        local_keys: &Keypair,
        remote_pub_key: &[u8],
        psk: &[u8],
    ) -> LpSession {
        // Use a dummy ID for testing, the important part is is_initiator
        let test_id = if is_initiator { 1 } else { 2 };
        LpSession::new(
            test_id,
            is_initiator,
            &local_keys.private,
            remote_pub_key,
            psk,
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
            create_handshake_test_session(true, &initiator_keys, &responder_keys.public, &psk);
        let responder_session = create_handshake_test_session(
            false,
            &responder_keys,
            &initiator_keys.public, // Responder also needs initiator's key for XK
            &psk,
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
            create_handshake_test_session(true, &initiator_keys, &responder_keys.public, &psk);
        let responder_session =
            create_handshake_test_session(false, &responder_keys, &initiator_keys.public, &psk);

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
            create_handshake_test_session(true, &initiator_keys, &responder_keys.public, &psk);
        let responder_session =
            create_handshake_test_session(false, &responder_keys, &initiator_keys.public, &psk);

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
            create_handshake_test_session(true, &initiator_keys, &responder_keys.public, &psk);
        let responder_session =
            create_handshake_test_session(false, &responder_keys, &initiator_keys.public, &psk);

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
            create_handshake_test_session(true, &initiator_keys, &responder_keys.public, &psk);

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
}
