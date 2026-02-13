// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTRequestData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::psq_initiator_create_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp, kem_to_ciphersuite};
use crate::psq::{
    AAD_INITIATOR_INNER_V1, AAD_INITIATOR_OUTER_V1, IntermediateHandshakeFailure,
    PSQHandshakeState, SESSION_CONTEXT_V1,
};
use crate::session::PqSharedSecret;
use crate::{ClientHelloData, LpError, LpMessage, LpSession};
use libcrux_psq::handshake::RegistrationInitiator;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, InitiatorCiphersuite, PrincipalBuilder,
};
use libcrux_psq::handshake::ciphersuites::CiphersuiteName;
use nym_kkt::context::KKTContext;
use nym_kkt::keys::EncapsulationKey;
use nym_kkt_ciphersuite::KEM;
use nym_lp_transport::traits::LpTransport;
use rand09::rng;
use tracing::debug;

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

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    /// Generate and send client hello to the responder
    pub(crate) async fn send_client_hello(&mut self) -> Result<ClientHelloData, LpError> {
        let protocol = self.protocol_version()?;

        // 1. Generate and send ClientHelloData with fresh salt and both public keys
        let timestamp = current_timestamp()?;

        let client_hello_data = self.local_peer.build_client_hello_data(timestamp);
        self.connection
            .send_packet(client_hello_data.into_lp_packet(protocol), None)
            .await?;
        Ok(client_hello_data)
    }

    /// Attempt to receive an ack to sent client hello. returns a boolean indicating
    /// whether the request has been successful or whether there has been a collision in receiver
    /// index requiring a retry
    pub(crate) async fn receive_client_hello_ack(&mut self) -> Result<bool, LpError> {
        match self.receive_non_error(None).await?.message {
            LpMessage::Ack => Ok(true),
            LpMessage::Collision => Ok(false),
            other => {
                // TODO: retry on collision
                Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Ack,
                ))
            }
        }
    }

    /// Attempt to send KKT request to begin the handshake
    pub(crate) async fn send_kkt_request(
        &mut self,
        session_id: u32,
        remote_peer: &LpRemotePeer,
        // ) -> Result<(KKTContext, KKTSessionSecret), LpError> {
    ) -> Result<(KKTContext, ()), LpError> {
        let protocol = self.protocol_version()?;
        todo!()

        // let (kkt_context, kkt_frame) = anonymous_initiator_process(&mut rng(), self.ciphersuite)?;
        // let (session_secret, encrypted_frame) =
        //     encrypt_initial_kkt_frame(&mut rng(), &remote_peer.x25519_public, &kkt_frame)?;
        // let lp_message = KKTRequestData::new(encrypted_frame).into();
        // let lp_packet = self.next_packet(session_id, protocol, lp_message);
        // self.connection.send_packet(lp_packet, None).await?;
        // Ok((kkt_context, session_secret))
    }

    /// Attempt to receive a KKT response to the previously sent request and extract (and validate)
    /// the received encapsulation key
    pub(crate) async fn receive_kkt_response(
        &mut self,
        // (kkt_context, session_secret): (KKTContext, KKTSessionSecret),
        (kkt_context, session_secret): (KKTContext, ()),
        remote_peer: &LpRemotePeer,
        // ) -> Result<EncapsulationKey<'static>, LpError> {
    ) -> Result<(), LpError> {
        todo!()
        // let kkt_response = match self.receive_non_error(None).await?.message {
        //     LpMessage::KKTResponse(response) => response,
        //     other => {
        //         return Err(LpError::unexpected_handshake_response(
        //             other.typ(),
        //             MessageType::KKTResponse,
        //         ));
        //     }
        // };
        // debug!("received KKT response");
        // let expected_kem_key_digest = remote_peer.expected_kem_key_hash(self.ciphersuite)?;
        //
        // let (response_frame, remote_context) =
        //     decrypt_kkt_frame(&session_secret, &kkt_response.0, KKT_RESPONSE_AAD)?;
        // let encapsulation_key = initiator_ingest_response(
        //     &kkt_context,
        //     &response_frame,
        //     &remote_context,
        //     &remote_peer.ed25519_public,
        //     &expected_kem_key_digest,
        // )?;
        // Ok(encapsulation_key)
    }

    /// Attempt to prepare and send initial PSQ msg1
    pub(crate) async fn send_psq_initiator_message(
        &mut self,
        remote_peer: &LpRemotePeer,
        // encapsulation_key: &EncapsulationKey<'_>,
        encapsulation_key: &(),
        salt: &[u8; 32],
        session_id_bytes: &[u8; 4],
    ) -> Result<(OuterAeadKey, NoiseProtocol, PqSharedSecret), LpError> {
        todo!()
        // let protocol = self.protocol_version()?;
        // let session_id = u32::from_le_bytes(*session_id_bytes);
        //
        // let psq_initiator = psq_initiator_create_message(
        //     self.local_peer.x25519.private_key(),
        //     &remote_peer.x25519_public,
        //     encapsulation_key,
        //     self.local_peer.ed25519.private_key(),
        //     self.local_peer.ed25519.public_key(),
        //     salt,
        //     session_id_bytes,
        // )?;
        // let psk = psq_initiator.psk;
        // let psq_payload = psq_initiator.payload;
        //
        // // TEMP \/
        // let outer_aead_key = OuterAeadKey::from_psk(&psk);
        // // TEMP /\
        //
        // // prepare noise state and msg1
        // let mut noise_protocol = NoiseProtocol::build_new_initiator(
        //     self.local_peer.x25519().private_key().as_bytes(),
        //     remote_peer.x25519_public.as_bytes(),
        //     &psk,
        // )?;
        //
        // // prepare noise msg1
        // let noise_msg1 = noise_protocol
        //     .get_bytes_to_send()
        //     .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg1"))??;
        // let psq_len = psq_payload.len() as u16;
        // let mut combined = Vec::with_capacity(2 + psq_payload.len() + noise_msg1.len());
        // combined.extend_from_slice(&psq_len.to_le_bytes());
        // combined.extend_from_slice(&psq_payload);
        // combined.extend_from_slice(&noise_msg1);
        //
        // let lp_message = HandshakeData::new(combined).into();
        // let lp_packet = self.next_packet(session_id, protocol, lp_message);
        //
        // self.connection.send_packet(lp_packet, None).await?;
        // Ok((
        //     outer_aead_key,
        //     noise_protocol,
        //     PqSharedSecret::new(psq_initiator.pq_shared_secret),
        // ))
    }

    /// Attempt to receive and validate received PSQ msg2
    pub(crate) async fn receive_psq_responder_message(
        &mut self,
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        todo!()
        // let psq_msg2 = match self
        //     .connection
        //     .receive_packet(Some(outer_aead_key))
        //     .await?
        //     .message
        // {
        //     LpMessage::Handshake(response) => response.0,
        //     other => {
        //         return Err(LpError::unexpected_handshake_response(
        //             other.typ(),
        //             MessageType::Handshake,
        //         ));
        //     }
        // };
        //
        // // Extract PSK handle: [u16 handle_len][handle_bytes][noise_msg]
        // if psq_msg2.len() < 2 {
        //     return Err(LpError::kkt_psq_handshake("too short msg2 received"));
        // }
        // let handle_len = u16::from_le_bytes([psq_msg2[0], psq_msg2[1]]) as usize;
        // if psq_msg2.len() < 2 + handle_len {
        //     return Err(LpError::kkt_psq_handshake("too short msg2 received"));
        // }
        // // Extract and "store" the PSK handle
        // let _psq_handle_bytes = &psq_msg2[2..2 + handle_len];
        // let noise_payload = &psq_msg2[2 + handle_len..];
        //
        // // *sigh* ignore the message
        // let _noise_msg2 = noise_protocol.read_message(noise_payload)?;
        // Ok(())
    }

    /// Attempt to prepare and send final PSQ msg3
    pub(crate) async fn send_final_psq_message(
        &mut self,
        session_id: u32,
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        todo!()
        // let protocol = self.protocol_version()?;
        //
        // let noise_msg3 = noise_protocol
        //     .get_bytes_to_send()
        //     .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg3"))??;
        //
        // let lp_message = HandshakeData::new(noise_msg3).into();
        // let lp_packet = self.next_packet(session_id, protocol, lp_message);
        // self.connection
        //     .send_packet(lp_packet, Some(outer_aead_key))
        //     .await?;
        //
        // if !noise_protocol.is_handshake_finished() {
        //     return Err(LpError::kkt_psq_handshake(
        //         "noise handshake not finished after msg3",
        //     ));
        // }
        //
        // Ok(())
    }

    /// Receive final ACK that indicates finalisation of the handshake
    pub(crate) async fn receive_final_ack(
        &mut self,
        outer_aead_key: &OuterAeadKey,
    ) -> Result<(), LpError> {
        match self
            .connection
            .receive_packet(Some(outer_aead_key))
            .await?
            .message
        {
            LpMessage::Ack => Ok(()),
            other => Err(LpError::unexpected_handshake_response(
                other.typ(),
                MessageType::Ack,
            )),
        }
    }

    async fn complete_as_initiator_inner(
        &mut self,
    ) -> Result<LpSession, IntermediateHandshakeFailure>
    where
        S: LpTransport + Unpin,
    {
        // 0. retrieve the expected kem key hash. if we don't know it,
        // there's no point in even trying to start the handshake
        let Some(remote_peer) = self.remote_peer.take() else {
            return Err(IntermediateHandshakeFailure::plain(
                LpError::kkt_psq_handshake("initiator can't proceed without remote information"),
            ));
        };

        // 1. Generate and send ClientHelloData with fresh salt and both public keys
        // and keep retrying until we manage to establish a receiver index without collisions
        let mut attempt = 0;
        let client_hello_data = loop {
            attempt += 1;

            debug!("sending client hello");
            let client_hello = self
                .send_client_hello()
                .await
                .map_err(IntermediateHandshakeFailure::plain)?;
            if self
                .receive_client_hello_ack()
                .await
                .map_err(IntermediateHandshakeFailure::plain)?
            {
                debug!("received client hello ACK");
                break client_hello;
            }
            debug!("received client hello collision");

            // TODO: make it configurable
            if attempt > 3 {
                return Err(IntermediateHandshakeFailure::plain(
                    LpError::kkt_psq_handshake(
                        "failed to establish receiver index without collision",
                    ),
                ));
            }
        };
        let session_id = client_hello_data.receiver_index;
        let session_id_bytes = session_id.to_le_bytes();
        let salt = client_hello_data.salt;

        // 3. prepare and send KKT request
        debug!("sending KKT request");
        let kkt_data = self
            .send_kkt_request(session_id, &remote_peer)
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;

        // 4. receive and process KKT response
        let encapsulation_key = self
            .receive_kkt_response(kkt_data, &remote_peer)
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;
        debug!("received KKT response");

        // 5. prepare and send PSQ msg1
        debug!("sending PSQ msg1");
        let (outer_aead_key, mut noise_protocol, pq_shared_secret) = self
            .send_psq_initiator_message(&remote_peer, &encapsulation_key, &salt, &session_id_bytes)
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;

        // 6. receive and process PSQ msg2
        debug!("received PSQ msg2");
        if let Err(source) = self
            .receive_psq_responder_message(&outer_aead_key, &mut noise_protocol)
            .await
        {
            return Err(IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: Some(outer_aead_key),
                source,
            });
        }

        // 7. prepare and send PSQ msg3
        debug!("sending PSQ msg3");
        if let Err(source) = self
            .send_final_psq_message(session_id, &outer_aead_key, &mut noise_protocol)
            .await
        {
            return Err(IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: Some(outer_aead_key),
                source,
            });
        }

        // 8. receive final ACK and finalise
        debug!("received final ACK");
        if let Err(source) = self.receive_final_ack(&outer_aead_key).await {
            return Err(IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: Some(outer_aead_key),
                source,
            });
        }

        #[allow(clippy::expect_used)]
        Ok(LpSession::new(
            session_id,
            self.protocol_version()
                .expect("protocol version is known at this point"),
            outer_aead_key,
            self.local_peer.clone(),
            remote_peer,
            pq_shared_secret,
            noise_protocol,
        ))
    }

    // TODO: missing: receive counter check
    pub async fn complete_as_initiator(mut self) -> Result<LpSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        match self.complete_as_initiator_inner().await {
            Ok(res) => Ok(res),
            Err(err) => Err(self.try_send_error_packet(err).await),
        }
    }
}
