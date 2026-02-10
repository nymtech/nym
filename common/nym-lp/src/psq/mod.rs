// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::LpHeader;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::{LpError, LpMessage, LpPacket};
use nym_kkt::ciphersuite::Ciphersuite;
use nym_lp_transport::traits::LpTransport;

mod helpers;
mod initiator;
mod responder;

pub struct PSQHandshakeState<'a, S> {
    /// The underlying connection established for the handshake
    connection: &'a mut S,

    /// Protocol version used for the exchange.
    /// either known implicitly through the directory (initiator)
    /// or established through client hello (responder)
    protocol_version: Option<u8>,

    /// Ciphersuite selected for the KKT/PSQ exchange
    ciphersuite: Ciphersuite,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: Option<LpRemotePeer>,

    /// Counter for outgoing packets
    sending_counter: u64,
}

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    pub fn new(connection: &'a mut S, ciphersuite: Ciphersuite, local_peer: LpLocalPeer) -> Self {
        PSQHandshakeState {
            connection,
            protocol_version: None,
            ciphersuite,
            local_peer,
            remote_peer: None,
            sending_counter: 0,
        }
    }

    #[must_use]
    pub fn with_protocol_version(mut self, protocol_version: u8) -> Self {
        self.protocol_version = Some(protocol_version);
        self
    }

    #[must_use]
    pub fn with_remote_peer(mut self, remote_peer: LpRemotePeer) -> Self {
        self.remote_peer = Some(remote_peer);
        self
    }

    fn protocol_version(&self) -> Result<u8, LpError> {
        self.protocol_version
            .ok_or_else(|| LpError::kkt_psq_handshake("unknown protocol version"))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::mock_peers;
    use nym_kkt::ciphersuite::{HashFunction, HashLength, KEM, SignatureScheme};
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, TimeboxedSpawnable};
    use tokio::join;

    #[tokio::test]
    async fn psq_handshake() -> anyhow::Result<()> {
        let conn_init = MockIOStream::default();
        let conn_resp = conn_init.try_get_remote_handle();

        // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
        // so they'd get 'static lifetime
        let conn_init = conn_init.leak();
        let conn_resp = conn_resp.leak();

        let ciphersuite = Ciphersuite::new(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            HashLength::Default,
        );

        let (init, resp) = mock_peers();
        let resp_remote = resp.as_remote();

        let handshake_init = PSQHandshakeState::new(conn_init, ciphersuite, init)
            .with_protocol_version(1)
            .with_remote_peer(resp_remote);
        let handshake_resp = PSQHandshakeState::new(conn_resp, ciphersuite, resp);

        let resp_fut = handshake_resp.complete_as_responder().spawn_timeboxed();
        let init_fut = handshake_init.psq_handshake_initiator().spawn_timeboxed();

        let (session_init, session_resp) = join!(init_fut, resp_fut);

        let session_init = session_init???;
        let session_resp = session_resp???;

        assert_eq!(session_init.id(), session_resp.id());
        assert_eq!(
            session_init.outer_aead_key().as_bytes(),
            session_resp.outer_aead_key().as_bytes()
        );
        assert_eq!(
            session_init.pq_shared_secret().as_bytes(),
            session_resp.pq_shared_secret().as_bytes()
        );

        Ok(())
    }
}

