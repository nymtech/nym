// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::peer::LpLocalPeer;
use crate::peer_config::{LP_PEER_CONFIG_SIZE, LpPeerConfig};
use crate::psq::handshake_message::{FRAME_LENGTH_PREFIX_SIZE, PSQMsg1, PSQMsg2};
use crate::psq::helpers::kem_to_ciphersuite;
use crate::psq::{
    AAD_RESPONDER_V1, AAD_RESPONDER_V2, PSQ_MSG2_SIZE, PSQHandshakeState, ResponderData,
    SESSION_CONTEXT_V1, SESSION_CONTEXT_V2, handshake_message, psq_msg1_size,
};
use crate::session::PersistentSessionBinding;
use crate::transport::LpTransportError;
use crate::transport::traits::{HandshakeMessage, LpHandshakeChannel};
use crate::{LpError, LpTransportSession};
use getrandom::SysRng;
use libcrux_psq::handshake::Responder;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, PrincipalBuilder, ResponderCiphersuite,
};
use libcrux_psq::{Channel, IntoSession};
use nym_kkt::context::KKTMode;
use nym_kkt::message::{KKTRequest, KKTResponse, ProcessedKKTRequest};
use nym_kkt::responder::KKTResponder;
use nym_kkt_ciphersuite::KEM;
use nym_lp_data::packet::version;
use rand::SeedableRng;
use tracing::debug;

