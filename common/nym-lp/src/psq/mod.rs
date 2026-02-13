// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::ErrorPacketData;
use crate::packet::LpHeader;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psq::helpers::LpTransportHandshakeExt;
use crate::{LpError, LpMessage, LpPacket};
use nym_lp_transport::traits::LpTransport;
use tracing::debug;

mod helpers;
mod initiator;
mod responder;

// IMPORT START:
use libcrux_psq::{
    Channel,
    handshake::{
        RegistrationInitiator, Responder,
        builders::{CiphersuiteBuilder, PrincipalBuilder},
        ciphersuites::CiphersuiteName,
        types::{DHKeyPair, DHPublicKey},
    },
};
use nym_kkt_ciphersuite::{Ciphersuite, KEM};
use rand09::rngs::ThreadRng;

use std::fmt::Debug;

pub(crate) const AAD_INITIATOR_OUTER_V1: &[u8] = b"NYM-PQ-AAD-INIT-OUTER-V1";
pub(crate) const AAD_INITIATOR_INNER_V1: &[u8] = b"NYM-PQ-AAD-INIT-INNER-V1";
pub(crate) const AAD_RESPONDER_V1: &[u8] = b"NYM-PQ-AAD-RESP-V1";
pub(crate) const SESSION_CONTEXT_V1: &[u8] = b"NYM-PQ-SESSION-CONTEXT-V1";

pub enum PSQState<'a> {
    Initiator(RegistrationInitiator<'a, ThreadRng>),
    Responder(Responder<'a, ThreadRng>),
}
impl Debug for PSQState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initiator(_) => f.debug_tuple("PSQ Initiator").finish(),
            Self::Responder(_) => f.debug_tuple("PSQ Responder").finish(),
        }
    }
}

pub fn initiator_process(initiator: &mut RegistrationInitiator<ThreadRng>) -> Vec<u8> {
    let mut buffer = vec![0u8; 4096];
    let msg_len = initiator.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}

// pub fn build_initiator<'a>(
//     ciphersuite: &'a Ciphersuite,
//     session_context: &'a [u8],
//     local_x25519_keys: &'a DHKeyPair,
//     remote_x25519_public: &'a DHPublicKey,
//     remote_kem_public: &'a EncapsulationKey,
// ) -> RegistrationInitiator<'a, rand09::rngs::ThreadRng> {
//     //georgio: handle errors
//
//     let initiator_cbuilder = match ciphersuite.kem() {
//         nym_kkt::ciphersuite::KEM::MlKem768 => match remote_kem_public {
//             EncapsulationKey::MlKem768(ml_kem_public_key) => CiphersuiteBuilder::new(
//                 CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
//             )
//             .peer_longterm_mlkem_pk(ml_kem_public_key),
//             _ => panic!(
//                 "wrong key type passed (remote_kem_public should be EncapsulationKey::MlKem768)"
//             ),
//         },
//         nym_kkt::ciphersuite::KEM::McEliece => match remote_kem_public {
//             EncapsulationKey::McEliece(mceliece_public_key) => CiphersuiteBuilder::new(
//                 CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
//             )
//             .peer_longterm_cmc_pk(mceliece_public_key),
//             _ => panic!(
//                 "wrong key type passed (remote_kem_public should be EncapsulationKey::McEliece)"
//             ),
//         },
//         _ => panic!("undefined"),
//     };
//     let initiator_ciphersuite = initiator_cbuilder
//         .longterm_x25519_keys(local_x25519_keys)
//         .peer_longterm_x25519_pk(remote_x25519_public)
//         .build_initiator_ciphersuite()
//         .unwrap();
//
//     PrincipalBuilder::new(rand09::rng())
//         .outer_aad(AAD_INITIATOR_OUTER)
//         .inner_aad(AAD_INITIATOR_INNER)
//         .context(session_context)
//         .build_registration_initiator(initiator_ciphersuite)
//         .unwrap()
// }
//
// // JS: I have removed the `ciphersuite` argument as it was only matching on the key types,
// // which we already obtained matching on the ciphersuite kem type in `LpSession::new`
// pub fn build_responder<'a>(
//     local_x25519_keys: &'a DHKeyPair,
//     local_kem_keys: &'a KemKeyPair,
// ) -> Responder<'a, rand09::rngs::ThreadRng> {
//     let responder_ciphersuite = match local_kem_keys {
//         KemKeyPair::MlKem768 {
//             encapsulation_key,
//             decapsulation_key,
//         } => CiphersuiteBuilder::new(
//             CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
//         )
//         .longterm_mlkem_encapsulation_key(encapsulation_key)
//         .longterm_mlkem_decapsulation_key(decapsulation_key),
//         KemKeyPair::McEliece {
//             encapsulation_key,
//             decapsulation_key,
//         } => CiphersuiteBuilder::new(
//             CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
//         )
//         .longterm_cmc_encapsulation_key(encapsulation_key)
//         .longterm_cmc_decapsulation_key(decapsulation_key),
//         KemKeyPair::XWing { .. } => panic!("unsupported"),
//         KemKeyPair::X25519 { .. } => panic!("unsupported"),
//     }
//     .longterm_x25519_keys(local_x25519_keys)
//     .build_responder_ciphersuite()
//     .unwrap();
//
//     PrincipalBuilder::new(rand09::rng())
//         .outer_aad(AAD_RESPONDER)
//         .context(SESSION_CONTEXT)
//         .build_responder(responder_ciphersuite)
//         .unwrap()
// }

