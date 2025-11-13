// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod codec;
pub mod error;
pub mod keypair;
pub mod kkt_orchestrator;
pub mod message;
pub mod noise_protocol;
pub mod packet;
pub mod psk;
pub mod replay;
pub mod session;
mod session_integration;
pub mod session_manager;

use std::hash::{DefaultHasher, Hasher as _};

pub use error::LpError;
use keypair::PublicKey;
pub use message::{ClientHelloData, LpMessage};
pub use packet::LpPacket;
pub use psk::derive_psk;
pub use replay::{ReceivingKeyCounterValidator, ReplayError};
pub use session::{generate_fresh_salt, LpSession};
pub use session_manager::SessionManager;

// Add the new state machine module
pub mod state_machine;
pub use state_machine::LpStateMachine;

pub const NOISE_PATTERN: &str = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
pub const NOISE_PSK_INDEX: u8 = 3;

#[cfg(test)]
pub fn sessions_for_tests() -> (LpSession, LpSession) {
    use crate::{keypair::Keypair, make_lp_id};
    use nym_crypto::asymmetric::ed25519;

    // X25519 keypairs for Noise protocol
    let keypair_1 = Keypair::default();
    let keypair_2 = Keypair::default();
    let id = make_lp_id(keypair_1.public_key(), keypair_2.public_key());

    // Ed25519 keypairs for PSQ authentication (placeholders for testing)
    let ed25519_keypair_1 = ed25519::KeyPair::from_secret([1u8; 32], 0);
    let ed25519_keypair_2 = ed25519::KeyPair::from_secret([2u8; 32], 1);

    // Use consistent salt for deterministic tests
    let salt = [1u8; 32];

    // PSQ will always derive the PSK during handshake using X25519 as DHKEM

    let initiator_session = LpSession::new(
        id,
        true,
        (ed25519_keypair_1.private_key(), ed25519_keypair_1.public_key()),
        keypair_1.private_key(),
        ed25519_keypair_2.public_key(),
        keypair_2.public_key(),
        &salt,
    )
    .expect("Test session creation failed");

    let responder_session = LpSession::new(
        id,
        false,
        (ed25519_keypair_2.private_key(), ed25519_keypair_2.public_key()),
        keypair_2.private_key(),
        ed25519_keypair_1.public_key(),
        keypair_1.public_key(),
        &salt,
    )
    .expect("Test session creation failed");

    (initiator_session, responder_session)
}

/// Generates a deterministic u32 session ID for the Lewes Protocol
/// based on two public keys. The order of the keys does not matter.
///
/// Uses a different internal delimiter than `make_conv_id` to avoid
/// potential collisions if the same key pairs were used in both contexts.
fn make_id(key1_bytes: &[u8], key2_bytes: &[u8], sep: u8) -> u32 {
    let mut hasher = DefaultHasher::new();

    // Ensure consistent order for hashing to make the ID order-independent.
    // This guarantees make_lp_id(a, b) == make_lp_id(b, a).
    if key1_bytes < key2_bytes {
        hasher.write(key1_bytes);
        // Use a delimiter specific to Lewes Protocol ID generation
        // (0xCC chosen arbitrarily, could be any value different from 0xFF)
        hasher.write_u8(sep);
        hasher.write(key2_bytes);
    } else {
        hasher.write(key2_bytes);
        hasher.write_u8(sep);
        hasher.write(key1_bytes);
    }

    // Truncate the u64 hash result to u32
    (hasher.finish() & 0xFFFF_FFFF) as u32
}

pub fn make_lp_id(key1_bytes: &PublicKey, key2_bytes: &PublicKey) -> u32 {
    make_id(key1_bytes.as_bytes(), key2_bytes.as_bytes(), 0xCC)
}

pub fn make_conv_id(src: &[u8], dst: &[u8]) -> u32 {
    make_id(src, dst, 0xFF)
}

#[cfg(test)]
mod tests {
    use crate::keypair::Keypair;
    use crate::message::LpMessage;
    use crate::packet::{LpHeader, LpPacket, TRAILER_LEN};
    use crate::session_manager::SessionManager;
    use crate::{make_lp_id, sessions_for_tests, LpError};
    use bytes::BytesMut;

    // Import the new standalone functions
    use crate::codec::{parse_lp_packet, serialize_lp_packet};

