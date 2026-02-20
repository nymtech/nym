// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::peer::{LpLocalPeer, LpRemotePeer};
use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, IntoEnumIterator, KEM, SignatureScheme};
use nym_lp_packet::version;
use nym_lp_transport::traits::LpHandshakeChannel;

pub(crate) mod handshake_message;
mod helpers;
pub mod initiator;
pub mod responder;

pub use initiator::PSQHandshakeStateInitiator;
pub use responder::PSQHandshakeStateResponder;

pub(crate) const AAD_INITIATOR_OUTER_V1: &[u8] = b"NYM-PQ-AAD-INIT-OUTER-V1";
pub(crate) const AAD_INITIATOR_INNER_V1: &[u8] = b"NYM-PQ-AAD-INIT-INNER-V1";
pub(crate) const AAD_RESPONDER_V1: &[u8] = b"NYM-PQ-AAD-RESP-V1";
pub(crate) const SESSION_CONTEXT_V1: &[u8] = b"NYM-PQ-SESSION-CONTEXT-V1";

/// Size of the first (initiator) PSQ message including all serialisation overheads if no additional payload has been attached
pub(crate) fn psq_msg1_size(kem: KEM) -> usize {
    match kem {
        KEM::MlKem768 => 1247,
        KEM::McEliece => 315,
    }
}

/// Size of the second (responder) PSQ message including all serialisation overheads if no additional payload has been attached
pub(crate) const PSQ_MSG2_SIZE: usize = 70;

pub struct PSQHandshakeState<'a, S> {
    /// The underlying connection established for the handshake
    connection: &'a mut S,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,
}

#[derive(Debug)]
pub struct InitiatorData {
    /// Protocol version used for the exchange known implicitly through the directory
    pub protocol_version: u8,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    pub remote_peer: LpRemotePeer,
}