/*
#[test]
    fn test_prepare_handshake_message_initial_state() {
        let receiver_index = 12345u32;

        let initiator_session = create_handshake_test_session(receiver_index, true);
        let responder_session = create_handshake_test_session(
            receiver_index,
            false,
            // Responder also needs initiator's key for XK
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
        let receiver_index = 12345u32;

        let initiator_session = create_handshake_test_session(receiver_index, true);
        let responder_session = create_handshake_test_session(receiver_index, false);

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
        let initiator_session = create_handshake_test_session(12345u32, true);
        let responder_session = create_handshake_test_session(12345u32, false);

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

    // ====================================================================
    // PSQ Handshake Integration Tests
    // ====================================================================

    /// Test that PSQ runs during handshake and derives a PSK
    #[test]
    fn test_psq_handshake_runs_with_psk_injection() {
        let initiator_session = create_handshake_test_session(12345u32, true);
        let responder_session = create_handshake_test_session(12345u32, false);

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

     /// Test that PSQ actually derives a different PSK (not using dummy)
    #[test]
    fn test_psq_derived_psk_differs_from_dummy() {
        // Create sessions - they start with dummy PSK [0u8; 32]
        let initiator_session = create_handshake_test_session(12345u32, true);
        let responder_session = create_handshake_test_session(12345u32, false);

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
        let initiator_session = create_handshake_test_session(12345u32, true);
        let responder_session = create_handshake_test_session(12345u32, false);

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
        // Create sessions with explicit Ed25519 keys
        let initiator_session = create_handshake_test_session(12345u32, true);
        let responder_session = create_handshake_test_session(12345u32, false);

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

        let responder_session = create_handshake_test_session(12345u32, false);

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
        let (init, resp) = mock_peers();

        let mut bad_resp = resp.as_remote();
        let wrong_ed25519 = ed25519::KeyPair::from_secret([99u8; 32], 99); // Different key!
        bad_resp.ed25519_public = *wrong_ed25519.public_key();

        let mut bad_init = init.as_remote();
        bad_init.ed25519_public = *wrong_ed25519.public_key();

        // Create sessions with MISMATCHED Ed25519 keys
        // This simulates authentication failure
        let receiver_index: u32 = 55555;
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            receiver_index,
            true,
            init.clone(),
            bad_resp,
            &salt,
            version::CURRENT,
        )
        .unwrap();

        // Initialize KKT state for test
        initiator_session.set_kkt_completed_for_test(resp.x25519.public_key());

        let responder_session = LpSession::new(
            receiver_index,
            false,
            resp,
            bad_init,
            &salt,
            version::CURRENT,
        )
        .unwrap();
        // Initialize KKT state for test
        responder_session.set_kkt_completed_for_test(init.x25519.public_key());

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
        let (init, resp) = mock_peers();

        let mut bad_init = init.as_remote();
        let wrong_ed25519 = ed25519::KeyPair::from_secret([99u8; 32], 99); // Different key!
        bad_init.ed25519_public = *wrong_ed25519.public_key();

        let receiver_index: u32 = 66666;
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            receiver_index,
            true,
            init.clone(),
            resp.as_remote(),
            &salt,
            version::CURRENT,
        )
        .unwrap();
        // Initialize KKT state for test
        initiator_session.set_kkt_completed_for_test(resp.x25519.public_key());

        let responder_session = LpSession::new(
            receiver_index,
            false,
            resp,
            bad_init,
            &salt,
            version::CURRENT,
        )
        .unwrap();
        // Initialize KKT state for test
        responder_session.set_kkt_completed_for_test(init.x25519.public_key());

        // Initiator creates message with valid signature (signed with [1u8])
        let msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        // Responder tries to verify with wrong public key [99u8]
        // This should fail Ed25519 signature verification
        let result = responder_session.process_handshake_message(&msg);

        assert!(result.is_err(), "Expected signature verification to fail");

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
        let responder_session = create_handshake_test_session(12345u32, false);

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
        let initiator_session = create_handshake_test_session(12345u32, true);

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

    #[test]
    fn test_transport_fails_without_psk_injection() {
        // This test verifies the safety mechanism that prevents transport mode operations
        // from running with the dummy PSK if PSQ injection fails or is skipped.

        // Create session but don't complete handshake (no PSK injection will occur)
        let mut session = create_handshake_test_session(12345u32, true);

        // Verify session was created successfully
        assert!(!session.is_handshake_complete());

        // Attempt to encrypt data - should fail with PskNotInjected
        let plaintext = b"test data";
        let encrypt_result = session.encrypt_data(plaintext);

        assert!(
            encrypt_result.is_err(),
            "encrypt_data should fail without PSK injection"
        );
        match encrypt_result.unwrap_err() {
            NoiseError::PskNotInjected => {
                // Expected - this is the safety mechanism working
            }
            e => panic!("Expected PskNotInjected error, got: {:?}", e),
        }

        // Create a dummy encrypted message to test decrypt
        let dummy_ciphertext = LpMessage::EncryptedData(EncryptedDataPayload(vec![0u8; 48]));

        // Attempt to decrypt data - should also fail with PskNotInjected
        let decrypt_result = session.decrypt_data(&dummy_ciphertext);

        assert!(
            decrypt_result.is_err(),
            "decrypt_data should fail without PSK injection"
        );
        match decrypt_result.unwrap_err() {
            NoiseError::PskNotInjected => {
                // Expected - this is the safety mechanism working
            }
            e => panic!("Expected PskNotInjected error, got: {:?}", e),
        }
    }













    #[tokio::test]
    async fn test_send_receive_client_hello_message() {
        use nym_lp::message::ClientHelloData;
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut rng = rand::thread_rng();
        let ed25519 = ed25519::KeyPair::new(&mut rng);
        let x25519 = ed25519.to_x25519();

        let client_key = *x25519.public_key();
        let client_ed25519_key = *ed25519.public_key();

        let hello_data =
            ClientHelloData::new_with_fresh_salt(client_key, client_ed25519_key, timestamp);
        let expected_salt = hello_data.salt; // Clone salt before moving hello_data

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: 300,
                    counter: 30,
                },
                LpMessage::ClientHello(hello_data),
            );
            handler.send_lp_packet(packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 300);
        assert_eq!(received.header().counter, 30);
        match received.message() {
            LpMessage::ClientHello(data) => {
                assert_eq!(data.client_lp_public_key, client_key);
                assert_eq!(data.salt, expected_salt);
            }
            _ => panic!("Expected ClientHello message"),
        }
    }

    // ==================== receive_client_hello Tests ====================

    #[tokio::test]
    async fn test_receive_client_hello_valid() {
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create and send valid ClientHello
        // Create separate Ed25519 keypair and derive X25519 from it (like production code)
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        let client_ed25519_keypair = ed25519::KeyPair::new(&mut OsRng);
        let client_x25519_public = client_ed25519_keypair.public_key().to_x25519().unwrap();

        let hello_data = ClientHelloData::new_with_fresh_salt(
            client_x25519_public,
            *client_ed25519_keypair.public_key(),
            timestamp,
        );
        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                reserved: [0u8; 3],
                receiver_idx: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data.clone()),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet)
            .await
            .unwrap();

        // Handler should receive and parse it
        let result = server_task.await.unwrap();
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

        let (x25519_pubkey, ed25519_pubkey, salt) = result.unwrap();
        assert_eq!(x25519_pubkey.as_bytes(), &client_x25519_public.to_bytes());
        assert_eq!(
            ed25519_pubkey.to_bytes(),
            client_ed25519_keypair.public_key().to_bytes()
        );
        assert_eq!(salt, hello_data.salt);
    }

    #[tokio::test]
    async fn test_receive_client_hello_timestamp_too_old() {
        use std::time::{SystemTime, UNIX_EPOCH};
        use tokio::net::{TcpListener, TcpStream};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create ClientHello with old timestamp
        // Use proper separate Ed25519 and X25519 keys (like production code)
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        let client_ed25519_keypair = ed25519::KeyPair::new(&mut OsRng);
        let client_x25519_public = client_ed25519_keypair.public_key().to_x25519().unwrap();

        let mut hello_data = ClientHelloData::new_with_fresh_salt(
            client_x25519_public,
            *client_ed25519_keypair.public_key(),
            timestamp,
        );

        // Manually set timestamp to be very old (100 seconds ago)
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 100;
        hello_data.salt[..8].copy_from_slice(&old_timestamp.to_le_bytes());

        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                reserved: [0u8; 3],
                receiver_idx: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet)
            .await
            .unwrap();

        // Should fail with timestamp error
        let result = server_task.await.unwrap();
        assert!(result.is_err());
        // Note: Can't use unwrap_err() directly because PublicKey doesn't implement Debug
        // Just check that it failed
        match result {
            Err(e) => {
                let err_msg = format!("{}", e);
                assert!(
                    err_msg.contains("too old"),
                    "Expected 'too old' in error, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error but got success"),
        }
    }


    #[tokio::test]
    async fn test_send_receive_handshake_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handshake_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let expected_data = handshake_data.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: 100,
                    counter: 10,
                },
                LpMessage::Handshake(HandshakeData(handshake_data)),
            );
            handler.send_lp_packet(packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream)
            .await
            .unwrap();
        assert_eq!(received.header().receiver_idx, 100);
        assert_eq!(received.header().counter, 10);
        match received.message() {
            LpMessage::Handshake(data) => assert_eq!(data, &HandshakeData(expected_data)),
            _ => panic!("Expected Handshake message"),
        }
    }


 */
