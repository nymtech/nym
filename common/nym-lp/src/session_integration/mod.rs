#[cfg(test)]
mod tests {
    use crate::codec::{parse_lp_packet, serialize_lp_packet};
    use crate::keypair::PublicKey;
    use crate::make_lp_id;
    use crate::{
        message::LpMessage,
        packet::{LpHeader, LpPacket, TRAILER_LEN},
        session_manager::SessionManager,
        LpError,
    };
    use bytes::BytesMut;
    use nym_crypto::asymmetric::ed25519;

    // Function to create a test packet - similar to how it's done in codec.rs tests
    fn create_test_packet(
        protocol_version: u8,
        session_id: u32,
        counter: u64,
        message: LpMessage,
    ) -> LpPacket {
        // Create the header
        let header = LpHeader {
            protocol_version,
            reserved: 0u16, // reserved
            session_id,
            counter,
        };

        // Create the trailer (zeros for now, in a real implementation this might be a MAC)
        let trailer = [0u8; TRAILER_LEN];

        // Create and return the packet directly
        LpPacket {
            header,
            message,
            trailer,
        }
    }

    /// Tests the complete session flow including:
    /// - Creation of sessions through session manager
    /// - Packet encoding/decoding with the session
    /// - Replay protection across the session
    /// - Multiple sessions with unique indices
    /// - Session removal and cleanup
    #[test]
    fn test_full_session_flow() {
        // 1. Initialize session manager
        let session_manager_1 = SessionManager::new();
        let session_manager_2 = SessionManager::new();

        // 2. Generate Ed25519 keypairs for PSQ authentication
        let ed25519_keypair_a = ed25519::KeyPair::from_secret([1u8; 32], 0);
        let ed25519_keypair_b = ed25519::KeyPair::from_secret([2u8; 32], 1);

        // Derive X25519 keys from Ed25519 (same as state machine does internally)
        let x25519_pub_a = ed25519_keypair_a
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");
        let x25519_pub_b = ed25519_keypair_b
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");

        // Convert to LP keypair types
        let lp_pub_a = PublicKey::from_bytes(x25519_pub_a.as_bytes())
            .expect("Failed to create PublicKey from bytes");
        let lp_pub_b = PublicKey::from_bytes(x25519_pub_b.as_bytes())
            .expect("Failed to create PublicKey from bytes");

        // Calculate lp_id (matches state machine's internal calculation)
        let lp_id = make_lp_id(&lp_pub_a, &lp_pub_b);

        // Test salt
        let salt = [42u8; 32];

        // 4. Create sessions using the pre-built Noise states
        let peer_a_sm = session_manager_1
            .create_session_state_machine(
                (
                    ed25519_keypair_a.private_key(),
                    ed25519_keypair_a.public_key(),
                ),
                ed25519_keypair_b.public_key(),
                true,
                &salt,
            )
            .expect("Failed to create session A");

        let peer_b_sm = session_manager_2
            .create_session_state_machine(
                (
                    ed25519_keypair_b.private_key(),
                    ed25519_keypair_b.public_key(),
                ),
                ed25519_keypair_a.public_key(),
                false,
                &salt,
            )
            .expect("Failed to create session B");

        // Verify session count
        assert_eq!(session_manager_1.session_count(), 1);
        assert_eq!(session_manager_2.session_count(), 1);

        // Initialize KKT state for both sessions (test bypass)
        session_manager_1
            .init_kkt_for_test(peer_a_sm, &lp_pub_b)
            .expect("Failed to init KKT for peer A");
        session_manager_2
            .init_kkt_for_test(peer_b_sm, &lp_pub_a)
            .expect("Failed to init KKT for peer B");

        // 5. Simulate Noise Handshake (Sans-IO)
        println!("Starting handshake simulation...");
        let mut i_msg_payload;
        let mut r_msg_payload = None;
        let mut rounds = 0;
        const MAX_ROUNDS: usize = 10;

        // Prime initiator's first message
        i_msg_payload = session_manager_1
            .prepare_handshake_message(peer_a_sm)
            .transpose()
            .unwrap();

        assert!(
            i_msg_payload.is_some(),
            "Initiator did not produce initial message"
        );

        while rounds < MAX_ROUNDS {
            rounds += 1;
            let mut did_exchange = false;

            // === Initiator -> Responder ===
            if let Some(payload) = i_msg_payload.take() {
                did_exchange = true;
                println!(
                    "  Round {}: Initiator -> Responder ({} bytes)",
                    rounds,
                    payload.len()
                );

                // A prepares packet
                let counter = session_manager_1.next_counter(lp_id).unwrap();
                let message_a_to_b = create_test_packet(1, lp_id, counter, payload);
                let mut encoded_msg = BytesMut::new();
                serialize_lp_packet(&message_a_to_b, &mut encoded_msg).expect("A serialize failed");

                // B parses packet and checks replay
                let decoded_packet = parse_lp_packet(&encoded_msg).expect("B parse failed");
                assert_eq!(decoded_packet.header.counter, counter);

                // Check replay before processing handshake
                session_manager_2
                    .receiving_counter_quick_check(peer_b_sm, decoded_packet.header.counter)
                    .expect("B replay check failed (A->B)");

                match session_manager_2
                    .process_handshake_message(peer_b_sm, &decoded_packet.message)
                {
                    Ok(_) => {
                        // Mark counter only after successful processing
                        session_manager_2
                            .receiving_counter_mark(peer_b_sm, decoded_packet.header.counter)
                            .expect("B mark counter failed");
                    }
                    Err(e) => panic!("Responder processing failed: {:?}", e),
                }
                // Check if responder needs to send a reply
                r_msg_payload = session_manager_2
                    .prepare_handshake_message(peer_b_sm)
                    .transpose()
                    .unwrap();
                println!("{:?}", r_msg_payload);
            }

            // Check completion
            if session_manager_1.is_handshake_complete(peer_a_sm).unwrap()
                && session_manager_2.is_handshake_complete(peer_b_sm).unwrap()
            {
                println!("Handshake completed after Initiator->Responder message.");
                break;
            }

            // === Responder -> Initiator ===
            if let Some(payload) = r_msg_payload.take() {
                did_exchange = true;
                println!(
                    "  Round {}: Responder -> Initiator ({} bytes)",
                    rounds,
                    payload.len()
                );

                // B prepares packet
                let counter = session_manager_2.next_counter(peer_b_sm).unwrap();
                let message_b_to_a = create_test_packet(1, lp_id, counter, payload);
                let mut encoded_msg = BytesMut::new();
                serialize_lp_packet(&message_b_to_a, &mut encoded_msg).expect("B serialize failed");

                // A parses packet and checks replay
                let decoded_packet = parse_lp_packet(&encoded_msg).expect("A parse failed");
                assert_eq!(decoded_packet.header.counter, counter);

                // Check replay before processing handshake
                session_manager_1
                    .receiving_counter_quick_check(peer_a_sm, decoded_packet.header.counter)
                    .expect("A replay check failed (B->A)");

                match session_manager_1
                    .process_handshake_message(peer_a_sm, &decoded_packet.message)
                {
                    Ok(_) => {
                        // Mark counter only after successful processing
                        session_manager_1
                            .receiving_counter_mark(peer_a_sm, decoded_packet.header.counter)
                            .expect("A mark counter failed");
                    }
                    Err(e) => panic!("Initiator processing failed: {:?}", e),
                }

                // Check if initiator needs to send a reply
                i_msg_payload = session_manager_1
                    .prepare_handshake_message(peer_a_sm)
                    .transpose()
                    .unwrap();
            }

            // println!("Initiator state: {}", session_manager_1.get_state(peer_a_sm).unwrap());
            // println!("Responder state: {}", session_manager_2.get_state(peer_b_sm).unwrap());

            println!(
                "Initiator state: {}",
                session_manager_1.is_handshake_complete(peer_a_sm).unwrap()
            );
            println!(
                "Responder state: {}",
                session_manager_2.is_handshake_complete(peer_b_sm).unwrap()
            );

            // Check completion again
            if session_manager_1.is_handshake_complete(peer_a_sm).unwrap()
                && session_manager_2.is_handshake_complete(peer_b_sm).unwrap()
            {
                println!("Handshake completed after Responder->Initiator message.");

                // Safety break if no messages were exchanged in a round
                if !did_exchange {
                    println!("No messages exchanged in round {}, breaking.", rounds);
                    break;
                }
            }

            assert!(rounds < MAX_ROUNDS, "Handshake loop exceeded max rounds");
        }
        assert!(
            session_manager_1.is_handshake_complete(peer_a_sm).unwrap(),
            "Initiator handshake did not complete"
        );
        assert!(
            session_manager_2.is_handshake_complete(peer_b_sm).unwrap(),
            "Responder handshake did not complete"
        );
        println!(
            "Handshake simulation completed successfully in {} rounds.",
            rounds
        );

        // --- Handshake Complete ---

        // 7. Simulate Data Transfer (Post-Handshake)
        println!("Starting data transfer simulation...");
        let plaintext_a_to_b = b"Hello from A!";

        // A encrypts data
        let ciphertext_a_to_b = session_manager_1
            .encrypt_data(peer_a_sm, plaintext_a_to_b)
            .expect("A encrypt failed");

        // A prepares packet
        let counter_a = session_manager_1.next_counter(peer_a_sm).unwrap();
        let message_a_to_b = create_test_packet(1, lp_id, counter_a, ciphertext_a_to_b);
        let mut encoded_data_a_to_b = BytesMut::new();
        serialize_lp_packet(&message_a_to_b, &mut encoded_data_a_to_b)
            .expect("A serialize data failed");

        // B parses packet and checks replay
        let decoded_packet_b = parse_lp_packet(&encoded_data_a_to_b).expect("B parse data failed");
        assert_eq!(decoded_packet_b.header.counter, counter_a);

        // Check replay before decrypting
        session_manager_2
            .receiving_counter_quick_check(peer_b_sm, decoded_packet_b.header.counter)
            .expect("B data replay check failed (A->B)");

        // B decrypts data
        let decrypted_payload = session_manager_2
            .decrypt_data(peer_b_sm, &decoded_packet_b.message)
            .expect("B decrypt failed");
        assert_eq!(decrypted_payload, plaintext_a_to_b);
        // Mark counter only after successful decryption
        session_manager_2
            .receiving_counter_mark(peer_b_sm, decoded_packet_b.header.counter)
            .expect("B mark data counter failed");
        println!(
            "  A->B: Decrypted successfully: {:?}",
            String::from_utf8_lossy(&decrypted_payload)
        );

        // B sends data to A
        let plaintext_b_to_a = b"Hello from B!";
        let ciphertext_b_to_a = session_manager_2
            .encrypt_data(peer_b_sm, plaintext_b_to_a)
            .expect("B encrypt failed");
        let counter_b = session_manager_2.next_counter(peer_b_sm).unwrap();
        let message_b_to_a = create_test_packet(1, lp_id, counter_b, ciphertext_b_to_a);
        let mut encoded_data_b_to_a = BytesMut::new();
        serialize_lp_packet(&message_b_to_a, &mut encoded_data_b_to_a)
            .expect("B serialize data failed");

        // A parses packet and checks replay
        let decoded_packet_a = parse_lp_packet(&encoded_data_b_to_a).expect("A parse data failed");
        assert_eq!(decoded_packet_a.header.counter, counter_b);

        // Check replay before decrypting
        session_manager_1
            .receiving_counter_quick_check(peer_a_sm, decoded_packet_a.header.counter)
            .expect("A data replay check failed (B->A)");

        // A decrypts data
        let decrypted_payload = session_manager_1
            .decrypt_data(peer_a_sm, &decoded_packet_a.message)
            .expect("A decrypt failed");
        assert_eq!(decrypted_payload, plaintext_b_to_a);
        // Mark counter only after successful decryption
        session_manager_1
            .receiving_counter_mark(peer_a_sm, decoded_packet_a.header.counter)
            .expect("A mark data counter failed");
        println!(
            "  B->A: Decrypted successfully: {:?}",
            String::from_utf8_lossy(&decrypted_payload)
        );

        println!("Data transfer simulation completed.");

        // 8. Replay Protection Test (Data Packet)
        println!("Testing data packet replay protection...");
        // Try to replay the last message from B to A
        // Need to re-encode because decode consumes the buffer
        let message_b_to_a_replay = create_test_packet(
            1,
            lp_id,
            counter_b,
            LpMessage::EncryptedData(crate::message::EncryptedDataPayload(
                plaintext_b_to_a.to_vec(),
            )), // Using plaintext here, but content doesn't matter for replay check
        );
        let mut encoded_data_b_to_a_replay = BytesMut::new();
        serialize_lp_packet(&message_b_to_a_replay, &mut encoded_data_b_to_a_replay)
            .expect("B serialize replay failed");

        let parsed_replay_packet =
            parse_lp_packet(&encoded_data_b_to_a_replay).expect("A parse replay failed");
        let replay_result = session_manager_1
            .receiving_counter_quick_check(peer_a_sm, parsed_replay_packet.header.counter);
        assert!(replay_result.is_err(), "Data replay should be prevented");
        assert!(
            matches!(replay_result.unwrap_err(), LpError::Replay(_)),
            "Should be a replay protection error for data packet"
        );
        println!("Data packet replay protection test passed.");

        // 9. Test out-of-order packet reception (send counter N+1 before counter N)
        println!("Testing out-of-order data packet reception...");
        let counter_a_next = session_manager_1.next_counter(peer_a_sm).unwrap(); // Should be counter_a + 1
        let counter_a_skip = session_manager_1.next_counter(peer_a_sm).unwrap(); // Should be counter_a + 2

        // Prepare data for counter_a_skip (N+1)
        let plaintext_skip = b"Out of order message";
        let ciphertext_skip = session_manager_1
            .encrypt_data(peer_a_sm, plaintext_skip)
            .expect("A encrypt skip failed");

        let message_a_to_b_skip = create_test_packet(
            1, // protocol version
            lp_id,
            counter_a_skip, // Send N+1 first
            ciphertext_skip,
        );

        // Encode the skip message
        let mut encoded_skip = BytesMut::new();
        serialize_lp_packet(&message_a_to_b_skip, &mut encoded_skip)
            .expect("Failed to serialize skip message");

        // B parses skip message and checks replay
        let decoded_packet_skip = parse_lp_packet(&encoded_skip).expect("B parse skip failed");
        session_manager_2
            .receiving_counter_quick_check(peer_b_sm, decoded_packet_skip.header.counter)
            .expect("B replay check skip failed");
        assert_eq!(decoded_packet_skip.header.counter, counter_a_skip);

        // B decrypts skip message
        let decrypted_payload = session_manager_2
            .decrypt_data(peer_b_sm, &decoded_packet_skip.message)
            .expect("B decrypt skip failed");
        assert_eq!(decrypted_payload, plaintext_skip);
        // Mark counter N+1
        session_manager_2
            .receiving_counter_mark(peer_b_sm, decoded_packet_skip.header.counter)
            .expect("B mark skip counter failed");
        println!(
            "  A->B (Counter {}): Decrypted successfully: {:?}",
            counter_a_skip,
            String::from_utf8_lossy(&decrypted_payload)
        );

        // 10. Now send the skipped counter N message (should still work)
        println!("Testing delayed data packet reception...");
        // Prepare data for counter_a_next (N)
        let plaintext_delayed = b"Delayed message";
        let ciphertext_delayed = session_manager_1
            .encrypt_data(peer_a_sm, plaintext_delayed)
            .expect("A encrypt delayed failed");

        let message_a_to_b_delayed = create_test_packet(
            1, // protocol version
            lp_id,
            counter_a_next, // counter N (delayed packet)
            ciphertext_delayed,
        );

        // Encode the delayed message
        let mut encoded_delayed = BytesMut::new();
        serialize_lp_packet(&message_a_to_b_delayed, &mut encoded_delayed)
            .expect("Failed to serialize delayed message");

        // Make a copy for replay test later
        let encoded_delayed_copy = encoded_delayed.clone();

        // B parses delayed message and checks replay
        let decoded_packet_delayed =
            parse_lp_packet(&encoded_delayed).expect("B parse delayed failed");
        session_manager_2
            .receiving_counter_quick_check(peer_b_sm, decoded_packet_delayed.header.counter)
            .expect("B replay check delayed failed");
        assert_eq!(decoded_packet_delayed.header.counter, counter_a_next);

        // B decrypts delayed message
        let decrypted_payload = session_manager_2
            .decrypt_data(peer_b_sm, &decoded_packet_delayed.message)
            .expect("B decrypt delayed failed");
        assert_eq!(decrypted_payload, plaintext_delayed);
        // Mark counter N
        session_manager_2
            .receiving_counter_mark(peer_b_sm, decoded_packet_delayed.header.counter)
            .expect("B mark delayed counter failed");
        println!(
            "  A->B (Counter {}): Decrypted successfully: {:?}",
            counter_a_next,
            String::from_utf8_lossy(&decrypted_payload)
        );

        println!("Delayed data packet reception test passed.");

        // 11. Try to replay message with counter N (should fail)
        println!("Testing replay of delayed packet...");
        let parsed_delayed_replay =
            parse_lp_packet(&encoded_delayed_copy).expect("Parse delayed replay failed");
        let result = session_manager_2
            .receiving_counter_quick_check(peer_b_sm, parsed_delayed_replay.header.counter);
        assert!(result.is_err(), "Replay attack should be prevented");
        assert!(
            matches!(result, Err(LpError::Replay(_))),
            "Should be a replay protection error"
        );

        // 12. Session removal
        assert!(session_manager_1.remove_state_machine(lp_id));
        assert_eq!(session_manager_1.session_count(), 0);

        // Verify the session is gone
        let session = session_manager_1.state_machine_exists(lp_id);
        assert!(!session, "Session should be removed");

        // But the other session still exists
        let session = session_manager_2.state_machine_exists(lp_id);
        assert!(session, "Session still exists in the other manager");
    }

