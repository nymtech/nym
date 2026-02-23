// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psq::handshake_message::{PSQMsg1, PSQMsg2};
use crate::psq::helpers::kem_to_ciphersuite;
use crate::psq::{
    AAD_INITIATOR_INNER_V1, AAD_INITIATOR_OUTER_V1, InitiatorData, PSQ_MSG2_SIZE,
    PSQHandshakeState, SESSION_CONTEXT_V1, handshake_message, psq_msg1_size,
};
use crate::session::PersistentSessionBinding;
use crate::{LpError, LpSession};
use libcrux_psq::handshake::RegistrationInitiator;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, InitiatorCiphersuite, PrincipalBuilder,
};
use libcrux_psq::handshake::types::Authenticator;
use libcrux_psq::{Channel, IntoSession};
use nym_kkt::initiator::KKTInitiator;
use nym_kkt::keys::EncapsulationKey;
use nym_kkt::message::{KKTRequest, KKTResponse};
use nym_lp_transport::traits::LpHandshakeChannel;
use rand09::SeedableRng;
use tracing::debug;

pub struct PSQHandshakeStateInitiator<'a, S> {
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
    S: LpHandshakeChannel + Unpin,
{
    /// Attempt to send KKT request to begin the handshake
    async fn send_kkt_request(&mut self, request: KKTRequest) -> Result<(), LpError> {
        let kem = self.inner_state.local_peer.ciphersuite.kem();

        self.inner_state
            .connection
            .send_handshake_message::<handshake_message::KKTRequest>(request.into(), kem)
            .await?;
        Ok(())
    }

    /// Attempt to receive a KKT response to the previously sent request
    async fn receive_kkt_response(&mut self) -> Result<KKTResponse, LpError> {
        let packet_len = KKTResponse::size(self.inner_state.local_peer.ciphersuite.kem());

        let resp = self
            .inner_state
            .connection
            .receive_handshake_message::<handshake_message::KKTResponse>(packet_len)
            .await?;

        Ok(resp.into())
    }

    pub async fn complete_handshake(self) -> Result<LpSession, LpError>
    where
        S: LpHandshakeChannel + Unpin,
    {
        let mut rng = rand09::rngs::StdRng::from_os_rng();
        self.complete_handshake_with_rng(&mut rng).await
    }

    pub async fn complete_handshake_with_rng<R>(mut self, rng: &mut R) -> Result<LpSession, LpError>
    where
        S: LpHandshakeChannel + Unpin,
        R: rand09::CryptoRng,
    {
        let ciphersuite = self.inner_state.local_peer.ciphersuite();
        let kem = ciphersuite.kem();

        // 1. retrieve the expected kem key hash. if we don't know it,
        let dir_hash = self
            .initiator_data
            .remote_peer
            .expected_kem_key_hash(ciphersuite)?;

        // 2. prepare and send KKT request
        let (mut initiator, kkt_request) = KKTInitiator::generate_one_way_request(
            rng,
            ciphersuite,
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
        let conn = self.inner_state.connection;

        // note: the clone is cheap due to internal Arcs
        let encapsulation_key = response.encapsulation_key.clone();

        // build the PSQ initiator
        let initiator_ciphersuite = build_psq_ciphersuite(
            &self.inner_state.local_peer,
            &self.initiator_data.remote_peer,
            &response.encapsulation_key,
        )?;

        let mut psq_initiator = build_psq_principal(rng, protocol, initiator_ciphersuite)?;

        // PSQ msg 1 send
        let mut buf = vec![0u8; psq_msg1_size(kem)];
        // annoyingly `RegistrationInitiator` has to write into unresizable `&mut [u8]`...
        let n = psq_initiator.write_message(&[], &mut buf)?;
        debug!("sending PSQ handshake msg");
        if n != buf.len() {
            return Err(LpError::Internal(
                "unexpected changes in PSQ msg1 size".to_string(),
            ));
        }
        let msg = PSQMsg1::new(buf);
        conn.send_handshake_message(msg, kem).await?;

        // 5. receive and process PSQ response
        let psq_msg: PSQMsg2 = conn.receive_handshake_message(PSQ_MSG2_SIZE).await?;
        debug!("received PSQ handshake msg");
        psq_initiator.read_message(&psq_msg, &mut [])?;

        if !psq_initiator.is_handshake_finished() {
            return Err(LpError::kkt_psq_handshake(
                "handshake not finished after receiving psq response",
            ));
        }

        let binding = PersistentSessionBinding {
            initiator_authenticator: Authenticator::Dh(self.inner_state.local_peer.x25519().pk),
            responder_ecdh_pk: self.initiator_data.remote_peer.x25519_public,
            responder_pq_pk: Some(encapsulation_key),
        };

        let psq_session = psq_initiator.into_session()?;
        LpSession::new(psq_session, binding, protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{decrypt_data, encrypt_data};
    use crate::peer::mock_peers;
    use crate::psq::{PSQ_MSG2_SIZE, psq_msg1_size, responder};
    use nym_kkt::context::KKTMode;
    use nym_kkt::responder::KKTResponder;
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, IntoEnumIterator, KEM, SignatureScheme};
    use nym_test_utils::helpers::{DeterministicRng09Send, u64_seeded_rng_09};
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, Timeboxed};

    #[tokio::test]
    async fn initiator_test_plain() -> anyhow::Result<()> {
        for kem in KEM::iter() {
            let conn_init = MockIOStream::default();
            let conn_resp = conn_init.try_get_remote_handle();

            // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
            // so they'd get 'static lifetime
            let conn_init = conn_init.leak();
            let conn_resp = conn_resp.leak();

            let (mut init, mut resp) = mock_peers();
            let init_remote = init.as_remote();
            let resp_remote = resp.as_remote();

            let ciphersuite = Ciphersuite::default().with_kem(kem);
            init.ciphersuite = ciphersuite;
            resp.ciphersuite = ciphersuite;
            let initiator_data = InitiatorData::new(1, resp_remote);

            let handshake_init =
                PSQHandshakeState::new(conn_init, init).as_initiator(initiator_data);

            let mut init_rng = DeterministicRng09Send::new(u64_seeded_rng_09(1));

            let init_fut = tokio::spawn(async move {
                handshake_init
                    .complete_handshake_with_rng(&mut init_rng)
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
            let raw_kkt_req: handshake_message::KKTRequest = conn_resp
                .receive_handshake_message(KKTRequest::size(KKTMode::OneWay, kem))
                .timeboxed()
                .await??;
            let req = raw_kkt_req.into();

            // 2. process
            let processed_req = kkt_responder.process_request(req)?;
            conn_resp
                .send_handshake_message(processed_req.response.into(), kem)
                .timeboxed()
                .await??;

            // 3. read PSQ req
            let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem)?;
            let mut responder =
                responder::build_psq_principal(rand09::rng(), 1, responder_ciphersuite)?;
            let response_len = psq_msg1_size(kem);

            let msg: PSQMsg1 = conn_resp
                .receive_handshake_message(response_len)
                .timeboxed()
                .await??;
            responder.read_message(&msg, &mut []).unwrap();

            // Get the authenticator out here, so we can deserialize the session later.
            let Some(initiator_authenticator) = responder.initiator_authenticator() else {
                panic!("No initiator authenticator found")
            };

            // 4 send PSQ response
            let mut buf = vec![0u8; PSQ_MSG2_SIZE];
            let n = responder.write_message(&[], &mut buf).unwrap();
            assert_eq!(n, buf.len());
            let msg = PSQMsg2::new(buf);
            conn_resp
                .send_handshake_message(msg, kem)
                .timeboxed()
                .await??;

            assert!(responder.is_handshake_finished());

            let mut session_init = init_fut.await???;

            let mut r_transport = responder.into_session().unwrap();

            // test serialization, deserialization
            let mut channel_i = session_init.active_transport();
            let mut channel_r = r_transport.transport_channel().unwrap();

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
}
