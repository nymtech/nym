// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod codec;
// georgio: config does not seem to be used anywhere
// pub mod config;
// pub use config::LpConfig;

pub mod error;
// georgio: no use for this
// pub mod kkt_orchestrator;
pub mod message;
pub mod noise_protocol;
pub mod packet;
pub mod peer;
pub mod psk;
pub mod psq;
pub mod replay;
pub mod session;
mod session_integration;
pub mod session_manager;
pub mod state_machine;

pub use error::LpError;
pub use message::{ClientHelloData, LpMessage};
pub use packet::{BOOTSTRAP_RECEIVER_IDX, LpPacket, OuterHeader};
pub use replay::{ReceivingKeyCounterValidator, ReplayError};
pub use session::LpSession;
pub use session_manager::SessionManager;
pub use state_machine::LpStateMachine;

// noiserm
pub const NOISE_PATTERN: &str = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
pub const NOISE_PSK_INDEX: u8 = 3;

#[cfg(test)]
pub fn kem_list() -> Vec<nym_kkt::ciphersuite::KEM> {
    use nym_kkt::ciphersuite::KEM;
    vec![KEM::MlKem768, KEM::McEliece, KEM::X25519]
}
#[cfg(test)]
pub fn sessions_for_tests<'a>(kem: nym_kkt::ciphersuite::KEM) -> (LpSession<'a>, LpSession<'a>) {
    let (init, resp) = crate::peer::mock_peers(kem);

    // Use a fixed receiver_index for deterministic tests
    let receiver_index: u32 = 12345;

    let initiator_session =
        LpSession::new(receiver_index, true, init.clone(), resp.as_remote(), &salt)
            .expect("Test session creation failed");

    let responder_session = LpSession::new(receiver_index, false, resp, init.as_remote(), &salt)
        .expect("Test session creation failed");

    (initiator_session, responder_session)
}

#[cfg(test)]
mod tests {
    use crate::message::LpMessage;
    use crate::packet::{LpHeader, LpPacket, TRAILER_LEN};
    use crate::session_manager::SessionManager;
    use crate::{LpError, kem_list, sessions_for_tests};
    use bytes::BytesMut;
    use nym_kkt::ciphersuite::{Ciphersuite, HashFunction, SignatureScheme};

    // Import the new standalone functions
    use crate::codec::{parse_lp_packet, serialize_lp_packet};
    use crate::peer::mock_peers;