    /// Tests simultaneous bidirectional communication between sessions
    #[test]
    fn test_bidirectional_communication() {
        // 1. Initialize session manager
        let session_manager_1 = SessionManager::new();
        let session_manager_2 = SessionManager::new();

        // 2. Generate Ed25519 keypairs for PSQ authentication
        let ed25519_keypair_a = ed25519::KeyPair::from_secret([3u8; 32], 0);
        let ed25519_keypair_b = ed25519::KeyPair::from_secret([4u8; 32], 1);

        // Derive X25519 keys from Ed25519 (same as state machine does internally)
        let x25519_pub_a = ed25519_keypair_a
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");
        let x25519_pub_b = ed25519_keypair_b
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");

        // Convert to LP keypair types
        let lp_pub_a = PublicKey::from_bytes(x25519_pub_a.as_bytes())
            .expect("Failed to create PublicKey from bytes");
        let lp_pub_b = PublicKey::from_bytes(x25519_pub_b.as_bytes())
            .expect("Failed to create PublicKey from bytes");

        // Calculate lp_id (matches state machine's internal calculation)
        let lp_id = make_lp_id(&lp_pub_a, &lp_pub_b);

        // Test salt
        let salt = [43u8; 32];

        let peer_a_sm = session_manager_1
            .create_session_state_machine(
                (
                    ed25519_keypair_a.private_key(),
                    ed25519_keypair_a.public_key(),
                ),
                ed25519_keypair_b.public_key(),
                true,
                &salt,
            )
            .unwrap();
        let peer_b_sm = session_manager_2
            .create_session_state_machine(
                (
                    ed25519_keypair_b.private_key(),
                    ed25519_keypair_b.public_key(),
                ),
                ed25519_keypair_a.public_key(),
                false,
                &salt,
            )
            .unwrap();

        // Initialize KKT state for both sessions (test bypass)
        session_manager_1
            .init_kkt_for_test(peer_a_sm, &lp_pub_b)
            .expect("Failed to init KKT for peer A");
        session_manager_2
            .init_kkt_for_test(peer_b_sm, &lp_pub_a)
            .expect("Failed to init KKT for peer B");

        // Drive handshake to completion (simplified)
        let mut i_msg = session_manager_1
            .prepare_handshake_message(peer_a_sm)
            .transpose()
            .unwrap()
            .unwrap();

        session_manager_2
            .process_handshake_message(peer_b_sm, &i_msg)
            .unwrap();
        session_manager_2
            .receiving_counter_mark(peer_b_sm, 0)
            .unwrap(); // Assume counter 0 for first msg
        let r_msg = session_manager_2
            .prepare_handshake_message(peer_b_sm)
            .transpose()
            .unwrap()
            .unwrap();
        session_manager_1
            .process_handshake_message(peer_a_sm, &r_msg)
            .unwrap();
        session_manager_1
            .receiving_counter_mark(peer_a_sm, 0)
            .unwrap(); // Assume counter 0 for first msg
        i_msg = session_manager_1
            .prepare_handshake_message(peer_a_sm)
            .transpose()
            .unwrap()
            .unwrap();

        session_manager_2
            .process_handshake_message(peer_b_sm, &i_msg)
            .unwrap();
        session_manager_2
            .receiving_counter_mark(peer_b_sm, 1)
            .unwrap(); // Assume counter 1 for second msg from A

        assert!(session_manager_1.is_handshake_complete(peer_a_sm).unwrap());
        assert!(session_manager_2.is_handshake_complete(peer_b_sm).unwrap());
        println!("Bidirectional test: Handshake complete.");

        // Counters after handshake (A sent 2, B sent 1)
        let mut counter_a = 2; // Next counter for A to send
        let mut counter_b = 1; // Next counter for B to send

        // 4. Send multiple encrypted messages both ways
        const NUM_MESSAGES: u64 = 5;
        for i in 0..NUM_MESSAGES {
            println!("Bidirectional test: Round {}", i);
            // --- A sends to B ---
            let plaintext_a = format!("A->B Message {}", i).into_bytes();
            let ciphertext_a = session_manager_1
                .encrypt_data(peer_a_sm, &plaintext_a)
                .expect("A encrypt failed");
            let current_counter_a = counter_a;
            counter_a += 1;

            let message_a = create_test_packet(1, lp_id, current_counter_a, ciphertext_a);
            let mut encoded_a = BytesMut::new();
            serialize_lp_packet(&message_a, &mut encoded_a).expect("A serialize failed");

            // B parses and checks replay
            let decoded_packet_b = parse_lp_packet(&encoded_a).expect("B parse failed");
            session_manager_2
                .receiving_counter_quick_check(peer_b_sm, decoded_packet_b.header.counter)
                .expect("B replay check failed (A->B)");
            assert_eq!(decoded_packet_b.header.counter, current_counter_a);
            let decrypted_payload = session_manager_2
                .decrypt_data(peer_b_sm, &decoded_packet_b.message)
                .expect("B decrypt failed");
            assert_eq!(decrypted_payload, plaintext_a);
            session_manager_2
                .receiving_counter_mark(peer_b_sm, current_counter_a)
                .expect("B mark counter failed");

            // --- B sends to A ---
            let plaintext_b = format!("B->A Message {}", i).into_bytes();
            let ciphertext_b = session_manager_2
                .encrypt_data(peer_b_sm, &plaintext_b)
                .expect("B encrypt failed");
            let current_counter_b = counter_b;
            counter_b += 1;

            let message_b = create_test_packet(1, lp_id, current_counter_b, ciphertext_b);
            let mut encoded_b = BytesMut::new();
            serialize_lp_packet(&message_b, &mut encoded_b).expect("B serialize failed");

            // A parses and checks replay
            let decoded_packet_a = parse_lp_packet(&encoded_b).expect("A parse failed");
            session_manager_1
                .receiving_counter_quick_check(peer_a_sm, decoded_packet_a.header.counter)
                .expect("A replay check failed (B->A)");
            assert_eq!(decoded_packet_a.header.counter, current_counter_b);
            let decrypted_payload = session_manager_1
                .decrypt_data(peer_a_sm, &decoded_packet_a.message)
                .expect("A decrypt failed");
            assert_eq!(decrypted_payload, plaintext_b);
            session_manager_1
                .receiving_counter_mark(peer_a_sm, current_counter_b)
                .expect("A mark counter failed");
        }

        // 5. Verify counter stats
        // Note: current_packet_cnt() returns (next_expected_receive_counter, total_received)
        let (next_recv_a, total_recv_a) = session_manager_1.current_packet_cnt(peer_a_sm).unwrap();
        let (next_recv_b, total_recv_b) = session_manager_2.current_packet_cnt(peer_b_sm).unwrap();

        // Peer A sent handshake(0), handshake(1) + 5 data packets = 7 total. Next send counter = 7.
        // Peer A received handshake(0) + 5 data packets = 6 total. Next expected recv counter = 6.
        assert_eq!(
            counter_a,
            2 + NUM_MESSAGES,
            "Peer A final send counter mismatch"
        );
        assert_eq!(
            total_recv_a,
            1 + NUM_MESSAGES,
            "Peer A total received count mismatch"
        ); // Received 1 handshake + 5 data
        assert_eq!(
            next_recv_a,
            1 + NUM_MESSAGES,
            "Peer A next expected receive counter mismatch"
        ); // Expected counter for msg from B

        // Peer B sent handshake(0) + 5 data packets = 6 total. Next send counter = 6.
        // Peer B received handshake(0), handshake(1) + 5 data packets = 7 total. Next expected recv counter = 7.
        assert_eq!(
            counter_b,
            1 + NUM_MESSAGES,
            "Peer B final send counter mismatch"
        );
        assert_eq!(
            total_recv_b,
            2 + NUM_MESSAGES,
            "Peer B total received count mismatch"
        ); // Received 2 handshake + 5 data
        assert_eq!(
            next_recv_b,
            2 + NUM_MESSAGES,
            "Peer B next expected receive counter mismatch"
        ); // Expected counter for msg from A

        println!("Bidirectional test completed.");
    }

