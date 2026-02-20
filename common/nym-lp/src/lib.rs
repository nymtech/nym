// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod codec;
// georgio: config does not seem to be used anywhere
// pub mod config;
// pub use config::LpConfig;

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

#[cfg(test)]
pub fn kem_list() -> Vec<nym_kkt_ciphersuite::KEM> {
    todo!()
    // use nym_kkt::ciphersuite::KEM;
    // vec![KEM::MlKem768, KEM::McEliece, KEM::X25519]
}

#[cfg(any(feature = "mock", test))]
pub struct SessionsMock {
    pub initiator: LpSession,
    pub responder: LpSession,
}

#[cfg(any(feature = "mock", test))]
impl SessionsMock {
    pub fn mock_post_handshake(session_id: u32) -> SessionsMock {
        todo!()
        // use crate::peer::mock_peers;
        // use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
        //
        // let (mut init, mut resp) = mock_peers();
        // let resp_remote = resp.as_remote();
        // let init_remote = init.as_remote();
        // let salt = [42u8; 32];
        // let session_id_bytes = session_id.to_le_bytes();
        //
        // // skip KKT by just deriving the kem key locally
        // let kem_keys = resp.kem_psq.as_ref().unwrap();
        //
        // let libcrux_private_key = libcrux_kem::PrivateKey::decode(
        //     libcrux_kem::Algorithm::X25519,
        //     kem_keys.private_key().as_bytes(),
        // )
        // .unwrap();
        // let decapsulation_key = DecapsulationKey::X25519(libcrux_private_key);
        //
        // let libcrux_public_key = libcrux_kem::PublicKey::decode(
        //     libcrux_kem::Algorithm::X25519,
        //     kem_keys.public_key().as_bytes(),
        // )
        // .unwrap();
        // let encapsulation_key = EncapsulationKey::X25519(libcrux_public_key);
        //
        // // INIT -> RESP: PSQ MSG1
        // let psq_initiator = crate::psk::psq_initiator_create_message(
        //     init.x25519.private_key(),
        //     &resp_remote.x25519_public,
        //     &encapsulation_key,
        //     init.ed25519.private_key(),
        //     init.ed25519.public_key(),
        //     &salt,
        //     &session_id_bytes,
        // )
        // .unwrap();
        //
        // let psk = psq_initiator.psk;
        // let psq_payload = psq_initiator.payload;
        // let outer_aead_key = crate::codec::OuterAeadKey::from_psk(&psk);
        //
        // let noise_state_init = snow::Builder::new(crate::noise_protocol::NoiseProtocol::params())
        //     .local_private_key(init.x25519().private_key().as_bytes())
        //     .remote_public_key(resp_remote.x25519_public.as_bytes())
        //     .psk(crate::NOISE_PSK_INDEX, &psk)
        //     .build_initiator()
        //     .unwrap();
        // let mut noise_protocol_init = crate::noise_protocol::NoiseProtocol::new(noise_state_init);
        // let noise_msg1 = noise_protocol_init.get_bytes_to_send().unwrap().unwrap();
        //
        // let psq_responder = crate::psk::psq_responder_process_message(
        //     resp.x25519.private_key(),
        //     &init_remote.x25519_public,
        //     (&decapsulation_key, &encapsulation_key),
        //     &init_remote.ed25519_public,
        //     &psq_payload,
        //     &salt,
        //     &session_id_bytes,
        // )
        // .unwrap();
        //
        // let noise_state_resp = snow::Builder::new(crate::noise_protocol::NoiseProtocol::params())
        //     .local_private_key(resp.x25519().private_key().as_bytes())
        //     .remote_public_key(init_remote.x25519_public.as_bytes())
        //     .psk(crate::NOISE_PSK_INDEX, &psk)
        //     .build_responder()
        //     .unwrap();
        // let mut noise_protocol_resp = crate::noise_protocol::NoiseProtocol::new(noise_state_resp);
        // noise_protocol_resp.read_message(&noise_msg1).unwrap();
        //
        // let noise_msg2 = noise_protocol_resp.get_bytes_to_send().unwrap().unwrap();
        // noise_protocol_init.read_message(&noise_msg2).unwrap();
        // let noise_msg3 = noise_protocol_init.get_bytes_to_send().unwrap().unwrap();
        //
        // assert!(noise_protocol_init.is_handshake_finished());
        //
        // noise_protocol_resp.read_message(&noise_msg3).unwrap();
        // assert!(noise_protocol_resp.is_handshake_finished());
        //
        // SessionsMock {
        //     initiator: LpSession::new(
        //         session_id,
        //         1,
        //         outer_aead_key.clone(),
        //         init,
        //         resp_remote,
        //         crate::session::PqSharedSecret::new(psq_initiator.pq_shared_secret),
        //         noise_protocol_init,
        //     ),
        //     responder: LpSession::new(
        //         session_id,
        //         1,
        //         outer_aead_key,
        //         resp,
        //         init_remote,
        //         crate::session::PqSharedSecret::new(psq_responder.pq_shared_secret),
        //         noise_protocol_resp,
        //     ),
        // }
    }

    // we just need a dummy 'valid' session for simpler tests
    pub fn mock_initiator() -> LpSession {
        Self::mock_post_handshake(1234).initiator
    }
}

#[cfg(any(feature = "mock", test))]
pub fn sessions_for_tests() -> (LpSession, LpSession) {
    let sessions = SessionsMock::mock_post_handshake(69);
    (sessions.initiator, sessions.responder)
}

#[cfg(any(feature = "mock", test))]
pub fn mock_session_for_test() -> LpSession {
    SessionsMock::mock_initiator()
}

#[cfg(test)]
mod tests {
    use crate::session_manager::SessionManager;
    use crate::{LpError, SessionsMock, kem_list, mock_session_for_test};
    use bytes::BytesMut;
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, SignatureScheme};

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

        for kem in kem_list() {
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
