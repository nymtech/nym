// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod codec;
pub mod error;
pub mod packet;
pub mod peer;
pub mod psq;
pub mod replay;
pub mod session;
mod session_integration;
pub mod session_manager;
pub mod state_machine;

pub use error::LpError;
pub use nym_kkt_ciphersuite::{
    Ciphersuite, HashFunction, HashLength, KEM, KEMKeyDigests, SignatureScheme,
};
pub use nym_lp_packet::{
    EncryptedLpPacket, LpMessage, LpPacket, OuterHeader,
    error::MalformedLpPacketError,
    message::{ApplicationData, ExpectedResponseSize, ForwardPacketData},
};
pub use replay::{ReceivingKeyCounterValidator, ReplayError};
pub use session::LpSession;
pub use session_manager::SessionManager;
pub use state_machine::LpStateMachine;

#[cfg(any(feature = "mock", test))]
use nym_test_utils::helpers::u64_seeded_rng_09;

#[cfg(any(feature = "mock", test))]
use crate::psq::{PSQ_MSG2_SIZE, initiator, psq_msg1_size, responder};

#[cfg(any(feature = "mock", test))]
use crate::session::PersistentSessionBinding;

#[cfg(any(feature = "mock", test))]
use libcrux_psq::{Channel, IntoSession};

#[cfg(any(feature = "mock", test))]
pub struct SessionsMock {
    pub initiator: LpSession,
    pub responder: LpSession,
}

#[cfg(any(feature = "mock", test))]
impl SessionsMock {
    pub fn mock_seeded_post_handshake(seed: u64, kem: KEM) -> SessionsMock {
        use crate::peer::mock_peers;

        let (init, resp) = mock_peers();
        let resp_remote = resp.as_remote();

        let init_rng = u64_seeded_rng_09(seed);
        let resp_rng = u64_seeded_rng_09(seed + 1);

        let kem_keys = resp.kem_keypairs.as_ref().unwrap();

        // skip KKT by just deriving the kem key locally
        let encapsulation_key = kem_keys.encapsulation_key(kem).unwrap();
        let enc_key = encapsulation_key.clone();

        let initiator_ciphersuite =
            initiator::build_psq_ciphersuite(&init, &resp_remote, &enc_key).unwrap();
        let mut initiator =
            initiator::build_psq_principal(init_rng, 1, initiator_ciphersuite).unwrap();

        let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem).unwrap();
        let mut responder =
            responder::build_psq_principal(resp_rng, 1, responder_ciphersuite).unwrap();

        // run PSQ
        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];

        // Send first message
        let mut buf = vec![0u8; psq_msg1_size(kem)];
        let len_i = initiator.write_message(&[], &mut buf).unwrap();
        assert_eq!(len_i, buf.len());

        // Read first message
        let (_, _) = responder
            .read_message(&buf, &mut payload_buf_responder)
            .unwrap();

        // Get the authenticator out here, so we can deserialize the session later.
        let Some(initiator_authenticator) = responder.initiator_authenticator() else {
            panic!("No initiator authenticator found")
        };

        // Respond
        let mut buf = [0u8; PSQ_MSG2_SIZE];
        let len_r = responder.write_message(&[], &mut buf).unwrap();
        assert_eq!(len_r, buf.len());

        // Finalize on registration initiator
        let (_, _) = initiator
            .read_message(&buf, &mut payload_buf_initiator)
            .unwrap();

        assert!(initiator.is_handshake_finished());
        assert!(responder.is_handshake_finished());

        let binding = PersistentSessionBinding {
            initiator_authenticator,
            responder_ecdh_pk: resp_remote.x25519_public,
            responder_pq_pk: Some(encapsulation_key),
        };

        SessionsMock {
            initiator: LpSession::new(initiator.into_session().unwrap(), binding.clone(), 1)
                .unwrap(),
            responder: LpSession::new(responder.into_session().unwrap(), binding, 1).unwrap(),
        }
    }

    pub fn mock_post_handshake(kem: KEM) -> SessionsMock {
        Self::mock_seeded_post_handshake(1, kem)
    }

    // we just need a dummy 'valid' session for simpler tests
    pub fn mock_initiator() -> LpSession {
        Self::mock_post_handshake(KEM::default()).initiator
    }
}

#[cfg(any(feature = "mock", test))]
pub fn sessions_for_tests() -> (LpSession, LpSession) {
    let sessions = SessionsMock::mock_post_handshake(KEM::default());
    (sessions.initiator, sessions.responder)
}

