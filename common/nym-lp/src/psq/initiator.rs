// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTRequestData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::psq_initiator_create_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp, kem_to_ciphersuite};
use crate::psq::{
    AAD_INITIATOR_INNER_V1, AAD_INITIATOR_OUTER_V1, InitiatorData, MinimalSession,
    PSQHandshakeState, SESSION_CONTEXT_V1, initiator,
};
use crate::session::PqSharedSecret;
use crate::{ClientHelloData, LpError, LpMessage, LpSession};
use libcrux_psq::handshake::RegistrationInitiator;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, InitiatorCiphersuite, PrincipalBuilder,
};
use libcrux_psq::handshake::ciphersuites::CiphersuiteName;
use libcrux_psq::{Channel, IntoSession};
use nym_kkt::context::KKTContext;
use nym_kkt::initiator::KKTInitiator;
use nym_kkt::keys::EncapsulationKey;
use nym_kkt::message::{KKTRequest, KKTResponse};
use nym_kkt_ciphersuite::KEM;
use nym_lp_transport::traits::LpTransport;
use rand09::rng;
use tracing::debug;

pub(crate) struct PSQHandshakeStateInitiator<'a, S> {
    pub(super) inner_state: PSQHandshakeState<'a, S>,
    pub(super) initiator_data: InitiatorData,
}

pub(crate) fn build_psq_principal<R>(
    rng: R,
    version: u8,
    ciphersuite: InitiatorCiphersuite,
) -> Result<RegistrationInitiator<R>, LpError>
where
    R: rand09::CryptoRng,
{
    let (ctx, inner_aad, outer_aad) = match version {
        1 => (
            SESSION_CONTEXT_V1,
            AAD_INITIATOR_INNER_V1,
            AAD_INITIATOR_OUTER_V1,
        ),
        other => return Err(LpError::UnsupportedVersion { version: other }),
    };

    PrincipalBuilder::new(rng)
        .outer_aad(outer_aad)
        .inner_aad(inner_aad)
        .context(ctx)
        .build_registration_initiator(ciphersuite)
        .map_err(|inner| LpError::PSQInitiatorBuilderFailure { inner })
}

pub(crate) fn build_psq_ciphersuite<'a>(
    init: &'a LpLocalPeer,
    responder: &'a LpRemotePeer,
    kem_key: &'a EncapsulationKey,
) -> Result<InitiatorCiphersuite<'a>, LpError> {
    let psq_ciphersuite = kem_to_ciphersuite(kem_key.kem());

    let builder = CiphersuiteBuilder::new(psq_ciphersuite)
        .longterm_x25519_keys(init.x25519())
        .peer_longterm_x25519_pk(responder.x25519());

    match kem_key {
        EncapsulationKey::McEliece(kem_key) => builder.peer_longterm_cmc_pk(kem_key),
        EncapsulationKey::MlKem768(kem_key) => builder.peer_longterm_mlkem_pk(kem_key),
    }
    .build_initiator_ciphersuite()
    .map_err(|inner| LpError::PSQInitiatorBuilderFailure { inner })
}