    /// Tests error handling in session flow
    #[test]
    fn test_session_error_handling() {
        // 1. Initialize session manager
        let session_manager = SessionManager::new();

        // Generate Ed25519 keypair for PSQ authentication
        let ed25519_keypair = ed25519::KeyPair::from_secret([5u8; 32], 0);

        // Derive X25519 key from Ed25519 (same as state machine does internally)
        let x25519_pub = ed25519_keypair
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");

        // Convert to LP keypair type
        let lp_pub = PublicKey::from_bytes(x25519_pub.as_bytes())
            .expect("Failed to create PublicKey from bytes");

        // Calculate lp_id (self-connection: both sides use same key)
        let lp_id = make_lp_id(&lp_pub, &lp_pub);

        // Test salt
        let salt = [44u8; 32];

        // 2. Create a session (using real noise state)
        let _session = session_manager
            .create_session_state_machine(
                (ed25519_keypair.private_key(), ed25519_keypair.public_key()),
                ed25519_keypair.public_key(),
                true,
                &salt,
            )
            .expect("Failed to create session");

        // 3. Try to get a non-existent session
        let result = session_manager.state_machine_exists(999);
        assert!(!result, "Non-existent session should return None");

        // 4. Try to remove a non-existent session
        let result = session_manager.remove_state_machine(999);
        assert!(
            !result,
            "Remove session should not remove a non-existent session"
        );

        // 5. Create and immediately remove a session
        let _temp_session = session_manager
            .create_session_state_machine(
                (ed25519_keypair.private_key(), ed25519_keypair.public_key()),
                ed25519_keypair.public_key(),
                true,
                &salt,
            )
            .expect("Failed to create temp session");

        assert!(
            session_manager.remove_state_machine(lp_id),
            "Should remove the session"
        );

        // 6. Create a codec and test error cases
        // let mut codec = LPCodec::new(session);

        // 7. Create an invalid message type packet
        let mut buf = BytesMut::new();

        // Add header
        buf.extend_from_slice(&[1, 0, 0, 0]); // Version + reserved
        buf.extend_from_slice(&lp_id.to_le_bytes()); // Sender index
        buf.extend_from_slice(&0u64.to_le_bytes()); // Counter

        // Add invalid message type
        buf.extend_from_slice(&0xFFFFu16.to_le_bytes());

        // Add some dummy data
        buf.extend_from_slice(&[0u8; 80]);

        // Add trailer
        buf.extend_from_slice(&[0u8; TRAILER_LEN]);

        // Try to parse the invalid message type
        let result = parse_lp_packet(&buf);
        assert!(result.is_err(), "Decoding invalid message type should fail");

        // Add assertion for the specific error type
        assert!(matches!(
            result.unwrap_err(),
            LpError::InvalidMessageType(0xFFFF)
        ));

        // 8. Test partial packet decoding
        let partial_packet = &buf[0..10]; // Too short to be a valid packet
        let partial_bytes = BytesMut::from(partial_packet);

        let result = parse_lp_packet(&partial_bytes);
        assert!(result.is_err(), "Parsing partial packet should fail");
        assert!(matches!(
            result.unwrap_err(),
            LpError::InsufficientBufferSize
        ));
    }
    // Remove unused imports if SessionManager methods are no longer direct dependencies
    // use crate::noise_protocol::{create_noise_state, create_noise_state_responder};
    use crate::{
        // Bring in state machine types
        state_machine::{LpAction, LpInput, LpStateBare},
        // message::LpMessage, // LpMessage likely still needed for LpInput/LpAction
        // packet::{LpHeader, LpPacket, TRAILER_LEN}, // LpPacket needed for LpAction/LpInput
    };
    use bytes::Bytes; // Use Bytes for SendData input

