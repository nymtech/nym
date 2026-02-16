// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psq::helpers::LpTransportHandshakeExt;
use crate::{LpError, LpMessage};
use libcrux_psq::session::Session;
use nym_kkt::keys::EncapsulationKey;
use nym_kkt_ciphersuite::Ciphersuite;
use nym_lp_transport::traits::LpTransport;

mod helpers;
mod initiator;
mod responder;

pub(crate) const AAD_INITIATOR_OUTER_V1: &[u8] = b"NYM-PQ-AAD-INIT-OUTER-V1";
pub(crate) const AAD_INITIATOR_INNER_V1: &[u8] = b"NYM-PQ-AAD-INIT-INNER-V1";
pub(crate) const AAD_RESPONDER_V1: &[u8] = b"NYM-PQ-AAD-RESP-V1";
pub(crate) const SESSION_CONTEXT_V1: &[u8] = b"NYM-PQ-SESSION-CONTEXT-V1";

pub struct MinimalSession {
    session: Session,
    encapsulation_key: Option<EncapsulationKey>,
}

#[deprecated]
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
    /// or established through KKTRequest (responder)
    protocol_version: Option<u8>,

    /// Ciphersuite selected for the KKT/PSQ exchange
    ciphersuite: Ciphersuite,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: Option<LpRemotePeer>,
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

    fn remote_peer(&self) -> Result<&LpRemotePeer, LpError> {
        self.remote_peer
            .as_ref()
            .ok_or(LpError::MissingRemotePeerInformation)
    }

    //
    // pub fn next_packet(
    //     &mut self,
    //     session_id: u32,
    //     protocol_version: u8,
    //     message: LpMessage,
    // ) -> LpPacket {
    //     let counter = self.next_counter();
    //     let header = LpHeader::new(session_id, counter, protocol_version);
    //     LpPacket::new(header, message)
    // }
    //
    // pub(crate) async fn try_send_error_packet(
    //     &mut self,
    //     err: IntermediateHandshakeFailure,
    // ) -> LpError {
    //     // if session_id is not known, we can't send the packet back (with the current design)
    //     let (Some(session_id), Some(protocol)) = (err.session_id, err.protocol_version) else {
    //         return err.source;
    //     };
    //     if let Err(err) = self
    //         .send_error_packet(
    //             session_id,
    //             protocol,
    //             err.source.to_string(),
    //             err.outer_aead_key.as_ref(),
    //         )
    //         .await
    //     {
    //         debug!("failed to send back error response: {err}")
    //     }
    //     err.source
    // }
    //
    // /// Attempt to send an error packet
    // pub(crate) async fn send_error_packet(
    //     &mut self,
    //     session_id: u32,
    //     protocol_version: u8,
    //     msg: impl Into<String>,
    //     outer_aead_key: Option<&OuterAeadKey>,
    // ) -> Result<(), LpError> {
    //     let packet = self.next_packet(
    //         session_id,
    //         protocol_version,
    //         LpMessage::Error(ErrorPacketData::new(msg)),
    //     );
    //     self.connection.send_packet(packet, outer_aead_key).await?;
    //     Ok(())
    // }
    //
    // /// Attempt to receive a packet from connection, explicitly checking for an error response
    // /// and returning corresponding message if received
    // pub(crate) async fn receive_non_error(
    //     &mut self,
    //     outer_aead_key: Option<&OuterAeadKey>,
    // ) -> Result<LpPacket, LpError> {
    //     let packet = self.connection.receive_packet(outer_aead_key).await?;
    //
    //     match &packet.message {
    //         LpMessage::Error(error_packet) => Err(LpError::kkt_psq_handshake(format!(
    //             "remote error: {}",
    //             error_packet.message
    //         ))),
    //         _ => Ok(packet),
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::mock_peers;
    use crate::psq::helpers::LpTransportHandshakeExt;
    use crate::psq::responder::DEFAULT_TIMESTAMP_TOLERANCE;
    use libcrux_psq::handshake::types::{Authenticator, PQEncapsulationKey};
    use libcrux_psq::session::{Session, SessionBinding};
    use libcrux_psq::{Channel, IntoSession};
    use mock_instant::thread_local::MockClock;
    use nym_kkt::initiator::KKTInitiator;
    use nym_kkt::key_utils::{
        generate_keypair_mceliece, generate_keypair_mlkem, generate_keypair_x25519,
        hash_encapsulation_key,
    };
    use nym_kkt::keys::EncapsulationKey;
    use nym_kkt::message::KKTRequest;
    use nym_kkt::responder::KKTResponder;
    use nym_kkt_ciphersuite::{HashFunction, HashLength, KEM, SignatureScheme};
    use nym_test_utils::helpers::{
        DeterministicRng09Send, deterministic_rng_09, u64_seeded_rng_09,
    };
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, Timeboxed, TimeboxedSpawnable};
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

        // SETUP START:
        let kem = KEM::MlKem768;
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

        let mut msg_channel = vec![0u8; 2048];
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
        let len_i = initiator.write_message(&[], &mut msg_channel).unwrap();

        // Read first message
        let (len_r_deserialized, len_r_payload) = responder
            .read_message(&msg_channel, &mut payload_buf_responder)
            .unwrap();

        // Get the authenticator out here, so we can deserialize the session later.
        let Some(initiator_authenticator) = responder.initiator_authenticator() else {
            panic!("No initiator authenticator found")
        };

        // Respond
        let len_r = responder.write_message(&[], &mut msg_channel).unwrap();

        // Finalize on registration initiator
        let (len_i_deserialized, len_i_payload) = initiator
            .read_message(&msg_channel, &mut payload_buf_initiator)
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

    #[tokio::test]
    async fn initiator_test_plain() -> anyhow::Result<()> {
        let conn_init = MockIOStream::default();
        let conn_resp = conn_init.try_get_remote_handle();

        // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
        // so they'd get 'static lifetime
        let conn_init = conn_init.leak();
        let conn_resp = conn_resp.leak();

        let (init, resp) = mock_peers();
        let init_remote = init.as_remote();
        let resp_remote = resp.as_remote();

        let kem = KEM::MlKem768;
        let ciphersuite = Ciphersuite::default().with_kem(kem);

        let handshake_init = PSQHandshakeState::new(conn_init, ciphersuite, init)
            .with_protocol_version(1)
            .with_remote_peer(resp_remote);

        let mut init_rng = DeterministicRng09Send::new(u64_seeded_rng_09(1));

        let init_fut = tokio::spawn(async move {
            handshake_init
                .complete_as_initiator_inner(&mut init_rng)
                .timeboxed()
                .await
        });

        // responder:
        let supported_sigs = [SignatureScheme::Ed25519];
        let supported_hash = [
            HashFunction::Blake3,
            HashFunction::Shake256,
            HashFunction::Shake128,
            HashFunction::SHA256,
        ];
        let resp_keys = resp.kem_keypairs.as_ref().unwrap();
        let responder_x25519_keypair = resp.x25519();

        let kkt_responder = KKTResponder::new(
            &responder_x25519_keypair,
            &resp_keys,
            &supported_hash,
            &supported_sigs,
            &[1],
        )?;

        // 1. read KKT request
        let raw_kkt_req = conn_resp.receive_raw_packet().timeboxed().await??;
        let req = KKTRequest::try_from_bytes(&raw_kkt_req)?;

        // 2. process
        let processed_req = kkt_responder.process_request(req)?;
        conn_resp
            .send_serialised_packet(&processed_req.response.into_bytes())
            .timeboxed()
            .await??;

        // 3. read PSQ req
        let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem)?;
        let mut responder =
            responder::build_psq_principal(rand09::rng(), 1, responder_ciphersuite)?;

        let raw_psq_req = conn_resp.receive_raw_packet().timeboxed().await??;
        let mut buf = [0u8; 2048];
        responder.read_message(&raw_psq_req, &mut buf).unwrap();

        // Get the authenticator out here, so we can deserialize the session later.
        let Some(initiator_authenticator) = responder.initiator_authenticator() else {
            panic!("No initiator authenticator found")
        };

        // 4 send PSQ response
        let mut buf = [0u8; 2048];
        let n = responder.write_message(&[], &mut buf).unwrap();
        conn_resp
            .send_serialised_packet(&buf[..n])
            .timeboxed()
            .await??;

        assert!(responder.is_handshake_finished());

        let session_init = init_fut.await???;

        let i_transport = session_init.session;
        let encapsulation_key = session_init.encapsulation_key.unwrap();
        let r_transport = responder.into_session().unwrap();

        // test serialization, deserialization
        let mut msg_channel = vec![0u8; 2048];
        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];
        let mut session_storage = vec![0u8; 4096];
        i_transport
            .serialize(
                &mut session_storage,
                SessionBinding {
                    initiator_authenticator: &Authenticator::Dh(init_remote.x25519_public),
                    responder_ecdh_pk: &responder_x25519_keypair.pk,
                    responder_pq_pk: Some(encapsulation_key.as_pq_encapsulation_key()),
                },
            )
            .unwrap();
        let mut i_transport = Session::deserialize(
            &session_storage,
            SessionBinding {
                initiator_authenticator: &Authenticator::Dh(init_remote.x25519_public),
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
        Ok(())
    }
}
