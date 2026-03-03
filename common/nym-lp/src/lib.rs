// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod codec;
pub mod error;
pub mod packet;
pub mod peer;
pub mod peer_config;
pub mod psq;
pub mod replay;
pub mod session;
mod session_integration;
pub mod session_manager;
pub mod state_machine;
pub mod transport;

pub use error::LpError;
pub use nym_kkt_ciphersuite::{
    Ciphersuite, HashFunction, HashLength, KEM, KEMKeyDigests, SignatureScheme,
};

#[cfg(any(feature = "mock", test))]
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
        use crate::peer_config::LpReceiverIndex;
        use rand09::Rng;

        let (init, resp) = mock_peers();
        let resp_remote = resp.as_remote();

        let mut init_rng = u64_seeded_rng_09(seed);
        let resp_rng = u64_seeded_rng_09(seed + 1);

        let receiver_index: LpReceiverIndex = init_rng.random();

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
            initiator_pq_pk: None,
        };

        SessionsMock {
            initiator: LpSession::new(
                initiator.into_session().unwrap(),
                binding.clone(),
                receiver_index,
                1,
            )
            .unwrap(),
            responder: LpSession::new(
                responder.into_session().unwrap(),
                binding,
                receiver_index,
                1,
            )
            .unwrap(),
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
