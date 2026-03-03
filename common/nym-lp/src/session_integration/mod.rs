#[cfg(test)]
mod tests {
    use crate::packet::{EncryptedLpPacket, LpMessage};
    use crate::state_machine::{LpAction, LpInput, LpStateBare};
    use crate::{LpError, SessionManager, SessionsMock};
    use nym_kkt_ciphersuite::{IntoEnumIterator, KEM};

    // helpers to make tests smaller
    trait ActionExtract {
        fn ciphertext(self) -> EncryptedLpPacket;

        fn data(self) -> LpMessage;
    }

    impl ActionExtract for LpAction {
        fn ciphertext(self) -> EncryptedLpPacket {
            if let LpAction::SendPacket(packet) = self {
                packet
            } else {
                panic!("invalid action");
            }
        }

        fn data(self) -> LpMessage {
            if let LpAction::DeliverData(data) = self {
                data
            } else {
                panic!("invalid action");
            }
        }
    }

    /// Tests simultaneous bidirectional communication between sessions
    #[test]
    fn test_bidirectional_communication() {
        for kem in KEM::iter() {
            // 1. Initialize session manager
            let mut session_manager_1 = SessionManager::new();
            let mut session_manager_2 = SessionManager::new();
            let sessions = SessionsMock::mock_post_handshake(kem);

            // 2. Create sessions using the pre-built Noise states
            let peer_a_sm = session_manager_1
                .create_session_state_machine(sessions.initiator)
                .unwrap();
            let peer_b_sm = session_manager_2
                .create_session_state_machine(sessions.responder)
                .unwrap();

            // 3. Send multiple encrypted messages both ways
            const NUM_MESSAGES: u64 = 5;
            for i in 0..NUM_MESSAGES {
                println!("Bidirectional test: Round {i}");
                // --- A sends to B ---
                let plaintext_a = format!("A->B Message {i}").into_bytes();
                let ciphertext_a = session_manager_1
                    .send_data(peer_a_sm, LpMessage::new_opaque(plaintext_a.clone()))
                    .unwrap()
                    .ciphertext();

                // B parses and checks replay
                let decrypted_payload = session_manager_2
                    .receive_packet(peer_b_sm, ciphertext_a)
                    .unwrap()
                    .unwrap()
                    .data();
                assert_eq!(decrypted_payload.content, plaintext_a);

                // --- B sends to A ---
                let plaintext_b = format!("B->A Message {i}").into_bytes();
                let ciphertext_b = session_manager_2
                    .send_data(peer_b_sm, LpMessage::new_opaque(plaintext_b.clone()))
                    .unwrap()
                    .ciphertext();

                // B parses and checks replay
                let decrypted_payload = session_manager_1
                    .receive_packet(peer_a_sm, ciphertext_b)
                    .unwrap()
                    .unwrap()
                    .data();
                assert_eq!(decrypted_payload.content, plaintext_b);
            }

            // 5. Verify counter stats
            // Note: current_packet_cnt() returns (next_expected_receive_counter, total_received)
            let count_a = session_manager_1.current_packet_cnt(peer_a_sm).unwrap();
            let count_b = session_manager_2.current_packet_cnt(peer_b_sm).unwrap();

            // Peer A sent handshake(0), handshake(1) + 5 data packets = 7 total. Next send counter = 7.
            // Peer A received handshake(0) + 5 data packets = 6 total. Next expected recv counter = 6.
            assert_eq!(
                count_a.received, NUM_MESSAGES,
                "Peer A total received count mismatch"
            ); // Received 5 data
            assert_eq!(
                count_a.next, NUM_MESSAGES,
                "Peer A next expected receive counter mismatch"
            ); // Expected counter for msg from B

            // Peer B sent handshake(0) + 5 data packets = 6 total. Next send counter = 6.
            // Peer B received handshake(0), handshake(1) + 5 data packets = 7 total. Next expected recv counter = 7.
            assert_eq!(
                count_b.received, NUM_MESSAGES,
                "Peer B total received count mismatch"
            ); // Received 5 data
            assert_eq!(
                count_b.next, NUM_MESSAGES,
                "Peer B next expected receive counter mismatch"
            ); // Expected counter for msg from A

            println!("Bidirectional test completed.");
        }
    }