    // Keep helper function for creating test packets if needed,
    // but LpAction::SendPacket should provide the packets now.
    // fn create_test_packet(...) -> LpPacket { ... }

    /// Tests the complete session flow using ONLY the process_input interface:
    /// - Creation of sessions through session manager
    /// - Handshake driven by StartHandshake, ReceivePacket inputs
    /// - Data transfer driven by SendData, ReceivePacket inputs
    /// - Actions like SendPacket, DeliverData handled from output
    /// - Implicit replay protection via state machine logic
    /// - Closing driven by Close input
    #[test]
    fn test_full_session_flow_with_process_input() {
        // 1. Initialize session managers
        let session_manager_1 = SessionManager::new();
        let session_manager_2 = SessionManager::new();

        // 2. Generate Ed25519 keypairs for PSQ authentication
        let ed25519_keypair_a = ed25519::KeyPair::from_secret([6u8; 32], 0);
        let ed25519_keypair_b = ed25519::KeyPair::from_secret([7u8; 32], 1);

        // Derive X25519 keys from Ed25519 (same as state machine does internally)
        let x25519_pub_a = ed25519_keypair_a
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");
        let x25519_pub_b = ed25519_keypair_b
            .public_key()
            .to_x25519()
            .expect("Failed to derive X25519 from Ed25519");

        // Convert to LP keypair types
        let lp_pub_a = PublicKey::from_bytes(x25519_pub_a.as_bytes())
            .expect("Failed to create PublicKey from bytes");
        let lp_pub_b = PublicKey::from_bytes(x25519_pub_b.as_bytes())
            .expect("Failed to create PublicKey from bytes");

        // Calculate lp_id (matches state machine's internal calculation)
        let lp_id = make_lp_id(&lp_pub_a, &lp_pub_b);

        // Test salt
        let salt = [45u8; 32];

        // 3. Create sessions state machines
        assert!(session_manager_1
            .create_session_state_machine(
                (
                    ed25519_keypair_a.private_key(),
                    ed25519_keypair_a.public_key()
                ),
                ed25519_keypair_b.public_key(),
                true,
                &salt,
            ) // Initiator
            .is_ok());
        assert!(session_manager_2
            .create_session_state_machine(
                (
                    ed25519_keypair_b.private_key(),
                    ed25519_keypair_b.public_key()
                ),
                ed25519_keypair_a.public_key(),
                false,
                &salt,
            ) // Responder
            .is_ok());

        assert_eq!(session_manager_1.session_count(), 1);
        assert_eq!(session_manager_2.session_count(), 1);
        assert!(session_manager_1.state_machine_exists(lp_id));
        assert!(session_manager_2.state_machine_exists(lp_id));

        // Verify initial states are ReadyToHandshake
        assert_eq!(
            session_manager_1.get_state(lp_id).unwrap(),
            LpStateBare::ReadyToHandshake
        );
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::ReadyToHandshake
        );