/// Length of the encrypted frame plus payload that follows the cleartext prefix of an opener.
const fn frame_length(mode: KKTMode, kem: KEM) -> usize {
    KKTRequest::size_excluding_payload(mode, kem) + LP_PEER_CONFIG_SIZE - KKTRequest::PLAINTEXT_SIZE
}

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
    R: rand::CryptoRng,
{
    let (ctx, aad) = match version {
        1 => (SESSION_CONTEXT_V1, AAD_RESPONDER_V1),
        2 => (SESSION_CONTEXT_V2, AAD_RESPONDER_V2),
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
    /// Reads the cleartext prefix, then whatever the version it declares says follows.
    async fn receive_kkt_request(&mut self) -> Result<KKTRequest, LpError> {
        let kem = self.inner_state.local_peer.ciphersuite.kem();
        let accepted_frame_lengths = self.accepted_frame_lengths();
        let connection = &mut *self.inner_state.connection;

        let plaintext = connection.read_n_bytes(KKTRequest::PLAINTEXT_SIZE).await?;
        let protocol_version = KKTRequest::peek_outer_protocol_version(
            &plaintext,
            &self.inner_state.local_peer.x25519().pk,
            &self.responder_data.supported_outer_protocol_versions,
        )?;

        let remainder = if protocol_version < version::V2 {
            // a version 1 opener is always one-way, so its size follows from our KEM alone
            // TODO: an initiator sending a KEM key of a different type than our ciphersuite
            // still cannot be received at this version
            frame_length(KKTMode::OneWay, kem)
        } else {
            let prefix: [u8; FRAME_LENGTH_PREFIX_SIZE] = connection
                .read_n_bytes(FRAME_LENGTH_PREFIX_SIZE)
                .await?
                .try_into()
                .map_err(|_| LpError::Internal("short read of the frame length".to_string()))?;
            let size = u32::from_le_bytes(prefix) as usize;
            if !accepted_frame_lengths.contains(&size) {
                return Err(LpTransportError::MalformedPacket(format!(
                    "no opener this responder could complete has a {size}-byte frame"
                ))
                .into());
            }
            size
        };

        let frame = connection.read_n_bytes(remainder).await?;
        Ok(KKTRequest::from_parts(&plaintext, frame)?)
    }

    /// The frame lengths a version 2 opener may declare: exactly those this responder could complete.
    fn accepted_frame_lengths(&self) -> Vec<usize> {
        // a one-way frame carries no KEM key, so its length is the same for every KEM; a mutual
        // one carries the initiator's, and can only be verified for a KEM we hold a digest for
        let one_way = frame_length(
            KKTMode::OneWay,
            self.inner_state.local_peer.ciphersuite.kem(),
        );
        let mutual = self
            .responder_data
            .initiator_kem_hashes
            .keys()
            .map(|kem| frame_length(KKTMode::Mutual, *kem));
        std::iter::once(one_way).chain(mutual).collect()
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
            &self.responder_data.initiator_kem_hashes,
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

    pub async fn complete_handshake(self) -> Result<LpTransportSession, LpError>
    where
        S: LpHandshakeChannel + Unpin,
    {
        let mut rng = rand::rngs::StdRng::try_from_rng(&mut SysRng)?;

        self.complete_handshake_with_rng(&mut rng).await
    }

    pub async fn complete_handshake_with_rng<R>(
        mut self,
        rng: &mut R,
    ) -> Result<LpTransportSession, LpError>
    where
        S: LpHandshakeChannel + Unpin,
        R: rand::CryptoRng,
    {
        // 1. receive and process KKTRequest
        let kkt_request = self.receive_kkt_request().await?;
        debug!("received KKT request");

        let processed_req = self.process_kkt_request(kkt_request)?;
        let kem = processed_req.requested_kem;
        let init_kem = processed_req.remote_encapsulation_key;

        let lp_peer_config = LpPeerConfig::deserialize(&processed_req.request_payload)?;

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

        let receiver_index =
            lp_peer_config.derive_receiver_index(&initiator_authenticator, &kem_key)?;

        let binding = PersistentSessionBinding {
            initiator_authenticator,
            responder_ecdh_pk: self.inner_state.local_peer.x25519().pk,
            responder_pq_pk: Some(kem_key),
            initiator_pq_pk: init_kem,
        };

        let psq_session = psq_responder.into_session()?;
        LpTransportSession::new(
            psq_session,
            binding,
            receiver_index,
            processed_req.outer_protocol_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::mock_peers;
    use crate::peer_config::LpPeerConfig;
    use crate::psq::initiator;
    use crate::test_helpers::assert_session_matches_transport;
    use nym_kkt::initiator::KKTInitiator;
    use nym_kkt_ciphersuite::{Ciphersuite, IntoEnumIterator};
    use nym_lp_data::packet::version;
    use nym_test_utils::helpers::{DeterministicRngSend, deterministic_rng, u64_seeded_rng};
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

            let mut resp_rng = DeterministicRngSend::new(u64_seeded_rng(2));
            let resp_fut = tokio::spawn(async move {
                handshake_resp
                    .complete_handshake_with_rng(&mut resp_rng)
                    .timeboxed()
                    .await
            });

            // initiator:

            let mut rng = deterministic_rng();
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
                .send_handshake_message(handshake_message::KKTRequest::new(request, 1), kem)
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
                initiator::build_psq_principal(rand::rng(), 1, initiator_ciphersuite)?;

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

            // both sides must have derived the same transport keys
            let mut channel_i = i_transport.transport_channel().unwrap();
            assert_session_matches_transport(&mut session_resp, &mut channel_i);
        }

        Ok(())
    }

    #[tokio::test]
    async fn responder_test_plain_mutual() -> anyhow::Result<()> {
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
            let init_remote = init.as_remote();

            let ciphersuite = Ciphersuite::default().with_kem(kem);
            init.ciphersuite = ciphersuite;
            resp.ciphersuite = ciphersuite;

            let responder_data = ResponderData::default()
                .with_initiator_kem_hashes(init_remote.expected_kem_key_digests);
            let handshake_resp =
                PSQHandshakeState::new(conn_resp, resp).as_responder(responder_data);

            let mut resp_rng = DeterministicRngSend::new(u64_seeded_rng(2));
            let resp_fut = tokio::spawn(async move {
                handshake_resp
                    .complete_handshake_with_rng(&mut resp_rng)
                    .timeboxed()
                    .await
            });

            // initiator:

            let mut rng = deterministic_rng();
            let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite)?;

            let lp_peer_config = LpPeerConfig::new_client_to_entry(&mut rng, false);

            // Mutual - MlKem
            // a version 1 opener is always read as one-way, so a mutual one is version 2 at least
            let (mut initiator, request) = KKTInitiator::generate_mutual_request(
                &mut rng,
                init.ciphersuite,
                init.encoded_kem_key(kem).unwrap(),
                &resp_remote.x25519_public,
                &dir_hash,
                version::V2,
                Some(Vec::from(lp_peer_config.serialize())),
            )?;

            // 1. send kkt request
            conn_init
                .send_handshake_message(
                    handshake_message::KKTRequest::new(request, version::V2),
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
                initiator::build_psq_principal(rand::rng(), version::V2, initiator_ciphersuite)?;

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

            // both sides must have derived the same transport keys
            let mut channel_i = i_transport.transport_channel().unwrap();
            assert_session_matches_transport(&mut session_resp, &mut channel_i);
        }

        Ok(())
    }

    /// Drives a full handshake with the responder advertising `supported` while the initiator
    /// proposes `proposed`, and returns whatever the responder ended up with.
    ///
    /// The initiator side is best-effort on purpose: if the responder refuses the proposed
    /// version it bails before replying, so the receives simply time out and we surface the
    /// responder's error instead.
    async fn responder_handshake_with_versions(
        supported: Vec<u8>,
        proposed: u8,
    ) -> anyhow::Result<Result<LpTransportSession, LpError>> {
        // version negotiation happens in KKT, ahead of anything KEM-specific, so one KEM is enough
        let kem = KEM::MlKem768;

        let conn_init = MockIOStream::default();
        let conn_resp = conn_init.try_get_remote_handle();
        let conn_init = conn_init.leak();
        let conn_resp = conn_resp.leak();

        let (mut init, mut resp) = mock_peers();
        let resp_remote = resp.as_remote();

        let ciphersuite = Ciphersuite::default().with_kem(kem);
        init.ciphersuite = ciphersuite;
        resp.ciphersuite = ciphersuite;

        let responder_data = ResponderData {
            supported_outer_protocol_versions: supported,
            ..Default::default()
        };
        let handshake_resp = PSQHandshakeState::new(conn_resp, resp).as_responder(responder_data);

        let mut resp_rng = DeterministicRngSend::new(u64_seeded_rng(2));
        let resp_fut = tokio::spawn(async move {
            handshake_resp
                .complete_handshake_with_rng(&mut resp_rng)
                .timeboxed()
                .await
        });

        let mut rng = deterministic_rng();
        let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite)?;
        let lp_peer_config = LpPeerConfig::new_client_to_entry(&mut rng, false);

        let (mut initiator, request) = KKTInitiator::generate_one_way_request(
            &mut rng,
            init.ciphersuite,
            &resp_remote.x25519_public,
            &dir_hash,
            proposed,
            Some(Vec::from(lp_peer_config.serialize())),
        )?;

        conn_init
            .send_handshake_message(handshake_message::KKTRequest::new(request, proposed), kem)
            .timeboxed()
            .await??;

        let response_len = KKTResponse::size_excluding_payload(kem);
        let received: Result<Result<handshake_message::KKTResponse, _>, _> = conn_init
            .receive_handshake_message(response_len)
            .timeboxed()
            .await;
        let Ok(Ok(kkt_response)) = received else {
            // nothing came back, so the responder has already given up on us
            return Ok(resp_fut.await??);
        };

        let response = initiator.process_response(kkt_response.into(), 0)?;
        let initiator_ciphersuite =
            initiator::build_psq_ciphersuite(&init, &resp_remote, &response.encapsulation_key)?;
        let mut initiator =
            initiator::build_psq_principal(rand::rng(), proposed, initiator_ciphersuite)?;

        let mut buf = vec![0u8; psq_msg1_size(kem)];
        let n = initiator.write_message(&[], &mut buf).unwrap();
        assert_eq!(n, buf.len());
        conn_init
            .send_handshake_message(PSQMsg1::new(buf), kem)
            .timeboxed()
            .await??;

        let msg: PSQMsg2 = conn_init
            .receive_handshake_message(PSQ_MSG2_SIZE)
            .timeboxed()
            .await??;
        initiator.read_message(&msg, &mut []).unwrap();

        Ok(resp_fut.await??)
    }

    #[tokio::test]
    async fn responder_accepts_an_older_version_it_still_supports() -> anyhow::Result<()> {
        // the shape of a node that has moved on to a newer version but still speaks the old one
        // to the clients already in the field
        let session =
            responder_handshake_with_versions(vec![version::V1, version::V1 + 1], version::V1)
                .await?
                .expect("handshake should succeed for a supported version");

        // and the session carries what was negotiated, not what this build calls current
        assert_eq!(version::V1, session.negotiated_version());

        Ok(())
    }

    #[tokio::test]
    async fn responder_refuses_a_version_it_does_not_support() -> anyhow::Result<()> {
        // identical to the test above but for the one version number, so the contrast is what
        // makes this meaningful: the supported set is genuinely enforced
        let result = responder_handshake_with_versions(vec![version::V1], version::V1 + 1).await?;

        // the version rides as a masked byte the responder brute-forces against its supported
        // set, so an unsupported version is deliberately indistinguishable from a corrupt one -
        // there is no version-specific error to assert on here
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn a_v2_initiator_negotiates_v2() -> anyhow::Result<()> {
        let session =
            responder_handshake_with_versions(vec![version::V1, version::V2], version::V2)
                .await??;

        assert_eq!(version::V2, session.negotiated_version());

        Ok(())
    }

    /// The responder's verdict on a genuine V2 one-way prefix followed by `declared_length` and nothing else.
    async fn responder_outcome_for_declared_length(
        declared_length: u32,
    ) -> anyhow::Result<Result<LpTransportSession, LpError>> {
        let kem = KEM::MlKem768;
        let mut conn_init = MockIOStream::default();
        let mut conn_resp = conn_init.try_get_remote_handle();

        let (mut init, mut resp) = mock_peers();
        let resp_remote = resp.as_remote();
        let ciphersuite = Ciphersuite::default().with_kem(kem);
        init.ciphersuite = ciphersuite;
        resp.ciphersuite = ciphersuite;

        let mut rng = deterministic_rng();
        let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite)?;
        let lp_peer_config = LpPeerConfig::new_client_to_entry(&mut rng, false);
        let (_, request) = KKTInitiator::generate_one_way_request(
            &mut rng,
            init.ciphersuite,
            &resp_remote.x25519_public,
            &dir_hash,
            version::V2,
            Some(Vec::from(lp_peer_config.serialize())),
        )?;

        let mut opener = request.into_bytes();
        opener.truncate(KKTRequest::PLAINTEXT_SIZE);
        opener.extend_from_slice(&declared_length.to_le_bytes());
        conn_init.write_all_and_flush(&opener).timeboxed().await??;

        Ok(
            LpTransportSession::psq_handshake_responder(&mut conn_resp, resp, None)
                .complete_handshake()
                .timeboxed()
                .await?,
        )
    }

    /// The prefix is read before anything else, so a length nothing could satisfy never reaches an allocation.
    #[tokio::test]
    async fn an_oversized_length_prefix_is_rejected_before_any_read() -> anyhow::Result<()> {
        let result = responder_outcome_for_declared_length(u32::MAX).await?;

        assert!(matches!(
            result,
            Err(LpError::TransportFailure(
                LpTransportError::MalformedPacket(_)
            ))
        ));

        Ok(())
    }

    /// A frame shorter than any opener's is rejected before it is read.
    #[tokio::test]
    async fn an_undersized_length_prefix_is_rejected_before_any_read() -> anyhow::Result<()> {
        let result = responder_outcome_for_declared_length(0).await?;

        assert!(matches!(
            result,
            Err(LpError::TransportFailure(
                LpTransportError::MalformedPacket(_)
            ))
        ));

        Ok(())
    }

    /// Without digests only a one-way opener can succeed, so a mutual-sized frame is not even read.
    #[tokio::test]
    async fn a_mutual_sized_frame_is_rejected_before_any_read_without_digests() -> anyhow::Result<()>
    {
        let mutual_frame = frame_length(KKTMode::Mutual, KEM::MlKem768);
        let result = responder_outcome_for_declared_length(mutual_frame as u32).await?;

        assert!(matches!(
            result,
            Err(LpError::TransportFailure(
                LpTransportError::MalformedPacket(_)
            ))
        ));

        Ok(())
    }

    /// What an unfixed istanbul dialer sends: read as one-way, it fails at decryption, not by hanging.
    #[tokio::test]
    async fn a_v1_mutual_opener_is_read_as_one_way_and_fails() -> anyhow::Result<()> {
        let kem = KEM::MlKem768;
        let mut conn_init = MockIOStream::default();
        let mut conn_resp = conn_init.try_get_remote_handle();

        let (mut init, mut resp) = mock_peers();
        let resp_remote = resp.as_remote();
        let init_remote = init.as_remote();
        let ciphersuite = Ciphersuite::default().with_kem(kem);
        init.ciphersuite = ciphersuite;
        resp.ciphersuite = ciphersuite;

        let mut rng = deterministic_rng();
        let dir_hash = resp_remote.expected_kem_key_hash(init.ciphersuite)?;
        let lp_peer_config = LpPeerConfig::new_client_to_entry(&mut rng, false);
        let (_, request) = KKTInitiator::generate_mutual_request(
            &mut rng,
            init.ciphersuite,
            init.encoded_kem_key(kem).unwrap(),
            &resp_remote.x25519_public,
            &dir_hash,
            version::V1,
            Some(Vec::from(lp_peer_config.serialize())),
        )?;
        conn_init
            .send_handshake_message(
                handshake_message::KKTRequest::new(request, version::V1),
                kem,
            )
            .timeboxed()
            .await??;

        let result = LpTransportSession::psq_handshake_responder(
            &mut conn_resp,
            resp,
            Some(init_remote.expected_kem_key_digests),
        )
        .complete_handshake()
        .timeboxed()
        .await?;

        assert!(matches!(result, Err(LpError::KKTFailure { .. })));

        Ok(())
    }
}