    /// Tests error handling in session flow
    #[test]
    fn test_session_error_handling() {
        for kem in KEM::iter() {
            // 1. Initialize session manager
            let mut session_manager = SessionManager::new();

            let sessions = SessionsMock::mock_post_handshake(kem);
            let session_id = sessions.initiator.receiver_index();

            let non_existent = 123;
            // sanity check in case of the 1 in 2^256
            assert_ne!(session_id, non_existent);

            let session1 = sessions.initiator;
            let session2 = sessions.responder;

            // 2. Create a session (using real noise state)
            let _session = session_manager.create_session_state_machine(session1);

            // 3. Try to get a non-existent session
            let result = session_manager.state_machine_exists(non_existent);
            assert!(!result, "Non-existent session should return None");

            // 4. Try to remove a non-existent session
            let result = session_manager.remove_state_machine(non_existent);
            assert!(
                !result,
                "Remove session should not remove a non-existent session"
            );

            // 5. Create and immediately remove a session
            let _temp_session = session_manager.create_session_state_machine(session2);

            assert!(
                session_manager.remove_state_machine(session_id),
                "Should remove the session"
            );
        }
    }

    /// Tests the complete session flow using ONLY the process_input interface:
    /// - Creation of sessions through session manager
    /// - Data transfer driven by SendData, ReceivePacket inputs
    /// - Actions like SendPacket, DeliverData handled from output
    /// - Implicit replay protection via state machine logic
    /// - Closing driven by Close input
    #[test]
    fn test_full_session_flow() {
        // 1. Initialize session managers
        let mut session_manager_1 = SessionManager::new();
        let mut session_manager_2 = SessionManager::new();

        for kem in KEM::iter() {
            let sessions = SessionsMock::mock_post_handshake(kem);
            let session_id = sessions.responder.receiver_index();

            // 2. Create sessions state machines
            session_manager_1
                .create_session_state_machine(sessions.initiator)
                .unwrap();
            session_manager_2
                .create_session_state_machine(sessions.responder)
                .unwrap();

            assert_eq!(session_manager_1.session_count(), 1);
            assert_eq!(session_manager_2.session_count(), 1);
            assert!(session_manager_1.state_machine_exists(session_id));
            assert!(session_manager_2.state_machine_exists(session_id));

            // Verify initial states are Transport
            assert_eq!(
                session_manager_1.get_state(session_id).unwrap(),
                LpStateBare::Transport
            );
            assert_eq!(
                session_manager_2.get_state(session_id).unwrap(),
                LpStateBare::Transport
            );

            // --- 3. Simulate Data Transfer via process_input ---
            println!("Starting data transfer simulation via process_input...");
            let plaintext_a_to_b =
                LpMessage::new_opaque(b"Hello from A via process_input!".to_vec());
            let plaintext_b_to_a =
                LpMessage::new_opaque(b"Hello from B via process_input!".to_vec());

            // --- A sends to B ---
            println!("  A sends to B");
            let action_a_send = session_manager_1
                .process_input(session_id, LpInput::SendData(plaintext_a_to_b.clone()))
                .expect("A SendData should produce action")
                .expect("A SendData failed");

            let data_packet_a = action_a_send.ciphertext();

            // B receives
            println!("  B receives from A");
            let action_b_recv = session_manager_2
                .process_input(session_id, LpInput::ReceivePacket(data_packet_a))
                .expect("B ReceivePacket (data) should produce action")
                .expect("B ReceivePacket (data) failed");

            if let LpAction::DeliverData(data) = action_b_recv {
                assert_eq!(data, plaintext_a_to_b, "Decrypted data mismatch A->B");
                println!(
                    "    B successfully decrypted: {:?}",
                    String::from_utf8_lossy(&data.content)
                );
            } else {
                panic!("B ReceivePacket did not produce DeliverData");
            }

            // --- B sends to A ---
            println!("  B sends to A");
            let action_b_send = session_manager_2
                .process_input(session_id, LpInput::SendData(plaintext_b_to_a.clone()))
                .expect("B SendData should produce action")
                .expect("B SendData failed");

            let data_packet_b = action_b_send.ciphertext();

            // Keep a copy for replay test
            let data_packet_b_replay = data_packet_b.clone();

            // A receives
            println!("  A receives from B");
            let action_a_recv = session_manager_1
                .process_input(session_id, LpInput::ReceivePacket(data_packet_b))
                .expect("A ReceivePacket (data) should produce action")
                .expect("A ReceivePacket (data) failed");

            if let LpAction::DeliverData(data) = action_a_recv {
                assert_eq!(data, plaintext_b_to_a, "Decrypted data mismatch B->A");
                println!(
                    "    A successfully decrypted: {:?}",
                    String::from_utf8_lossy(&data.content)
                );
            } else {
                panic!("A ReceivePacket did not produce DeliverData");
            }
            println!("Data transfer simulation completed.");

            // --- 4. Replay Protection Test ---
            println!("Testing data packet replay protection via process_input...");
            let replay_result = session_manager_1
                .process_input(session_id, LpInput::ReceivePacket(data_packet_b_replay)); // Use cloned packet

            assert!(replay_result.is_err(), "Replay should produce Err(...)");
            let error = replay_result.err().unwrap();
            assert!(
                matches!(error, LpError::Replay(_)),
                "Expected Replay error, got {:?}",
                error
            );
            println!("Data packet replay protection test passed.");

            // --- 5. Out-of-Order Test ---
            println!("Testing out-of-order reception via process_input...");

            // A prepares N+1 then N
            let data_n_plus_1 = LpMessage::new_opaque(b"Message N+1".to_vec());
            let data_n = LpMessage::new_opaque(b"Message N".to_vec());

            let action_send_n1 = session_manager_1
                .process_input(session_id, LpInput::SendData(data_n_plus_1.clone()))
                .unwrap()
                .unwrap();
            let packet_n1 = match action_send_n1 {
                LpAction::SendPacket(p) => p,
                _ => panic!("Expected SendPacket"),
            };

            let action_send_n = session_manager_1
                .process_input(session_id, LpInput::SendData(data_n.clone()))
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
                .process_input(session_id, LpInput::ReceivePacket(packet_n1))
                .unwrap()
                .unwrap();
            match action_recv_n1 {
                LpAction::DeliverData(d) => assert_eq!(d, data_n_plus_1, "Data N+1 mismatch"),
                _ => panic!("Expected DeliverData for N+1"),
            }

            // B receives N second (should work)
            println!("  B receives N");
            let action_recv_n = session_manager_2
                .process_input(session_id, LpInput::ReceivePacket(packet_n))
                .unwrap()
                .unwrap();
            match action_recv_n {
                LpAction::DeliverData(d) => assert_eq!(d, data_n, "Data N mismatch"),
                _ => panic!("Expected DeliverData for N"),
            }

            // B tries to replay N (should fail)
            println!("  B tries to replay N");
            let replay_n_result = session_manager_2
                .process_input(session_id, LpInput::ReceivePacket(packet_n_replay));
            assert!(replay_n_result.is_err(), "Replay N should produce Err");
            assert!(
                matches!(replay_n_result.err().unwrap(), LpError::Replay(_)),
                "Expected Replay error for N"
            );
            println!("Out-of-order test passed.");

            // --- 6. Close Test ---
            println!("Testing close via process_input...");

            // A closes
            let action_a_close = session_manager_1
                .process_input(session_id, LpInput::Close)
                .expect("A Close should produce action")
                .expect("A Close failed");
            assert!(matches!(action_a_close, LpAction::ConnectionClosed));
            assert_eq!(
                session_manager_1.get_state(session_id).unwrap(),
                LpStateBare::Closed
            );

            // Further actions on A fail
            let send_after_close_a = session_manager_1.process_input(
                session_id,
                LpInput::SendData(LpMessage::new_opaque(b"fail".to_vec())),
            );
            assert!(send_after_close_a.is_err());
            assert!(matches!(
                send_after_close_a.err().unwrap(),
                LpError::LpSessionClosed
            ));

            // B closes
            let action_b_close = session_manager_2
                .process_input(session_id, LpInput::Close)
                .expect("B Close should produce action")
                .expect("B Close failed");
            assert!(matches!(action_b_close, LpAction::ConnectionClosed));
            assert_eq!(
                session_manager_2.get_state(session_id).unwrap(),
                LpStateBare::Closed
            );

            // Further actions on B fail
            let send_after_close_b = session_manager_2.process_input(
                session_id,
                LpInput::SendData(LpMessage::new_opaque(b"fail".to_vec())),
            );
            assert!(send_after_close_b.is_err());
            assert!(matches!(
                send_after_close_b.err().unwrap(),
                LpError::LpSessionClosed
            ));
            println!("Close test passed.");

            // --- 7. Session Removal ---
            assert!(session_manager_1.remove_state_machine(session_id));
            assert_eq!(session_manager_1.session_count(), 0);
            assert!(!session_manager_1.state_machine_exists(session_id));

            // B's session manager still has it until removed
            assert!(session_manager_2.state_machine_exists(session_id));
            assert!(session_manager_2.remove_state_machine(session_id));
            assert_eq!(session_manager_2.session_count(), 0);
            assert!(!session_manager_2.state_machine_exists(session_id));
            println!("Session removal test passed.");
        }
    }
    // ... other tests ...
}
