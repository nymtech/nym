// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::peer::LpLocalPeer;
use crate::peer_config::{LP_PEER_CONFIG_SIZE, LpPeerConfig};
use crate::psq::handshake_message::{PSQMsg1, PSQMsg2};
use crate::psq::helpers::kem_to_ciphersuite;
use crate::psq::{
    AAD_RESPONDER_V1, PSQ_MSG2_SIZE, PSQHandshakeState, ResponderData, SESSION_CONTEXT_V1,
    handshake_message, psq_msg1_size,
};
use crate::session::PersistentSessionBinding;
use crate::{LpError, LpSession};
use libcrux_psq::handshake::Responder;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, PrincipalBuilder, ResponderCiphersuite,
};
use libcrux_psq::{Channel, IntoSession};
use nym_kkt::context::KKTMode;
use nym_kkt::message::{KKTRequest, KKTResponse, ProcessedKKTRequest};
use nym_kkt::responder::KKTResponder;
use nym_kkt_ciphersuite::KEM;
use nym_lp_transport::traits::{HandshakeMessage, LpHandshakeChannel};
use rand09::SeedableRng;
use tracing::debug;

pub struct PSQHandshakeStateResponder<'a, S> {
    pub(super) inner_state: PSQHandshakeState<'a, S>,
    pub(super) responder_data: ResponderData,
}

pub(crate) fn build_psq_principal<R>(
    rng: R,
    version: u8,
    ciphersuite: ResponderCiphersuite,
) -> Result<Responder<R>, LpError>
where
    R: rand09::CryptoRng,
{
    let (ctx, aad) = match version {
        1 => (SESSION_CONTEXT_V1, AAD_RESPONDER_V1),
        other => return Err(LpError::UnsupportedVersion { version: other }),
    };

    PrincipalBuilder::new(rng)
        .context(ctx)
        .outer_aad(aad)
        .recent_keys_upper_bound(30)
        .build_responder(ciphersuite)
        .map_err(|inner| LpError::PSQResponderBuilderFailure { inner })
}

pub(crate) fn build_psq_ciphersuite(
    peer: &LpLocalPeer,
    kem: KEM,
) -> Result<ResponderCiphersuite<'_>, LpError> {
    let Some(kem_keys) = peer.kem_keypairs.as_ref() else {
        return Err(LpError::ResponderWithMissingKEMKeys);
    };

    let psq_ciphersuite = kem_to_ciphersuite(kem);
    let builder = CiphersuiteBuilder::new(psq_ciphersuite).longterm_x25519_keys(peer.x25519());

    match kem {
        KEM::MlKem768 => builder
            .longterm_mlkem_encapsulation_key(kem_keys.ml_kem768_encapsulation_key())
            .longterm_mlkem_decapsulation_key(kem_keys.ml_kem768_decapsulation_key()),
        KEM::McEliece => builder
            .longterm_cmc_encapsulation_key(kem_keys.mc_eliece_encapsulation_key())
            .longterm_cmc_decapsulation_key(kem_keys.mc_eliece_decapsulation_key()),
    }
    .build_responder_ciphersuite()
    .map_err(|inner| LpError::PSQResponderBuilderFailure { inner })
}