    #[test]
    fn test_replay_protection_integration() {
        for kem in kem_list() {
            // Create session
            let session = sessions_for_tests(kem).0;

            // === Packet 1 (Counter 0 - Should succeed) ===
            let packet1 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: 42, // Matches session's sending_index assumption for this test
                    counter: 0,
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize packet
            let mut buf1 = BytesMut::new();
            serialize_lp_packet(&packet1, &mut buf1, None).unwrap();

            // Parse packet
            let parsed_packet1 = parse_lp_packet(&buf1, None).unwrap();

            // Perform replay check (should pass)
            session
                .receiving_counter_quick_check(parsed_packet1.header.counter)
                .expect("Initial packet failed replay check");

            // Mark received (simulating successful processing)
            session
                .receiving_counter_mark(parsed_packet1.header.counter)
                .expect("Failed to mark initial packet received");

            // === Packet 2 (Counter 0 - Replay, should fail check) ===
            let packet2 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: 42,
                    counter: 0, // Same counter as before (replay)
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize packet
            let mut buf2 = BytesMut::new();
            serialize_lp_packet(&packet2, &mut buf2, None).unwrap();

            // Parse packet
            let parsed_packet2 = parse_lp_packet(&buf2, None).unwrap();

            // Perform replay check (should fail)
            let replay_result =
                session.receiving_counter_quick_check(parsed_packet2.header.counter);
            assert!(replay_result.is_err());
            match replay_result.unwrap_err() {
                LpError::Replay(e) => {
                    assert!(matches!(e, crate::replay::ReplayError::DuplicateCounter));
                }
                e => panic!("Expected replay error, got {:?}", e),
            }
            // Do not mark received as it failed validation

            // === Packet 3 (Counter 1 - Should succeed) ===
            let packet3 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: 42,
                    counter: 1, // Incremented counter
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize packet
            let mut buf3 = BytesMut::new();
            serialize_lp_packet(&packet3, &mut buf3, None).unwrap();

            // Parse packet
            let parsed_packet3 = parse_lp_packet(&buf3, None).unwrap();

            // Perform replay check (should pass)
            session
                .receiving_counter_quick_check(parsed_packet3.header.counter)
                .expect("Packet 3 failed replay check");

            // Mark received
            session
                .receiving_counter_mark(parsed_packet3.header.counter)
                .expect("Failed to mark packet 3 received");

            // Verify validator state directly on the session
            let state = session.current_packet_cnt();
            assert_eq!(state.0, 2); // Next expected counter (correct - was 1, now expects 2)
            assert_eq!(state.1, 2); // Total marked received (correct - packets 1 and 3)
        }
    }
    #[test]
    fn test_session_manager_integration() {
        // Create session manager
        let mut local_manager = SessionManager::new();
        let mut remote_manager = SessionManager::new();

        for kem in kem_list() {
            // Generate Ed25519 keypairs for PSQ authentication
            let (init, resp) = mock_peers(kem);

            let mut ciphersuite = Ciphersuite::resolve_ciphersuite(
                kem,
                HashFunction::Blake3,
                SignatureScheme::Ed25519,
                None,
            );

            // Use fixed receiver_index for deterministic test
            let receiver_index: u32 = 54321;

            // Test salt
            let salt = [46u8; 32];

            // Create a session via manager
            let _ = local_manager
                .create_session_state_machine(
                    receiver_index,
                    true,
                    init.clone(),
                    resp.as_remote(),
                    &salt,
                )
                .unwrap();

            let _ = remote_manager
                .create_session_state_machine(receiver_index, false, resp, init.as_remote(), &salt)
                .unwrap();
            // === Packet 1 (Counter 0 - Should succeed) ===
            let packet1 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: receiver_index,
                    counter: 0,
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize
            let mut buf1 = BytesMut::new();
            serialize_lp_packet(&packet1, &mut buf1, None).unwrap();

            // Parse
            let parsed_packet1 = parse_lp_packet(&buf1, None).unwrap();

            // Process via SessionManager method (which should handle checks + marking)
            // NOTE: We might need a method on SessionManager/LpSession like `process_incoming_packet`
            //       that encapsulates parse -> check -> process_noise -> mark.
            //       For now, we simulate the steps using the retrieved session.

            // Perform replay check
            local_manager
                .receiving_counter_quick_check(receiver_index, parsed_packet1.header.counter)
                .expect("Packet 1 check failed");
            // Mark received
            local_manager
                .receiving_counter_mark(receiver_index, parsed_packet1.header.counter)
                .expect("Packet 1 mark failed");

            // === Packet 2 (Counter 1 - Should succeed on same session) ===
            let packet2 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: receiver_index,
                    counter: 1,
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize
            let mut buf2 = BytesMut::new();
            serialize_lp_packet(&packet2, &mut buf2, None).unwrap();

            // Parse
            let parsed_packet2 = parse_lp_packet(&buf2, None).unwrap();

            // Perform replay check
            local_manager
                .receiving_counter_quick_check(receiver_index, parsed_packet2.header.counter)
                .expect("Packet 2 check failed");
            // Mark received
            local_manager
                .receiving_counter_mark(receiver_index, parsed_packet2.header.counter)
                .expect("Packet 2 mark failed");

            // === Packet 3 (Counter 0 - Replay, should fail check) ===
            let packet3 = LpPacket {
                header: LpHeader {
                    protocol_version: 1,
                    reserved: [0u8; 3],
                    receiver_idx: receiver_index,
                    counter: 0, // Replay of first packet
                },
                message: LpMessage::Busy,
                trailer: [0u8; TRAILER_LEN],
            };

            // Serialize
            let mut buf3 = BytesMut::new();
            serialize_lp_packet(&packet3, &mut buf3, None).unwrap();

            // Parse
            let parsed_packet3 = parse_lp_packet(&buf3, None).unwrap();

            // Perform replay check (should fail)
            let replay_result = local_manager
                .receiving_counter_quick_check(receiver_index, parsed_packet3.header.counter);
            assert!(replay_result.is_err());
            match replay_result.unwrap_err() {
                LpError::Replay(e) => {
                    assert!(matches!(e, crate::replay::ReplayError::DuplicateCounter));
                }
                e => panic!("Expected replay error for packet 3, got {:?}", e),
            }
            // Do not mark received
        }
    }
}