pub fn psq_responder_process<'a>(
    responder: &'a mut Responder<ThreadRng>,
    initiator_message: &[u8],
) -> Vec<u8> {
    let mut payload = vec![0u8; 4096];
    responder
        .read_message(initiator_message, &mut payload)
        .unwrap();

    let mut buffer = vec![0u8; 4096];
    let msg_len = responder.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}
// IMPORT END

pub(crate) struct IntermediateHandshakeFailure {
    /// Session id established during exchange if we managed to derive it
    session_id: Option<u32>,

    /// Protocol version established during the exchange
    protocol_version: Option<u8>,

    /// Outer aead key established during exchange if we managed to derive it
    outer_aead_key: Option<OuterAeadKey>,

    /// The error source
    source: LpError,
}

impl IntermediateHandshakeFailure {
    fn plain(source: LpError) -> IntermediateHandshakeFailure {
        IntermediateHandshakeFailure {
            session_id: None,
            protocol_version: None,
            outer_aead_key: None,
            source,
        }
    }
}

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

    pub(crate) async fn try_send_error_packet(
        &mut self,
        err: IntermediateHandshakeFailure,
    ) -> LpError {
        // if session_id is not known, we can't send the packet back (with the current design)
        let (Some(session_id), Some(protocol)) = (err.session_id, err.protocol_version) else {
            return err.source;
        };
        if let Err(err) = self
            .send_error_packet(
                session_id,
                protocol,
                err.source.to_string(),
                err.outer_aead_key.as_ref(),
            )
            .await
        {
            debug!("failed to send back error response: {err}")
        }
        err.source
    }

    /// Attempt to send an error packet
    pub(crate) async fn send_error_packet(
        &mut self,
        session_id: u32,
        protocol_version: u8,
        msg: impl Into<String>,
        outer_aead_key: Option<&OuterAeadKey>,
    ) -> Result<(), LpError> {
        let packet = self.next_packet(
            session_id,
            protocol_version,
            LpMessage::Error(ErrorPacketData::new(msg)),
        );
        self.connection.send_packet(packet, outer_aead_key).await?;
        Ok(())
    }

    /// Attempt to receive a packet from connection, explicitly checking for an error response
    /// and returning corresponding message if received
    pub(crate) async fn receive_non_error(
        &mut self,
        outer_aead_key: Option<&OuterAeadKey>,
    ) -> Result<LpPacket, LpError> {
        let packet = self.connection.receive_packet(outer_aead_key).await?;

        match &packet.message {
            LpMessage::Error(error_packet) => Err(LpError::kkt_psq_handshake(format!(
                "remote error: {}",
                error_packet.message
            ))),
            _ => Ok(packet),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::mock_peers;
    use crate::psq::helpers::LpTransportHandshakeExt;
    use crate::psq::responder::DEFAULT_TIMESTAMP_TOLERANCE;
    use libcrux_psq::IntoSession;
    use libcrux_psq::handshake::types::{Authenticator, PQEncapsulationKey};
    use libcrux_psq::session::{Session, SessionBinding};
    use mock_instant::thread_local::MockClock;
    use nym_kkt::initiator::KKTInitiator;
    use nym_kkt::key_utils::{
        generate_keypair_mceliece, generate_keypair_mlkem, generate_keypair_x25519,
        hash_encapsulation_key,
    };
    use nym_kkt::keys::EncapsulationKey;
    use nym_kkt::responder::KKTResponder;
    use nym_kkt_ciphersuite::{HashFunction, HashLength, SignatureScheme};
    use nym_test_utils::helpers::deterministic_rng_09;
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, TimeboxedSpawnable};
    use std::time::Duration;
    use tokio::join;

    #[allow(dead_code)]
    async fn extract_error(conn: &mut MockIOStream) -> String {
        let packet = conn.receive_packet(None).await.unwrap();
        match packet.message {
            LpMessage::Error(error) => error.message,
            _ => panic!("non error packet"),
        }
    }

    #[tokio::test]
    async fn e2e_psq_handshake() -> anyhow::Result<()> {
        todo!()
        // let conn_init = MockIOStream::default();
        // let conn_resp = conn_init.try_get_remote_handle();
        //
        // // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
        // // so they'd get 'static lifetime
        // let conn_init = conn_init.leak();
        // let conn_resp = conn_resp.leak();
        //
        // let ciphersuite = Ciphersuite::new(
        //     KEM::X25519,
        //     HashFunction::Blake3,
        //     SignatureScheme::Ed25519,
        //     HashLength::Default,
        // );
        //
        // let (init, resp) = mock_peers();
        // let resp_remote = resp.as_remote();
        //
        // let handshake_init = PSQHandshakeState::new(conn_init, ciphersuite, init)
        //     .with_protocol_version(1)
        //     .with_remote_peer(resp_remote);
        // let handshake_resp = PSQHandshakeState::new(conn_resp, ciphersuite, resp);
        //
        // let resp_fut = handshake_resp.complete_as_responder().spawn_timeboxed();
        // let init_fut = handshake_init.complete_as_initiator().spawn_timeboxed();
        //
        // let (session_init, session_resp) = join!(init_fut, resp_fut);
        //
        // let session_init = session_init???;
        // let session_resp = session_resp???;
        //
        // assert_eq!(session_init.id(), session_resp.id());
        // assert_eq!(
        //     session_init.outer_aead_key().as_bytes(),
        //     session_resp.outer_aead_key().as_bytes()
        // );
        // assert_eq!(
        //     session_init.pq_shared_secret().as_bytes(),
        //     session_resp.pq_shared_secret().as_bytes()
        // );
        //
        // Ok(())
    }

    #[tokio::test]
    async fn preparing_client_hello_initiator() -> anyhow::Result<()> {
        todo!()
        // let mut conn_init = MockIOStream::default();
        // let mut conn_resp = conn_init.try_get_remote_handle();
        //
        // let ciphersuite = Ciphersuite::new(
        //     KEM::X25519,
        //     HashFunction::Blake3,
        //     SignatureScheme::Ed25519,
        //     HashLength::Default,
        // );
        // let (init, resp) = mock_peers();
        // let resp_remote = resp.as_remote();
        //
        // // as initiator
        // let mut handshake_init = PSQHandshakeState::new(&mut conn_init, ciphersuite, init)
        //     .with_protocol_version(1)
        //     .with_remote_peer(resp_remote);
        //
        // // you can generate and send (valid) client hello as initiator
        // let client_hello = handshake_init.send_client_hello().await?;
        // let LpMessage::ClientHello(received_client_hello) =
        //     conn_resp.receive_packet(None).await?.message
        // else {
        //     panic!("wrong message type");
        // };
        // assert_eq!(client_hello, received_client_hello);
        // Ok(())
    }

    // essentially make sure you can't accidentally trigger the handshake as the responder
    #[tokio::test]
    async fn preparing_client_hello_responder() -> anyhow::Result<()> {
        todo!()
        // let conn_init = MockIOStream::default();
        // let mut conn_resp = conn_init.try_get_remote_handle();
        //
        // let ciphersuite = Ciphersuite::new(
        //     KEM::X25519,
        //     HashFunction::Blake3,
        //     SignatureScheme::Ed25519,
        //     HashLength::Default,
        // );
        // let (_, resp) = mock_peers();
        //
        // // as initiator
        // let mut handshake_resp = PSQHandshakeState::new(&mut conn_resp, ciphersuite, resp);
        //
        // // you can generate and send (valid) client hello as initiator
        // let sending_res = handshake_resp.send_client_hello().await;
        // assert!(sending_res.is_err());
        // Ok(())
    }

    #[tokio::test]
    async fn test_receive_client_hello_timestamp_too_skewed() -> anyhow::Result<()> {
        todo!()
        // let current_time = Duration::from_secs(10000);
        // MockClock::set_system_time(current_time);
        //
        // let too_old = current_time - DEFAULT_TIMESTAMP_TOLERANCE - Duration::from_secs(1);
        // let too_recent = current_time + DEFAULT_TIMESTAMP_TOLERANCE + Duration::from_secs(1);
        //
        // let ciphersuite = Ciphersuite::new(
        //     KEM::X25519,
        //     HashFunction::Blake3,
        //     SignatureScheme::Ed25519,
        //     HashLength::Default,
        // );
        //
        // // TOO OLD
        // let mut conn_init = MockIOStream::default();
        // let mut conn_resp = conn_init.try_get_remote_handle();
        // let (init, resp) = mock_peers();
        //
        // let mut handshake_resp = PSQHandshakeState::new(&mut conn_resp, ciphersuite, resp);
        // let client_hello_too_old = init.build_client_hello_data(too_old.as_secs());
        //
        // conn_init
        //     .send_packet(client_hello_too_old.into_lp_packet(1), None)
        //     .await?;
        // let err = handshake_resp.receive_client_hello().await.unwrap_err();
        // assert!(err.to_string().contains("too old"));
        //
        // // TOO RECENT
        // let mut conn_init = MockIOStream::default();
        // let mut conn_resp = conn_init.try_get_remote_handle();
        // let (init, resp) = mock_peers();
        //
        // let mut handshake_resp = PSQHandshakeState::new(&mut conn_resp, ciphersuite, resp);
        // let client_hello_too_recent = init.build_client_hello_data(too_recent.as_secs());
        //
        // conn_init
        //     .send_packet(client_hello_too_recent.into_lp_packet(1), None)
        //     .await?;
        // let err = handshake_resp.receive_client_hello().await.unwrap_err();
        //
        // assert!(err.to_string().contains("too future"));
        // Ok(())
    }

    // plain test without any wrappers
    #[test]
    fn e2e_test_plain() {
        let mut rng = deterministic_rng_09();

        let kem = KEM::MlKem768;
        let protocol_version = 1;
        let (mut init, resp) = mock_peers();

        init.ciphersuite = Ciphersuite::default().with_kem(kem);
        let resp_remote = resp.as_remote();
        let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite).unwrap();

        let resp_keys = resp.kem_keypairs.as_ref().unwrap();

        // generate responder x25519 keys
        let responder_x25519_keypair = resp.x25519();
        let hash_function = HashFunction::Blake3;
        // generate kem public keys

        let kkt_responder = KKTResponder::new(
            &responder_x25519_keypair,
            &resp_keys,
            &[
                HashFunction::Blake3,
                HashFunction::SHA256,
                HashFunction::Shake128,
                HashFunction::Shake256,
            ],
            &[protocol_version],
            &[SignatureScheme::Ed25519],
        )
        .unwrap();

        // OneWay - MlKem
        let psq_ciphersuite = CiphersuiteName::X25519_MLKEM768_X25519_AESGCM128_HKDFSHA256;

        let ciphersuite = Ciphersuite::resolve_ciphersuite(
            KEM::MlKem768,
            hash_function,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap();

        let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
            &mut rng,
            &ciphersuite,
            &responder_x25519_keypair.pk,
            &dir_hash,
            protocol_version,
        )
        .unwrap();

        let (response_bytes, _) = kkt_responder.process_request(&request_bytes).unwrap();

        let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

        assert_eq!(
            i_obtained_key,
            resp_keys
                .ml_kem768_encapsulation_key()
                .as_slice()
                .as_slice(),
        );

        let mlkem_key =
            libcrux_kem::MlKem768PublicKey::try_from(i_obtained_key.as_slice()).unwrap();

        let encapsulation_key = EncapsulationKey::MlKem768(mlkem_key);

        let mut msg_channel = vec![0u8; 8192];
        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];

        let initiator_ciphersuite =
            initiator::build_psq_ciphersuite(&init, &resp_remote, &encapsulation_key).unwrap();
        let mut initiator =
            initiator::build_psq_principal(rand09::rng(), protocol_version, initiator_ciphersuite)
                .unwrap();

        let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem).unwrap();
        let mut responder =
            responder::build_psq_principal(rand09::rng(), protocol_version, responder_ciphersuite)
                .unwrap();

        // Send first message
        let registration_payload_initiator = b"Registration_init";
        let len_i = initiator
            .write_message(registration_payload_initiator, &mut msg_channel)
            .unwrap();

        // Read first message
        let (len_r_deserialized, len_r_payload) = responder
            .read_message(&msg_channel, &mut payload_buf_responder)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r_deserialized, len_i);
        assert_eq!(len_r_payload, registration_payload_initiator.len());
        assert_eq!(
            &payload_buf_responder[0..len_r_payload],
            registration_payload_initiator
        );

        // Get the authenticator out here, so we can deserialize the session later.
        let Some(initiator_authenticator) = responder.initiator_authenticator() else {
            panic!("No initiator authenticator found")
        };

        // Respond
        let registration_payload_responder = b"Registration_respond";
        let len_r = responder
            .write_message(registration_payload_responder, &mut msg_channel)
            .unwrap();

        // Finalize on registration initiator
        let (len_i_deserialized, len_i_payload) = initiator
            .read_message(&msg_channel, &mut payload_buf_initiator)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r, len_i_deserialized);
        assert_eq!(registration_payload_responder.len(), len_i_payload);
        assert_eq!(
            &payload_buf_initiator[0..len_i_payload],
            registration_payload_responder
        );

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

        let len_i = channel_i
            .write_message(app_data_i, &mut msg_channel)
            .unwrap();

        let (len_r_deserialized, len_r_payload) = channel_r
            .read_message(&msg_channel, &mut payload_buf_responder)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r_deserialized, len_i);
        assert_eq!(len_r_payload, app_data_i.len());
        assert_eq!(&payload_buf_responder[0..len_r_payload], app_data_i);

        let len_r = channel_r
            .write_message(app_data_r, &mut msg_channel)
            .unwrap();

        let (len_i_deserialized, len_i_payload) = channel_i
            .read_message(&msg_channel, &mut payload_buf_initiator)
            .unwrap();

        assert_eq!(len_r, len_i_deserialized);
        assert_eq!(app_data_r.len(), len_i_payload);
        assert_eq!(&payload_buf_initiator[0..len_i_payload], app_data_r);
    }
}