#[cfg(any(feature = "mock", test))]
pub fn mock_session_for_test() -> LpSession {
    SessionsMock::mock_initiator()
}

#[cfg(test)]
mod tests {
    use crate::session_manager::SessionManager;
    use crate::{LpError, SessionsMock, mock_session_for_test};
    use bytes::BytesMut;
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, IntoEnumIterator, KEM, SignatureScheme};

    // Import the new standalone functions
    use crate::codec::serialize_lp_packet;

    #[test]
    fn test_replay_protection_integration() {
        todo!()
        // for kem in kem_list() {
        //     // Create session
        //     let mut session = mock_session_for_test();
        //
        //     // === Packet 1 (Counter 0 - Should succeed) ===
        //     let packet1 = LpPacket {
        //         header: LpHeader {
        //             protocol_version: 1,
        //             reserved: [0u8; 3],
        //             receiver_idx: 42, // Matches session's sending_index assumption for this test
        //             counter: 0,
        //         },
        //         message: LpMessage::Busy,
        //         trailer: [0u8; TRAILER_LEN],
        //     };
        //
        //     // Serialize packet
        //     let mut buf1 = BytesMut::new();
        //     serialize_lp_packet(&packet1, &mut buf1, None).unwrap();
        //
        //     // Parse packet
        //     let parsed_packet1 = parse_lp_packet(&buf1, None).unwrap();
        //
        //     // Perform replay check (should pass)
        //     session
        //         .receiving_counter_quick_check(parsed_packet1.header.counter)
        //         .expect("Initial packet failed replay check");
        //
        //     // Mark received (simulating successful processing)
        //     session
        //         .receiving_counter_mark(parsed_packet1.header.counter)
        //         .expect("Failed to mark initial packet received");
        //
        //     // === Packet 2 (Counter 0 - Replay, should fail check) ===
        //     let packet2 = LpPacket {
        //         header: LpHeader {
        //             protocol_version: 1,
        //             reserved: [0u8; 3],
        //             receiver_idx: 42,
        //             counter: 0, // Same counter as before (replay)
        //         },
        //         message: LpMessage::Busy,
        //         trailer: [0u8; TRAILER_LEN],
        //     };
        //
        //     // Serialize packet
        //     let mut buf2 = BytesMut::new();
        //     serialize_lp_packet(&packet2, &mut buf2, None).unwrap();
        //
        //     // Parse packet
        //     let parsed_packet2 = parse_lp_packet(&buf2, None).unwrap();
        //
        //     // Perform replay check (should fail)
        //     let replay_result =
        //         session.receiving_counter_quick_check(parsed_packet2.header.counter);
        //     assert!(replay_result.is_err());
        //     match replay_result.unwrap_err() {
        //         LpError::Replay(e) => {
        //             assert!(matches!(e, crate::replay::ReplayError::DuplicateCounter));
        //         }
        //         e => panic!("Expected replay error, got {:?}", e),
        //     }
        //     // Do not mark received as it failed validation
        //
        //     // === Packet 3 (Counter 1 - Should succeed) ===
        //     let packet3 = LpPacket {
        //         header: LpHeader {
        //             protocol_version: 1,
        //             reserved: [0u8; 3],
        //             receiver_idx: 42,
        //             counter: 1, // Incremented counter
        //         },
        //         message: LpMessage::Busy,
        //         trailer: [0u8; TRAILER_LEN],
        //     };
        //
        //     // Serialize packet
        //     let mut buf3 = BytesMut::new();
        //     serialize_lp_packet(&packet3, &mut buf3, None).unwrap();
        //
        //     // Parse packet
        //     let parsed_packet3 = parse_lp_packet(&buf3, None).unwrap();
        //
        //     // Perform replay check (should pass)
        //     session
        //         .receiving_counter_quick_check(parsed_packet3.header.counter)
        //         .expect("Packet 3 failed replay check");
        //
        //     // Mark received
        //     session
        //         .receiving_counter_mark(parsed_packet3.header.counter)
        //         .expect("Failed to mark packet 3 received");
        //
        //     // Verify validator state directly on the session
        //     let state = session.current_packet_cnt();
        //     assert_eq!(state.0, 2); // Next expected counter (correct - was 1, now expects 2)
        //     assert_eq!(state.1, 2); // Total marked received (correct - packets 1 and 3)
        // }
    }

    #[test]
    fn test_session_manager_integration() {
        // Create session manager
        let mut local_manager = SessionManager::new();
        let mut remote_manager = SessionManager::new();

        for kem in KEM::iter() {
            todo!()
            // // Generate Ed25519 keypairs for PSQ authentication
            // let (init, resp) = mock_peers(kem);
            //
            // let mut ciphersuite = Ciphersuite::resolve_ciphersuite(
            //     kem,
            //     HashFunction::Blake3,
            //     SignatureScheme::Ed25519,
            //     None,
            // );
            //
            // // Use fixed receiver_index for deterministic test
            // let receiver_index: u32 = 54321;
            //
            // let sessions = SessionsMock::mock_post_handshake(receiver_index);
            // let local_session = sessions.initiator;
            // let remote_session = sessions.responder;
            //
            // // Create a session via manager
            // let _ = local_manager.create_session_state_machine(local_session);
            // let _ = remote_manager.create_session_state_machine(remote_session);
            //
            // // === Packet 1 (Counter 0 - Should succeed) ===
            // let packet1 = LpPacket {
            //     header: LpHeader {
            //         protocol_version: 1,
            //         reserved: [0u8; 3],
            //         receiver_idx: receiver_index,
            //         counter: 0,
            //     },
            //     message: LpMessage::Busy,
            //     trailer: [0u8; TRAILER_LEN],
            // };
            //
            // // Serialize
            // let mut buf1 = BytesMut::new();
            // serialize_lp_packet(&packet1, &mut buf1, None).unwrap();
            //
            // // Parse
            // let parsed_packet1 = parse_lp_packet(&buf1, None).unwrap();
            //
            // // Process via SessionManager method (which should handle checks + marking)
            // // NOTE: We might need a method on SessionManager/LpSession like `process_incoming_packet`
            // //       that encapsulates parse -> check -> process_noise -> mark.
            // //       For now, we simulate the steps using the retrieved session.
            //
            // // Perform replay check
            // local_manager
            //     .receiving_counter_quick_check(receiver_index, parsed_packet1.header.counter)
            //     .expect("Packet 1 check failed");
            // // Mark received
            // local_manager
            //     .receiving_counter_mark(receiver_index, parsed_packet1.header.counter)
            //     .expect("Packet 1 mark failed");
            //
            // // === Packet 2 (Counter 1 - Should succeed on same session) ===
            // let packet2 = LpPacket {
            //     header: LpHeader {
            //         protocol_version: 1,
            //         reserved: [0u8; 3],
            //         receiver_idx: receiver_index,
            //         counter: 1,
            //     },
            //     message: LpMessage::Busy,
            //     trailer: [0u8; TRAILER_LEN],
            // };
            //
            // // Serialize
            // let mut buf2 = BytesMut::new();
            // serialize_lp_packet(&packet2, &mut buf2, None).unwrap();
            //
            // // Parse
            // let parsed_packet2 = parse_lp_packet(&buf2, None).unwrap();
            //
            // // Perform replay check
            // local_manager
            //     .receiving_counter_quick_check(receiver_index, parsed_packet2.header.counter)
            //     .expect("Packet 2 check failed");
            // // Mark received
            // local_manager
            //     .receiving_counter_mark(receiver_index, parsed_packet2.header.counter)
            //     .expect("Packet 2 mark failed");
            //
            // // === Packet 3 (Counter 0 - Replay, should fail check) ===
            // let packet3 = LpPacket {
            //     header: LpHeader {
            //         protocol_version: 1,
            //         reserved: [0u8; 3],
            //         receiver_idx: receiver_index,
            //         counter: 0, // Replay of first packet
            //     },
            //     message: LpMessage::Busy,
            //     trailer: [0u8; TRAILER_LEN],
            // };
            //
            // // Serialize
            // let mut buf3 = BytesMut::new();
            // serialize_lp_packet(&packet3, &mut buf3, None).unwrap();
            //
            // // Parse
            // let parsed_packet3 = parse_lp_packet(&buf3, None).unwrap();
            //
            // // Perform replay check (should fail)
            // let replay_result = local_manager
            //     .receiving_counter_quick_check(receiver_index, parsed_packet3.header.counter);
            // assert!(replay_result.is_err());
            // match replay_result.unwrap_err() {
            //     LpError::Replay(e) => {
            //         assert!(matches!(e, crate::replay::ReplayError::DuplicateCounter));
            //     }
            //     e => panic!("Expected replay error for packet 3, got {:?}", e),
            // }
            // // Do not mark received
        }
    }
}