        // --- 4. Simulate Noise Handshake via process_input ---
        println!("Starting handshake simulation via process_input...");

        let mut packet_a_to_b: Option<LpPacket>;
        let mut packet_b_to_a: Option<LpPacket>;
        let mut rounds = 0;
        const MAX_ROUNDS: usize = 10; // KKT (2 messages) + XK handshake (3 messages) + PSQ = 6 rounds total

        // --- Round 1: Initiator Starts ---
        println!("  Round {}: Initiator starts handshake", rounds);
        let action_a1 = session_manager_1
            .process_input(lp_id, LpInput::StartHandshake)
            .expect("Initiator StartHandshake should produce an action")
            .expect("Initiator StartHandshake failed");

        if let LpAction::SendPacket(packet) = action_a1 {
            println!("    Initiator produced SendPacket (KKT request)");
            packet_a_to_b = Some(packet);
        } else {
            panic!("Initiator StartHandshake did not produce SendPacket");
        }
        // After StartHandshake, initiator should be in KKTExchange state (not Handshaking yet)
        assert_eq!(
            session_manager_1.get_state(lp_id).unwrap(),
            LpStateBare::KKTExchange,
            "Initiator state wrong after StartHandshake (should be KKTExchange)"
        );

        // *** ADD THIS BLOCK for Responder StartHandshake ***
        println!(
            "  Round {}: Responder explicitly enters KKTExchange state",
            rounds
        );
        let action_b_start = session_manager_2.process_input(lp_id, LpInput::StartHandshake);
        // Responder's StartHandshake should not produce an action to send
        assert!(
            action_b_start.as_ref().unwrap().is_none(),
            "Responder StartHandshake should produce None action, got {:?}",
            action_b_start
        );
        // Verify responder transitions to KKTExchange state (not Handshaking yet)
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::KKTExchange, // Responder also enters KKTExchange state
            "Responder state should be KKTExchange after its StartHandshake"
        );
        // *** END OF ADDED BLOCK ***

        // --- Round 2: Responder Receives KKT Request, Sends KKT Response ---
        rounds += 1;
        println!(
            "  Round {}: Responder receives KKT request, sends KKT response",
            rounds
        );
        let packet_to_process = packet_a_to_b
            .take()
            .expect("KKT request from A was missing");

        // Simulate network: serialize -> parse (optional but good practice)
        let mut buf_a = BytesMut::new();
        serialize_lp_packet(&packet_to_process, &mut buf_a).unwrap();
        let parsed_packet_a = parse_lp_packet(&buf_a).unwrap();

        // Responder processes KKT request
        let action_b1 = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(parsed_packet_a))
            .expect("Responder ReceivePacket should produce an action")
            .expect("Responder ReceivePacket failed");

        if let LpAction::SendPacket(packet) = action_b1 {
            println!("    Responder received KKT request, produced KKT response");
            packet_b_to_a = Some(packet);
        } else {
            panic!("Responder ReceivePacket did not produce SendPacket for KKT response");
        }
        // Responder transitions to Handshaking after KKT completes
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::Handshaking,
            "Responder state should be Handshaking after KKT exchange"
        );

        // --- Round 3: Initiator Receives KKT Response, Sends First Noise Message (with PSQ) ---
        rounds += 1;
        println!(
            "  Round {}: Initiator receives KKT response, sends first Noise message (with PSQ)",
            rounds
        );
        let packet_to_process = packet_b_to_a
            .take()
            .expect("KKT response from B was missing");

        // Simulate network
        let mut buf_b = BytesMut::new();
        serialize_lp_packet(&packet_to_process, &mut buf_b).unwrap();
        let parsed_packet_b = parse_lp_packet(&buf_b).unwrap();

        // Initiator processes KKT response
        let action_a2 = session_manager_1
            .process_input(lp_id, LpInput::ReceivePacket(parsed_packet_b))
            .expect("Initiator ReceivePacket should produce an action")
            .expect("Initiator ReceivePacket failed");

        match action_a2 {
            LpAction::SendPacket(packet) => {
                println!(
                    "    Initiator received KKT response, produced first Noise message (-> e)"
                );
                packet_a_to_b = Some(packet);
                // Initiator transitions to Handshaking after KKT completes
                assert_eq!(
                    session_manager_1.get_state(lp_id).unwrap(),
                    LpStateBare::Handshaking,
                    "Initiator state should be Handshaking after receiving KKT response"
                );
            }
            LpAction::KKTComplete => {
                println!("    Initiator received KKT response, produced KKTComplete (will send Noise in next step)");
                // KKT completed, now need to explicitly trigger handshake message
                // This might be the case if KKT completion doesn't automatically send the first Noise message
                // Let's try to prepare the handshake message
                if let Some(msg_result) = session_manager_1.prepare_handshake_message(lp_id) {
                    let msg = msg_result.expect("Failed to prepare handshake message after KKT");
                    // Create a packet from the message
                    let packet = create_test_packet(1, lp_id, 0, msg);
                    packet_a_to_b = Some(packet);
                    println!("    Prepared first Noise message after KKTComplete");
                } else {
                    panic!("No handshake message available after KKT complete");
                }
            }
            other => {
                panic!(
                    "Initiator ReceivePacket produced unexpected action after KKT response: {:?}",
                    other
                );
            }
        }

        // --- Round 4: Responder Receives First Noise Message, Sends Second ---
        rounds += 1;
        println!(
            "  Round {}: Responder receives first Noise message, sends second",
            rounds
        );
        let packet_to_process = packet_a_to_b
            .take()
            .expect("First Noise packet from A was missing");

        // Simulate network
        let mut buf_a2 = BytesMut::new();
        serialize_lp_packet(&packet_to_process, &mut buf_a2).unwrap();
        let parsed_packet_a2 = parse_lp_packet(&buf_a2).unwrap();

        // Responder processes first Noise message and sends second Noise message
        let action_b2 = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(parsed_packet_a2))
            .expect("Responder ReceivePacket should produce an action")
            .expect("Responder ReceivePacket failed");

        if let LpAction::SendPacket(packet) = action_b2 {
            println!("    Responder received first Noise message, produced second Noise message (<- e, ee, s, es)");
            packet_b_to_a = Some(packet);
        } else {
            panic!("Responder did not produce SendPacket for second Noise message");
        }
        // Responder still in Handshaking, waiting for final message
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::Handshaking,
            "Responder state should still be Handshaking after sending second message"
        );

        // --- Round 5: Initiator Receives Second Noise Message, Sends Third, Completes ---
        rounds += 1;
        println!(
            "  Round {}: Initiator receives second Noise message, sends third, completes",
            rounds
        );
        let packet_to_process = packet_b_to_a
            .take()
            .expect("Second Noise packet from B was missing");

        let mut buf_b2 = BytesMut::new();
        serialize_lp_packet(&packet_to_process, &mut buf_b2).unwrap();
        let parsed_packet_b2 = parse_lp_packet(&buf_b2).unwrap();

        let action_a3 = session_manager_1
            .process_input(lp_id, LpInput::ReceivePacket(parsed_packet_b2))
            .expect("Initiator ReceivePacket should produce an action")
            .expect("Initiator ReceivePacket failed");

        if let LpAction::SendPacket(packet) = action_a3 {
            println!("    Initiator received second Noise message, produced third Noise message (-> s, se)");
            packet_a_to_b = Some(packet);
        } else {
            panic!("Initiator did not produce SendPacket for third Noise message");
        }
        // Initiator transitions to Transport after sending third message
        assert_eq!(
            session_manager_1.get_state(lp_id).unwrap(),
            LpStateBare::Transport,
            "Initiator state should be Transport after sending third message"
        );

        // --- Round 6: Responder Receives Third Noise Message, Completes ---
        rounds += 1;
        println!(
            "  Round {}: Responder receives third Noise message, completes",
            rounds
        );
        let packet_to_process = packet_a_to_b
            .take()
            .expect("Third Noise packet from A was missing");

        let mut buf_a3 = BytesMut::new();
        serialize_lp_packet(&packet_to_process, &mut buf_a3).unwrap();
        let parsed_packet_a3 = parse_lp_packet(&buf_a3).unwrap();

        let action_b3 = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(parsed_packet_a3))
            .expect("Responder final ReceivePacket should produce an action")
            .expect("Responder final ReceivePacket failed");

        // Responder completes handshake
        if let LpAction::HandshakeComplete = action_b3 {
            println!("    Responder received third Noise message, produced HandshakeComplete");
        } else {
            println!(
                "    Responder received third Noise message (Action: {:?})",
                action_b3
            );
        }
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::Transport,
            "Responder state should be Transport after processing third message"
        );

        // --- Verification ---
        assert!(rounds < MAX_ROUNDS, "Handshake took too many rounds");
        assert_eq!(
            session_manager_1.get_state(lp_id).unwrap(),
            LpStateBare::Transport
        );
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::Transport
        );
        println!("Handshake simulation completed successfully via process_input.");

        // --- 5. Simulate Data Transfer via process_input ---
        println!("Starting data transfer simulation via process_input...");
        let plaintext_a_to_b = b"Hello from A via process_input!";
        let plaintext_b_to_a = b"Hello from B via process_input!";

        // --- A sends to B ---
        println!("  A sends to B");
        let action_a_send = session_manager_1
            .process_input(lp_id, LpInput::SendData(plaintext_a_to_b.to_vec()))
            .expect("A SendData should produce action")
            .expect("A SendData failed");

        let data_packet_a = if let LpAction::SendPacket(packet) = action_a_send {
            packet
        } else {
            panic!("A SendData did not produce SendPacket");
        };

        // Simulate network
        let mut buf_data_a = BytesMut::new();
        serialize_lp_packet(&data_packet_a, &mut buf_data_a).unwrap();
        let parsed_data_a = parse_lp_packet(&buf_data_a).unwrap();

        // B receives
        println!("  B receives from A");
        let action_b_recv = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(parsed_data_a))
            .expect("B ReceivePacket (data) should produce action")
            .expect("B ReceivePacket (data) failed");

        if let LpAction::DeliverData(data) = action_b_recv {
            assert_eq!(
                data,
                Bytes::copy_from_slice(plaintext_a_to_b),
                "Decrypted data mismatch A->B"
            );
            println!(
                "    B successfully decrypted: {:?}",
                String::from_utf8_lossy(&data)
            );
        } else {
            panic!("B ReceivePacket did not produce DeliverData");
        }

        // --- B sends to A ---
        println!("  B sends to A");
        let action_b_send = session_manager_2
            .process_input(lp_id, LpInput::SendData(plaintext_b_to_a.to_vec()))
            .expect("B SendData should produce action")
            .expect("B SendData failed");

        let data_packet_b = if let LpAction::SendPacket(packet) = action_b_send {
            packet
        } else {
            panic!("B SendData did not produce SendPacket");
        };
        // Keep a copy for replay test
        let data_packet_b_replay = data_packet_b.clone();

        // Simulate network
        let mut buf_data_b = BytesMut::new();
        serialize_lp_packet(&data_packet_b, &mut buf_data_b).unwrap();
        let parsed_data_b = parse_lp_packet(&buf_data_b).unwrap();

        // A receives
        println!("  A receives from B");
        let action_a_recv = session_manager_1
            .process_input(lp_id, LpInput::ReceivePacket(parsed_data_b))
            .expect("A ReceivePacket (data) should produce action")
            .expect("A ReceivePacket (data) failed");

        if let LpAction::DeliverData(data) = action_a_recv {
            assert_eq!(
                data,
                Bytes::copy_from_slice(plaintext_b_to_a),
                "Decrypted data mismatch B->A"
            );
            println!(
                "    A successfully decrypted: {:?}",
                String::from_utf8_lossy(&data)
            );
        } else {
            panic!("A ReceivePacket did not produce DeliverData");
        }
        println!("Data transfer simulation completed.");

        // --- 6. Replay Protection Test ---
        println!("Testing data packet replay protection via process_input...");
        let replay_result =
            session_manager_1.process_input(lp_id, LpInput::ReceivePacket(data_packet_b_replay)); // Use cloned packet

        assert!(replay_result.is_err(), "Replay should produce Err(...)");
        let error = replay_result.err().unwrap();
        assert!(
            matches!(error, LpError::Replay(_)),
            "Expected Replay error, got {:?}",
            error
        );
        println!("Data packet replay protection test passed.");

        // --- 7. Out-of-Order Test ---
        println!("Testing out-of-order reception via process_input...");

        // A prepares N+1 then N
        let data_n_plus_1 = Bytes::from_static(b"Message N+1");
        let data_n = Bytes::from_static(b"Message N");

        let action_send_n1 = session_manager_1
            .process_input(lp_id, LpInput::SendData(data_n_plus_1.to_vec()))
            .unwrap()
            .unwrap();
        let packet_n1 = match action_send_n1 {
            LpAction::SendPacket(p) => p,
            _ => panic!("Expected SendPacket"),
        };

        let action_send_n = session_manager_1
            .process_input(lp_id, LpInput::SendData(data_n.to_vec()))
            .unwrap()
            .unwrap();
        let packet_n = match action_send_n {
            LpAction::SendPacket(p) => p,
            _ => panic!("Expected SendPacket"),
        };
        let packet_n_replay = packet_n.clone(); // For replay test

        // B receives N+1 first
        println!("  B receives N+1");
        let action_recv_n1 = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(packet_n1))
            .unwrap()
            .unwrap();
        match action_recv_n1 {
            LpAction::DeliverData(d) => assert_eq!(d, data_n_plus_1, "Data N+1 mismatch"),
            _ => panic!("Expected DeliverData for N+1"),
        }

        // B receives N second (should work)
        println!("  B receives N");
        let action_recv_n = session_manager_2
            .process_input(lp_id, LpInput::ReceivePacket(packet_n))
            .unwrap()
            .unwrap();
        match action_recv_n {
            LpAction::DeliverData(d) => assert_eq!(d, data_n, "Data N mismatch"),
            _ => panic!("Expected DeliverData for N"),
        }

        // B tries to replay N (should fail)
        println!("  B tries to replay N");
        let replay_n_result =
            session_manager_2.process_input(lp_id, LpInput::ReceivePacket(packet_n_replay));
        assert!(replay_n_result.is_err(), "Replay N should produce Err");
        assert!(
            matches!(replay_n_result.err().unwrap(), LpError::Replay(_)),
            "Expected Replay error for N"
        );
        println!("Out-of-order test passed.");

        // --- 8. Close Test ---
        println!("Testing close via process_input...");

        // A closes
        let action_a_close = session_manager_1
            .process_input(lp_id, LpInput::Close)
            .expect("A Close should produce action")
            .expect("A Close failed");
        assert!(matches!(action_a_close, LpAction::ConnectionClosed));
        assert_eq!(
            session_manager_1.get_state(lp_id).unwrap(),
            LpStateBare::Closed
        );

        // Further actions on A fail
        let send_after_close_a =
            session_manager_1.process_input(lp_id, LpInput::SendData(b"fail".to_vec()));
        assert!(send_after_close_a.is_err());
        assert!(matches!(
            send_after_close_a.err().unwrap(),
            LpError::LpSessionClosed
        ));

        // B closes
        let action_b_close = session_manager_2
            .process_input(lp_id, LpInput::Close)
            .expect("B Close should produce action")
            .expect("B Close failed");
        assert!(matches!(action_b_close, LpAction::ConnectionClosed));
        assert_eq!(
            session_manager_2.get_state(lp_id).unwrap(),
            LpStateBare::Closed
        );

        // Further actions on B fail
        let send_after_close_b =
            session_manager_2.process_input(lp_id, LpInput::SendData(b"fail".to_vec()));
        assert!(send_after_close_b.is_err());
        assert!(matches!(
            send_after_close_b.err().unwrap(),
            LpError::LpSessionClosed
        ));
        println!("Close test passed.");

        // --- 9. Session Removal ---
        assert!(session_manager_1.remove_state_machine(lp_id));
        assert_eq!(session_manager_1.session_count(), 0);
        assert!(!session_manager_1.state_machine_exists(lp_id));

        // B's session manager still has it until removed
        assert!(session_manager_2.state_machine_exists(lp_id));
        assert!(session_manager_2.remove_state_machine(lp_id));
        assert_eq!(session_manager_2.session_count(), 0);
        assert!(!session_manager_2.state_machine_exists(lp_id));
        println!("Session removal test passed.");
    }
    // ... other tests ...
}