impl InitiatorData {
    pub fn new(protocol_version: u8, remote_peer: LpRemotePeer) -> Self {
        InitiatorData {
            protocol_version,
            remote_peer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponderData {
    /// List of supported Hash Functions by this Responder
    pub supported_hash_functions: Vec<HashFunction>,

    /// List of supported Signature Schemes by this Responder
    pub supported_signature_schemes: Vec<SignatureScheme>,

    /// List of supported outer (LP) protocol version by this Responder
    pub supported_outer_protocol_versions: Vec<u8>,
}

impl Default for ResponderData {
    fn default() -> Self {
        // by default all schemes are supported
        ResponderData {
            supported_hash_functions: HashFunction::iter().collect(),
            supported_signature_schemes: SignatureScheme::iter().collect(),
            supported_outer_protocol_versions: vec![version::CURRENT],
        }
    }
}

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpHandshakeChannel + Unpin,
{
    pub fn new(connection: &'a mut S, local_peer: LpLocalPeer) -> Self {
        PSQHandshakeState {
            connection,
            local_peer,
        }
    }

    pub fn as_initiator(self, initiator_data: InitiatorData) -> PSQHandshakeStateInitiator<'a, S> {
        PSQHandshakeStateInitiator {
            initiator_data,
            inner_state: self,
        }
    }

    pub fn as_responder(self, responder_data: ResponderData) -> PSQHandshakeStateResponder<'a, S> {
        PSQHandshakeStateResponder {
            responder_data,
            inner_state: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{decrypt_data, encrypt_data};
    use crate::peer::mock_peers;
    use libcrux_psq::handshake::types::Authenticator;
    use libcrux_psq::session::{Session, SessionBinding};
    use libcrux_psq::{Channel, IntoSession};
    use nym_kkt::initiator::KKTInitiator;
    use nym_kkt::message::KKTRequest;
    use nym_kkt::responder::KKTResponder;
    use nym_kkt_ciphersuite::{HashFunction, KEM, SignatureScheme};
    use nym_test_utils::helpers::{
        DeterministicRng09Send, deterministic_rng_09, u64_seeded_rng_09,
    };
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, TimeboxedSpawnable};
    use tokio::join;

    #[tokio::test]
    async fn e2e_psq_handshake() -> anyhow::Result<()> {
        for kem in KEM::iter() {
            let conn_init = MockIOStream::default();
            let conn_resp = conn_init.try_get_remote_handle();

            // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
            // so they'd get 'static lifetime
            let conn_init = conn_init.leak();
            let conn_resp = conn_resp.leak();
            let ciphersuite = Ciphersuite::default().with_kem(kem);

            let (mut init, mut resp) = mock_peers();
            init.ciphersuite = ciphersuite;
            resp.ciphersuite = ciphersuite;
            let resp_remote = resp.as_remote();

            let handshake_init = PSQHandshakeState::new(conn_init, init)
                .as_initiator(InitiatorData::new(1, resp_remote));
            let handshake_resp =
                PSQHandshakeState::new(conn_resp, resp).as_responder(ResponderData::default());

            let init_rng = DeterministicRng09Send::new(u64_seeded_rng_09(1));
            let resp_rng = DeterministicRng09Send::new(u64_seeded_rng_09(2));

            // similarly leak the rngs to get the static lifetimes
            let init_rng = init_rng.leak();
            let resp_rng = resp_rng.leak();

            let init_fut = handshake_init
                .complete_handshake_with_rng(init_rng)
                .spawn_timeboxed();
            let resp_fut = handshake_resp
                .complete_handshake_with_rng(resp_rng)
                .spawn_timeboxed();

            let (session_init, session_resp) = join!(init_fut, resp_fut);

            let mut session_init = session_init???;
            let mut session_resp = session_resp???;

            assert_eq!(
                session_init.session_identifier(),
                session_resp.session_identifier()
            );

            // test serialization, deserialization
            let mut channel_i = session_init.active_transport();
            let mut channel_r = session_resp.active_transport();

            assert_eq!(channel_i.identifier(), channel_r.identifier());

            let app_data_i = b"Derived session hey".as_slice();
            let app_data_r = b"Derived session ho".as_slice();

            let ct_i = encrypt_data(app_data_i, &mut channel_i)?;
            let pt_r = decrypt_data(&ct_i, &mut channel_r)?;

            assert_eq!(app_data_i, pt_r);

            let ct_r = encrypt_data(app_data_r, &mut channel_r)?;
            let pt_i = decrypt_data(&ct_r, &mut channel_i)?;

            assert_eq!(app_data_r, pt_i);
        }

        Ok(())
    }

    // plain test without any wrappers
    #[test]
    fn e2e_test_plain() {
        let mut rng = deterministic_rng_09();

        for kem in KEM::iter() {
            // SETUP START:
            let protocol_version = 1;
            let (mut init, resp) = mock_peers();
            init.ciphersuite = Ciphersuite::default().with_kem(kem);
            let resp_remote = resp.as_remote();
            let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite).unwrap();

            let resp_keys = resp.kem_keypairs.as_ref().unwrap();
            let responder_x25519_keypair = resp.x25519();

            let supported_sigs = [SignatureScheme::Ed25519];
            let supported_hash = [
                HashFunction::Blake3,
                HashFunction::Shake256,
                HashFunction::Shake128,
                HashFunction::SHA256,
            ];
            let kkt_responder = KKTResponder::new(
                &responder_x25519_keypair,
                &resp_keys,
                &supported_hash,
                &supported_sigs,
                &[protocol_version],
            )
            .unwrap();

            // SETUP END

            // OneWay - MlKem
            let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                &mut rng,
                init.ciphersuite,
                &responder_x25519_keypair.pk,
                &dir_hash,
                protocol_version,
            )
            .unwrap();

            let processed_req = kkt_responder.process_request(request).unwrap();

            let response = initiator.process_response(processed_req.response).unwrap();
            let encapsulation_key = response.encapsulation_key;

            let mut payload_buf_responder = vec![0u8; 4096];
            let mut payload_buf_initiator = vec![0u8; 4096];

            let initiator_ciphersuite =
                initiator::build_psq_ciphersuite(&init, &resp_remote, &encapsulation_key).unwrap();
            let mut initiator = initiator::build_psq_principal(
                rand09::rng(),
                protocol_version,
                initiator_ciphersuite,
            )
            .unwrap();

            let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem).unwrap();
            let mut responder = responder::build_psq_principal(
                rand09::rng(),
                protocol_version,
                responder_ciphersuite,
            )
            .unwrap();

            // Send first message
            let mut buf = vec![0u8; psq_msg1_size(kem)];
            let len_i = initiator.write_message(&[], &mut buf).unwrap();
            assert_eq!(len_i, buf.len());

            // Read first message
            let (len_r_deserialized, len_r_payload) = responder
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
            let (len_i_deserialized, len_i_payload) = initiator
                .read_message(&buf, &mut payload_buf_initiator)
                .unwrap();

            // We read the same amount of data.
            assert_eq!(len_r, len_i_deserialized);

            // Ready for transport mode
            assert!(initiator.is_handshake_finished());
            assert!(responder.is_handshake_finished());

            let i_transport = initiator.into_session().unwrap();
            let r_transport = responder.into_session().unwrap();

            // test serialization, deserialization
            let mut session_storage = vec![0u8; 4096];
            i_transport
                .serialize(
                    &mut session_storage,
                    SessionBinding {
                        initiator_authenticator: &Authenticator::Dh(init.x25519().pk),
                        responder_ecdh_pk: &responder_x25519_keypair.pk,
                        responder_pq_pk: Some(encapsulation_key.as_pq_encapsulation_key()),
                    },
                )
                .unwrap();
            let mut i_transport = Session::deserialize(
                &session_storage,
                SessionBinding {
                    initiator_authenticator: &Authenticator::Dh(init.x25519().pk),
                    responder_ecdh_pk: &responder_x25519_keypair.pk,
                    responder_pq_pk: Some(encapsulation_key.as_pq_encapsulation_key()),
                },
            )
            .unwrap();

            r_transport
                .serialize(
                    &mut session_storage,
                    SessionBinding {
                        initiator_authenticator: &initiator_authenticator,
                        responder_ecdh_pk: &responder_x25519_keypair.pk,
                        responder_pq_pk: Some(encapsulation_key.as_pq_encapsulation_key()),
                    },
                )
                .unwrap();
            let mut r_transport = Session::deserialize(
                &session_storage,
                SessionBinding {
                    initiator_authenticator: &initiator_authenticator,
                    responder_ecdh_pk: &responder_x25519_keypair.pk,
                    responder_pq_pk: Some(encapsulation_key.as_pq_encapsulation_key()),
                },
            )
            .unwrap();

            let mut channel_i = i_transport.transport_channel().unwrap();
            let mut channel_r = r_transport.transport_channel().unwrap();

            assert_eq!(channel_i.identifier(), channel_r.identifier());

            let app_data_i = b"Derived session hey".as_slice();
            let app_data_r = b"Derived session ho".as_slice();

            let ct_i = encrypt_data(app_data_i, &mut channel_i).unwrap();
            let pt_r = decrypt_data(&ct_i, &mut channel_r).unwrap();

            assert_eq!(app_data_i, pt_r);

            let ct_r = encrypt_data(app_data_r, &mut channel_r).unwrap();
            let pt_i = decrypt_data(&ct_r, &mut channel_i).unwrap();

            assert_eq!(app_data_r, pt_i);
        }
    }
}