impl<'a, S> PSQHandshakeStateResponder<'a, S>
where
    S: LpHandshakeChannel + Unpin,
{
    /// Attempt to receive a KKT request from a one-way client
    async fn receive_one_way_kkt_request(&mut self) -> Result<KKTRequest, LpError> {
        let packet_len = KKTRequest::size_excluding_payload(
            KKTMode::OneWay,
            self.inner_state.local_peer.ciphersuite.kem(),
        ) + LP_PEER_CONFIG_SIZE;

        let req = self
            .inner_state
            .connection
            .receive_handshake_message::<handshake_message::KKTRequest>(packet_len)
            .await?;

        Ok(req.into())
    }

    /// Attempt to process the received KKT request
    fn process_kkt_request(&self, kkt_request: KKTRequest) -> Result<ProcessedKKTRequest, LpError> {
        let kem_keys = &self
            .inner_state
            .local_peer
            .kem_keypairs
            .as_ref()
            .ok_or(LpError::ResponderWithMissingKEMKeys)?;

        let processed_req = KKTResponder::new(
            &self.inner_state.local_peer.x25519,
            kem_keys,
            &self.responder_data.supported_hash_functions,
            &self.responder_data.supported_signature_schemes,
            &self.responder_data.supported_outer_protocol_versions,
        )?
        .process_request(kkt_request, LP_PEER_CONFIG_SIZE)?;
        Ok(processed_req)
    }

    /// Attempt to send KKT response to the previously received request
    async fn send_kkt_response(&mut self, response: KKTResponse, kem: KEM) -> Result<(), LpError> {
        self.inner_state
            .connection
            .send_handshake_message::<handshake_message::KKTResponse>(response.into(), kem)
            .await?;
        Ok(())
    }

    /// Attempt to receive and process a PSQ msg1 request
    async fn receive_psq_initiator_message(&mut self, kem: KEM) -> Result<Vec<u8>, LpError> {
        let packet_len = psq_msg1_size(kem);
        let msg: PSQMsg1 = self
            .inner_state
            .connection
            .receive_handshake_message(packet_len)
            .await?;
        Ok(msg.into_bytes())
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
        // 1. receive and process KKTRequest
        let kkt_request = self.receive_one_way_kkt_request().await?;
        debug!("received KKT request");

        let processed_req = self.process_kkt_request(kkt_request)?;
        let kem = processed_req.requested_kem;

        let parsed_payload = LpPeerConfig::deserialize(&processed_req.request_payload)?;

        // 2. send back the KKTResponse
        debug!("sending KKT response");
        self.send_kkt_response(processed_req.response, kem).await?;

        // 3. receive and process PSQ request
        let raw_psq1 = self.receive_psq_initiator_message(kem).await?;
        debug!("received PSQ handshake msg");

        // construct the responder and process the message
        let responder_ciphersuite = build_psq_ciphersuite(&self.inner_state.local_peer, kem)?;
        let version = processed_req.outer_protocol_version;
        let mut psq_responder = build_psq_principal(rng, version, responder_ciphersuite)?;
        psq_responder.read_message(&raw_psq1, &mut [])?;

        let initiator_authenticator = psq_responder
            .initiator_authenticator()
            .ok_or(LpError::MissingInitiatorAuthenticator)?;

        // 4. send PSQ response
        let conn = self.inner_state.connection;

        let mut buf = vec![0u8; PSQ_MSG2_SIZE];
        psq_responder.write_message(&[], &mut buf)?;
        debug!("sending PSQ handshake msg");
        conn.send_handshake_message(PSQMsg2::new(buf), kem).await?;

        if !psq_responder.is_handshake_finished() {
            return Err(LpError::kkt_psq_handshake(
                "handshake not finished after receiving psq response",
            ));
        }

        // SAFETY: we have completed the exchange so this key MUST HAVE been present
        #[allow(clippy::unwrap_used)]
        let kem_key = self
            .inner_state
            .local_peer
            .kem_keypairs
            .as_ref()
            .unwrap()
            .encapsulation_key(kem)
            .unwrap();

        let binding = PersistentSessionBinding {
            initiator_authenticator,
            responder_ecdh_pk: self.inner_state.local_peer.x25519().pk,
            responder_pq_pk: Some(kem_key),
        };

        let psq_session = psq_responder.into_session()?;
        LpSession::new(
            psq_session,
            binding,
            processed_req.receiver_index,
            processed_req.outer_protocol_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{decrypt_data, encrypt_data};
    use crate::peer::mock_peers;
    use crate::peer_config::LpPeerConfig;
    use crate::psq::initiator;
    use nym_kkt::initiator::KKTInitiator;
    use nym_kkt_ciphersuite::{Ciphersuite, IntoEnumIterator};
    use nym_test_utils::helpers::{
        DeterministicRng09Send, deterministic_rng_09, u64_seeded_rng_09,
    };
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::{Leak, Timeboxed};

    #[tokio::test]
    async fn responder_test_plain() -> anyhow::Result<()> {
        for kem in KEM::iter() {
            let conn_init = MockIOStream::default();
            let conn_resp = conn_init.try_get_remote_handle();

            // SETUP START:
            // leak the connections (JUST FOR THE PURPOSE OF THIS TEST!)
            // so they'd get 'static lifetime
            let conn_init = conn_init.leak();
            let conn_resp = conn_resp.leak();

            let (mut init, mut resp) = mock_peers();
            let resp_remote = resp.as_remote();

            let ciphersuite = Ciphersuite::default().with_kem(kem);
            init.ciphersuite = ciphersuite;
            resp.ciphersuite = ciphersuite;

            let responder_data = ResponderData::default();
            let handshake_resp =
                PSQHandshakeState::new(conn_resp, resp).as_responder(responder_data);

            let mut resp_rng = DeterministicRng09Send::new(u64_seeded_rng_09(2));
            let resp_fut = tokio::spawn(async move {
                handshake_resp
                    .complete_handshake_with_rng(&mut resp_rng)
                    .timeboxed()
                    .await
            });

            // initiator:

            let mut rng = deterministic_rng_09();
            let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite)?;

            let lp_peer_config = LpPeerConfig::new_client_to_entry(&mut rng, false);

            // OneWay - MlKem
            let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                &mut rng,
                init.ciphersuite,
                &resp_remote.x25519_public,
                &dir_hash,
                1,
                Some(Vec::from(lp_peer_config.serialize())),
            )?;

            // 1. send kkt request
            conn_init
                .send_handshake_message::<handshake_message::KKTRequest>(
                    request.request.into(),
                    kem,
                )
                .timeboxed()
                .await??;

            // 2. receive KKT response
            let response_len = KKTResponse::size_excluding_payload(kem);
            let resp: handshake_message::KKTResponse = conn_init
                .receive_handshake_message(response_len)
                .timeboxed()
                .await??;
            let kkt_response = resp.into();

            let response = initiator.process_response(kkt_response, 0)?;
            let encapsulation_key = response.encapsulation_key;

            let initiator_ciphersuite =
                initiator::build_psq_ciphersuite(&init, &resp_remote, &encapsulation_key)?;
            let mut initiator =
                initiator::build_psq_principal(rand09::rng(), 1, initiator_ciphersuite)?;

            // 3. send PSQ msg1
            // Send first message
            let mut buf = vec![0u8; psq_msg1_size(kem)];
            let n = initiator.write_message(&[], &mut buf).unwrap();
            assert_eq!(n, buf.len());
            let msg = PSQMsg1::new(buf);
            conn_init
                .send_handshake_message(msg, kem)
                .timeboxed()
                .await??;

            // 4. receive PSQ msg2
            let msg: PSQMsg2 = conn_init
                .receive_handshake_message(PSQ_MSG2_SIZE)
                .timeboxed()
                .await??;
            initiator.read_message(&msg, &mut []).unwrap();

            assert!(initiator.is_handshake_finished());

            let mut session_resp = resp_fut.await???;

            let mut i_transport = initiator.into_session().unwrap();

            // test serialization, deserialization
            let mut channel_i = i_transport.transport_channel().unwrap();
            let channel_r = session_resp.active_transport();

            assert_eq!(channel_i.identifier(), channel_r.identifier());

            let app_data_i = b"Derived session hey".as_slice();
            let app_data_r = b"Derived session ho".as_slice();

            let ct_i = encrypt_data(app_data_i, &mut channel_i)?;
            let pt_r = decrypt_data(&ct_i, channel_r)?;

            assert_eq!(app_data_i, pt_r);

            let ct_r = encrypt_data(app_data_r, channel_r)?;
            let pt_i = decrypt_data(&ct_r, &mut channel_i)?;

            assert_eq!(app_data_r, pt_i);
        }

        Ok(())
    }
}