    #[test]
    fn test_replay_protection_integration() {
        // Create session
        let session = sessions_for_tests().0;

        // === Packet 1 (Counter 0 - Should succeed) ===
        let packet1 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                session_id: 42, // Matches session's sending_index assumption for this test
                counter: 0,
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize packet
        let mut buf1 = BytesMut::new();
        serialize_lp_packet(&packet1, &mut buf1).unwrap();

        // Parse packet
        let parsed_packet1 = parse_lp_packet(&buf1).unwrap();

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
                reserved: 0,
                session_id: 42,
                counter: 0, // Same counter as before (replay)
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize packet
        let mut buf2 = BytesMut::new();
        serialize_lp_packet(&packet2, &mut buf2).unwrap();

        // Parse packet
        let parsed_packet2 = parse_lp_packet(&buf2).unwrap();

        // Perform replay check (should fail)
        let replay_result = session.receiving_counter_quick_check(parsed_packet2.header.counter);
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
                reserved: 0,
                session_id: 42,
                counter: 1, // Incremented counter
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize packet
        let mut buf3 = BytesMut::new();
        serialize_lp_packet(&packet3, &mut buf3).unwrap();

        // Parse packet
        let parsed_packet3 = parse_lp_packet(&buf3).unwrap();

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

    #[test]
    fn test_session_manager_integration() {
        use nym_crypto::asymmetric::ed25519;

        // Create session manager
        let local_manager = SessionManager::new();
        let remote_manager = SessionManager::new();
        let local_keypair = Keypair::default();
        let remote_keypair = Keypair::default();
        let lp_id = make_lp_id(local_keypair.public_key(), remote_keypair.public_key());

        // Ed25519 keypairs for PSQ authentication
        let ed25519_keypair_local = ed25519::KeyPair::from_secret([8u8; 32], 0);
        let ed25519_keypair_remote = ed25519::KeyPair::from_secret([9u8; 32], 1);

        // Test salt
        let salt = [46u8; 32];

        // Create a session via manager
        let _ = local_manager
            .create_session_state_machine(
                &local_keypair,
                (ed25519_keypair_local.private_key(), ed25519_keypair_local.public_key()),
                remote_keypair.public_key(),
                ed25519_keypair_remote.public_key(),
                true,
                &salt,
            )
            .unwrap();

        let _ = remote_manager
            .create_session_state_machine(
                &remote_keypair,
                (ed25519_keypair_remote.private_key(), ed25519_keypair_remote.public_key()),
                local_keypair.public_key(),
                ed25519_keypair_local.public_key(),
                false,
                &salt,
            )
            .unwrap();
        // === Packet 1 (Counter 0 - Should succeed) ===
        let packet1 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                session_id: lp_id,
                counter: 0,
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize
        let mut buf1 = BytesMut::new();
        serialize_lp_packet(&packet1, &mut buf1).unwrap();

        // Parse
        let parsed_packet1 = parse_lp_packet(&buf1).unwrap();

        // Process via SessionManager method (which should handle checks + marking)
        // NOTE: We might need a method on SessionManager/LpSession like `process_incoming_packet`
        //       that encapsulates parse -> check -> process_noise -> mark.
        //       For now, we simulate the steps using the retrieved session.

        // Perform replay check
        local_manager
            .receiving_counter_quick_check(lp_id, parsed_packet1.header.counter)
            .expect("Packet 1 check failed");
        // Mark received
        local_manager
            .receiving_counter_mark(lp_id, parsed_packet1.header.counter)
            .expect("Packet 1 mark failed");

        // === Packet 2 (Counter 1 - Should succeed on same session) ===
        let packet2 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                session_id: lp_id,
                counter: 1,
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize
        let mut buf2 = BytesMut::new();
        serialize_lp_packet(&packet2, &mut buf2).unwrap();

        // Parse
        let parsed_packet2 = parse_lp_packet(&buf2).unwrap();

        // Perform replay check
        local_manager
            .receiving_counter_quick_check(lp_id, parsed_packet2.header.counter)
            .expect("Packet 2 check failed");
        // Mark received
        local_manager
            .receiving_counter_mark(lp_id, parsed_packet2.header.counter)
            .expect("Packet 2 mark failed");

        // === Packet 3 (Counter 0 - Replay, should fail check) ===
        let packet3 = LpPacket {
            header: LpHeader {
                protocol_version: 1,
                reserved: 0,
                session_id: lp_id,
                counter: 0, // Replay of first packet
            },
            message: LpMessage::Busy,
            trailer: [0u8; TRAILER_LEN],
        };

        // Serialize
        let mut buf3 = BytesMut::new();
        serialize_lp_packet(&packet3, &mut buf3).unwrap();

        // Parse
        let parsed_packet3 = parse_lp_packet(&buf3).unwrap();

        // Perform replay check (should fail)
        let replay_result =
            local_manager.receiving_counter_quick_check(lp_id, parsed_packet3.header.counter);
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
