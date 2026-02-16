// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTRequestData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::psq_initiator_create_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp, kem_to_ciphersuite};
use crate::psq::{
    AAD_INITIATOR_INNER_V1, AAD_INITIATOR_OUTER_V1, IntermediateHandshakeFailure, MinimalSession,
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
    fn build_psq_initiator_principal<'b>(
        &'b self,
        encapsulation_key: &'b EncapsulationKey,
    ) -> Result<RegistrationInitiator<'b, rand09::rngs::ThreadRng>, LpError> {
        let initiator_ciphersuite =
            build_psq_ciphersuite(&self.local_peer, self.remote_peer()?, &encapsulation_key)?;
        let initiator =
            build_psq_principal(rng(), self.protocol_version()?, initiator_ciphersuite)?;
        Ok(initiator)
    }

    /// Attempt to send KKT request to begin the handshake
    pub(crate) async fn send_kkt_request(&mut self, request: KKTRequest) -> Result<(), LpError> {
        // TODO: extra header
        self.connection
            .send_serialised_packet(&request.into_bytes())
            .await?;
        Ok(())
    }

    /// Attempt to receive a KKT response to the previously sent request
    pub(crate) async fn receive_kkt_response(&mut self) -> Result<KKTResponse, LpError> {
        let data = self.connection.receive_raw_packet().await?;
        Ok(KKTResponse::from_bytes(data))
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

    pub async fn complete_as_initiator_inner<R>(
        mut self,
        rng: &mut R,
    ) -> Result<MinimalSession, LpError>
    where
        S: LpTransport + Unpin,
        R: rand09::CryptoRng,
    {
        // 1. retrieve the expected kem key hash. if we don't know it,
        let dir_hash = self
            .remote_peer()?
            .expected_kem_key_hash(self.ciphersuite)?;

        // 2. prepare and send KKT request
        let (mut initiator, kkt_request) = KKTInitiator::generate_one_way_request(
            rng,
            self.ciphersuite,
            self.remote_peer()?.x25519(),
            &dir_hash,
            self.protocol_version()?,
        )?;
        debug!("sending KKT request");
        self.send_kkt_request(kkt_request).await?;

        // 3. receive and process KKT response
        let raw_response = self.receive_kkt_response().await?;
        debug!("received KKT response");
        let response = initiator.process_response(raw_response)?;

        // 4. generate and send PSQ request
        let protocol = self.protocol_version()?;
        let mut conn = self.connection;

        let remote_peer = self
            .remote_peer
            .as_ref()
            .ok_or(LpError::MissingRemotePeerInformation)?;

        // build the PSQ initiator
        let initiator_ciphersuite =
            build_psq_ciphersuite(&self.local_peer, remote_peer, &response.encapsulation_key)?;

        let mut psq_initiator = build_psq_principal(rng, protocol, initiator_ciphersuite)?;

        // PSQ msg 1 send
        let mut buf = [0u8; 2048];
        // annoyingly `RegistrationInitiator` has to write into unresizable `&mut [u8]`...
        let n = psq_initiator.write_message(&[], &mut buf)?;
        debug!("sending PSQ handshake msg");
        conn.send_serialised_packet(&buf[..n]).await?;

        // 5. receive and process PSQ response
        let TODO = "change buf size";
        let mut buf = [0u8; 2048];
        let psq_msg = conn.receive_raw_packet().await?;
        debug!("received PSQ handshake msg");
        psq_initiator.read_message(&psq_msg, &mut buf)?;

        if !psq_initiator.is_handshake_finished() {
            return Err(LpError::kkt_psq_handshake(
                "handshake not finished after receiving psq response",
            ));
        }

        let session = psq_initiator.into_session()?;
        Ok(MinimalSession {
            session,
            encapsulation_key: Some(response.encapsulation_key),
        })
    }

    // TODO: missing: receive counter check
    pub async fn complete_as_initiator(mut self) -> Result<LpSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        todo!()
        // match self.complete_as_initiator_inner().await {
        //     Ok(res) => Ok(res),
        //     Err(err) => Err(self.try_send_error_packet(err).await),
        // }
    }
}