impl<'a, S> PSQHandshakeStateInitiator<'a, S>
where
    S: LpTransport + Unpin,
{
    fn build_psq_initiator_principal<'b>(
        &'b self,
        encapsulation_key: &'b EncapsulationKey,
    ) -> Result<RegistrationInitiator<'b, rand09::rngs::ThreadRng>, LpError> {
        let initiator_ciphersuite = build_psq_ciphersuite(
            &self.inner_state.local_peer,
            &self.initiator_data.remote_peer,
            &encapsulation_key,
        )?;
        let initiator = build_psq_principal(
            rng(),
            self.initiator_data.protocol_version,
            initiator_ciphersuite,
        )?;
        Ok(initiator)
    }

    /// Attempt to send KKT request to begin the handshake
    async fn send_kkt_request(&mut self, request: KKTRequest) -> Result<(), LpError> {
        // TODO: extra header
        self.inner_state
            .connection
            .send_serialised_packet(&request.into_bytes())
            .await?;
        Ok(())
    }

    /// Attempt to receive a KKT response to the previously sent request
    async fn receive_kkt_response(&mut self) -> Result<KKTResponse, LpError> {
        let data = self.inner_state.connection.receive_raw_packet().await?;
        Ok(KKTResponse::from_bytes(data))
    }

    pub async fn complete_handshake<R>(mut self, rng: &mut R) -> Result<MinimalSession, LpError>
    where
        S: LpTransport + Unpin,
        R: rand09::CryptoRng,
    {
        // 1. retrieve the expected kem key hash. if we don't know it,
        let dir_hash = self
            .initiator_data
            .remote_peer
            .expected_kem_key_hash(self.inner_state.ciphersuite)?;

        // 2. prepare and send KKT request
        let (mut initiator, kkt_request) = KKTInitiator::generate_one_way_request(
            rng,
            self.inner_state.ciphersuite,
            self.initiator_data.remote_peer.x25519(),
            &dir_hash,
            self.initiator_data.protocol_version,
        )?;
        debug!("sending KKT request");
        self.send_kkt_request(kkt_request).await?;

        // 3. receive and process KKT response
        let raw_response = self.receive_kkt_response().await?;
        debug!("received KKT response");
        let response = initiator.process_response(raw_response)?;

        // 4. generate and send PSQ request
        let protocol = self.initiator_data.protocol_version;
        let mut conn = self.inner_state.connection;

        // build the PSQ initiator
        let initiator_ciphersuite = build_psq_ciphersuite(
            &self.inner_state.local_peer,
            &self.initiator_data.remote_peer,
            &response.encapsulation_key,
        )?;

        let mut psq_initiator = build_psq_principal(rng, protocol, initiator_ciphersuite)?;

        // PSQ msg 1 send
        let mut buf = [0u8; 1536];
        // annoyingly `RegistrationInitiator` has to write into unresizable `&mut [u8]`...
        let n = psq_initiator.write_message(&[], &mut buf)?;
        debug!("sending PSQ handshake msg");
        conn.send_serialised_packet(&buf[..n]).await?;

        // 5. receive and process PSQ response
        let psq_msg = conn.receive_raw_packet().await?;
        debug!("received PSQ handshake msg");
        psq_initiator.read_message(&psq_msg, &mut [])?;

        if !psq_initiator.is_handshake_finished() {
            return Err(LpError::kkt_psq_handshake(
                "handshake not finished after receiving psq response",
            ));
        }

        let session = psq_initiator.into_session()?;
        Ok(MinimalSession {
            session,
            encapsulation_key: Some(response.encapsulation_key),
            init_authenticator: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::mock_peers;
    use crate::psq::responder;
    use libcrux_psq::handshake::types::Authenticator;
    use libcrux_psq::session::{Session, SessionBinding};
    use nym_kkt::responder::KKTResponder;
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, SignatureScheme};
    use nym_test_utils::helpers::{DeterministicRng09Send, u64_seeded_rng_09};
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, Timeboxed};

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
        let initiator_data = InitiatorData::new(1, resp_remote);

        let handshake_init =
            PSQHandshakeState::new(conn_init, ciphersuite, init).as_initiator(initiator_data);

        let mut init_rng = DeterministicRng09Send::new(u64_seeded_rng_09(1));

        let init_fut = tokio::spawn(async move {
            handshake_init
                .complete_handshake(&mut init_rng)
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
        responder.read_message(&raw_psq_req, &mut []).unwrap();

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

        let encapsulation_key = session_init.encapsulation_key.unwrap();
        let mut i_transport = session_init.session;
        let mut r_transport = responder.into_session().unwrap();

        // test serialization, deserialization
        let mut msg_channel = vec![0u8; 2048];
        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];

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
