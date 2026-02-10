// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTResponseData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::peer::LpRemotePeer;
use crate::psk::psq_responder_process_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp};
use crate::psq::{IntermediateHandshakeFailure, PSQHandshakeState};
use crate::session::PqSharedSecret;
use crate::{ClientHelloData, LpError, LpMessage, LpSession};
use nym_kkt::KKT_RESPONSE_AAD;
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
use nym_kkt::context::KKTContext;
use nym_kkt::encryption::{KKTSessionSecret, decrypt_initial_kkt_frame, encrypt_kkt_frame};
use nym_kkt::frame::KKTSessionId;
use nym_kkt::session::{responder_ingest_message, responder_process};
use nym_lp_transport::traits::LpTransport;
use rand09::rng;
use std::time::Duration;
use tracing::debug;

pub const DEFAULT_TIMESTAMP_TOLERANCE: Duration = Duration::from_secs(30);

// this will be removed anyway, so no point in doing anything more than a hardcoded placeholder
fn validate_client_hello_timestamp(
    client_timestamp: u64,
    tolerance: Duration,
) -> Result<(), LpError> {
    let now = current_timestamp()?;

    let age = now.abs_diff(client_timestamp);
    if age > tolerance.as_secs() {
        let direction = if now >= client_timestamp {
            "old"
        } else {
            "future"
        };

        return Err(LpError::kkt_psq_handshake(format!(
            "ClientHello timestamp is too {direction} (age: {age}s, tolerance: {}s)",
            tolerance.as_secs()
        )));
    }

    Ok(())
}

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    pub(crate) fn encapsulated_kem_keys(
        &self,
    ) -> Result<(DecapsulationKey<'static>, EncapsulationKey<'static>), LpError> {
        let kem_keys = self
            .local_peer
            .kem_psq
            .as_ref()
            .ok_or(LpError::ResponderWithMissingKEMKey)?;

        let libcrux_private_key = libcrux_kem::PrivateKey::decode(
            libcrux_kem::Algorithm::X25519,
            kem_keys.private_key().as_bytes(),
        )
        .map_err(|e| {
            LpError::KKTError(format!(
                "Failed to convert X25519 private key to libcrux PrivateKey: {e:?}",
            ))
        })?;
        let dec_key = DecapsulationKey::X25519(libcrux_private_key);

        let libcrux_public_key = libcrux_kem::PublicKey::decode(
            libcrux_kem::Algorithm::X25519,
            kem_keys.public_key().as_bytes(),
        )
        .map_err(|e| {
            LpError::KKTError(format!(
                "Failed to convert X25519 public key to libcrux PublicKey: {e:?}",
            ))
        })?;
        let enc_key = EncapsulationKey::X25519(libcrux_public_key);
        Ok((dec_key, enc_key))
    }

    /// Attempt to receive and validate ClientHello
    pub(crate) async fn receive_client_hello(
        &mut self,
    ) -> Result<(ClientHelloData, LpRemotePeer), LpError> {
        let client_hello_packet = self.receive_non_error(None).await?;
        let client_hello = match client_hello_packet.message {
            LpMessage::ClientHello(client_hello) => client_hello,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::ClientHello,
                ));
            }
        };

        validate_client_hello_timestamp(
            client_hello.extract_timestamp(),
            DEFAULT_TIMESTAMP_TOLERANCE,
        )?;

        // TODO: somehow check for collision

        // set version and remote peer information
        self.protocol_version = Some(client_hello_packet.header.protocol_version);
        let remote_peer = LpRemotePeer::new(
            client_hello.client_ed25519_public_key,
            client_hello.client_lp_public_key,
        );

        Ok((client_hello, remote_peer))
    }

    /// Send client hello ACK
    pub(crate) async fn send_client_hello_ack(&mut self, session_id: u32) -> Result<(), LpError> {
        let protocol = self.protocol_version()?;

        let ack = self.next_packet(session_id, protocol, LpMessage::Ack);
        self.connection.send_packet(ack, None).await?;
        Ok(())
    }

    /// Attempt to receive and process a KKT request
    pub(crate) async fn receive_kkt_request(
        &mut self,
    ) -> Result<(KKTContext, KKTSessionSecret, KKTSessionId), LpError> {
        let kkt_request = match self.receive_non_error(None).await?.message {
            LpMessage::KKTRequest(request) => request.0,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::KKTRequest,
                ));
            }
        };

        let (session_secret, request_frame, remote_context) =
            decrypt_initial_kkt_frame(self.local_peer.x25519.private_key(), &kkt_request)?;
        let (context, _) = responder_ingest_message(&remote_context, None, None, &request_frame)?;

        Ok((context, session_secret, request_frame.session_id()))
    }

    /// Attempt to send KKT response to the previously received request
    pub(crate) async fn send_kkt_response(
        &mut self,
        session_id: u32,
        (kkt_context, session_secret, kkt_session_id): (KKTContext, KKTSessionSecret, KKTSessionId),
        encapsulation_key: &EncapsulationKey<'_>,
    ) -> Result<(), LpError> {
        let protocol = self.protocol_version()?;

        let response_frame = responder_process(
            &kkt_context,
            kkt_session_id,
            self.local_peer.ed25519().private_key(),
            encapsulation_key,
        )?;
        let encrypted_frame = encrypt_kkt_frame(
            &mut rng(),
            &session_secret,
            &response_frame,
            KKT_RESPONSE_AAD,
        )?;
        let lp_message = KKTResponseData::new(encrypted_frame).into();
        let lp_packet = self.next_packet(session_id, protocol, lp_message);

        self.connection.send_packet(lp_packet, None).await?;
        Ok(())
    }

    /// Attempt to receive and process a PSQ msg1 request
    pub(crate) async fn receive_psq_initiator_message(
        &mut self,
        remote_peer: &LpRemotePeer,
        local_kem_keypair: (&DecapsulationKey<'_>, &EncapsulationKey<'_>),
        salt: &[u8; 32],
        session_id_bytes: &[u8; 4],
    ) -> Result<(OuterAeadKey, NoiseProtocol, PqSharedSecret, Vec<u8>), LpError> {
        let psq_msg1 = match self.receive_non_error(None).await?.message {
            LpMessage::Handshake(response) => response.0,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Handshake,
                ));
            }
        };

        // Extract PSQ payload: [u16 psq_len][psq_payload][noise_msg]
        if psq_msg1.len() < 2 {
            return Err(LpError::kkt_psq_handshake("too short msg1 received"));
        }
        let handle_len = u16::from_le_bytes([psq_msg1[0], psq_msg1[1]]) as usize;
        if psq_msg1.len() < 2 + handle_len {
            return Err(LpError::kkt_psq_handshake("too short msg1 received"));
        }
        let psq_payload = &psq_msg1[2..2 + handle_len];
        let noise_payload = &psq_msg1[2 + handle_len..];

        // Decapsulate PSK from PSQ payload using X25519 as DHKEM
        let psq_responder = psq_responder_process_message(
            self.local_peer.x25519.private_key(),
            &remote_peer.x25519_public,
            local_kem_keypair,
            &remote_peer.ed25519_public,
            psq_payload,
            salt,
            session_id_bytes,
        )?;

        let psk = psq_responder.psk;
        let psk_handle = psq_responder.psk_handle;

        // TEMP \/
        let outer_aead_key = OuterAeadKey::from_psk(&psk);
        // TEMP /\

        let mut noise_protocol = NoiseProtocol::build_new_responder(
            self.local_peer.x25519().private_key().as_bytes(),
            remote_peer.x25519_public.as_bytes(),
            &psk,
        )?;
        noise_protocol.read_message(noise_payload)?;

        Ok((
            outer_aead_key,
            noise_protocol,
            PqSharedSecret::new(psq_responder.pq_shared_secret),
            psk_handle,
        ))
    }

    /// Attempt to prepare and generate a responder PSQ msg2
    pub(crate) async fn send_psq_responder_message(
        &mut self,
        session_id: u32,
        psk_handle: &[u8],
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        let protocol = self.protocol_version()?;

        let msg2 = noise_protocol
            .get_bytes_to_send()
            .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg2"))??;
        // Embed PSK handle in message: [u16 handle_len][handle_bytes][noise_msg]
        let handle_len = psk_handle.len() as u16;
        let mut combined = Vec::with_capacity(2 + psk_handle.len() + msg2.len());
        combined.extend_from_slice(&handle_len.to_le_bytes());
        combined.extend_from_slice(psk_handle);
        combined.extend_from_slice(&msg2);

        let lp_message = HandshakeData::new(combined).into();
        let lp_packet = self.next_packet(session_id, protocol, lp_message);
        self.connection
            .send_packet(lp_packet, Some(outer_aead_key))
            .await?;
        Ok(())
    }

    /// Attempt to receive and process final PSQ msg3
    pub(crate) async fn receive_final_psq_message(
        &mut self,
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        let psq_msg3 = match self
            .connection
            .receive_packet(Some(outer_aead_key))
            .await?
            .message
        {
            LpMessage::Handshake(response) => response.0,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Handshake,
                ));
            }
        };

        noise_protocol.read_message(&psq_msg3)?;
        if !noise_protocol.is_handshake_finished() {
            return Err(LpError::kkt_psq_handshake(
                "noise handshake not finished after msg3",
            ));
        }
        Ok(())
    }

    /// Send final ACK to indicate finalisation of the handshake
    pub(crate) async fn send_final_ack(
        &mut self,
        session_id: u32,
        outer_aead_key: &OuterAeadKey,
    ) -> Result<(), LpError> {
        let protocol = self.protocol_version()?;

        let ack = self.next_packet(session_id, protocol, LpMessage::Ack);
        self.connection
            .send_packet(ack, Some(outer_aead_key))
            .await?;
        Ok(())
    }

    async fn complete_as_responder_inner(
        &mut self,
    ) -> Result<LpSession, IntermediateHandshakeFailure>
    where
        S: LpTransport + Unpin,
    {
        // 1. receive and validate ClientHello
        let (client_hello_data, remote_peer) =
            self.receive_client_hello()
                .await
                .map_err(|source| IntermediateHandshakeFailure {
                    session_id: None,
                    protocol_version: self.protocol_version,
                    outer_aead_key: None,
                    source,
                })?;
        debug!("received client hello");

        let session_id = client_hello_data.receiver_index;
        let session_id_bytes = session_id.to_le_bytes();
        let salt = client_hello_data.salt;

        // 2. send ack
        debug!("sending client hello ACK");
        self.send_client_hello_ack(session_id)
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;

        // 3. receive and process KKT request
        let kkt_data =
            self.receive_kkt_request()
                .await
                .map_err(|source| IntermediateHandshakeFailure {
                    session_id: Some(session_id),
                    protocol_version: self.protocol_version,
                    outer_aead_key: None,
                    source,
                })?;
        debug!("received KKT request");

        // TEMP: 'derive' KEM keys
        let (dec_key, enc_key) =
            self.encapsulated_kem_keys()
                .map_err(|source| IntermediateHandshakeFailure {
                    session_id: Some(session_id),
                    protocol_version: self.protocol_version,
                    outer_aead_key: None,
                    source,
                })?;

        // 4. prepare and send KKT response
        debug!("sending KKT response");
        self.send_kkt_response(session_id, kkt_data, &enc_key)
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;

        // 5. receive and process PSQ msg1
        debug!("received PSQ msg1");
        let (outer_aead_key, mut noise_protocol, pq_shared_secret, psk_handle) = self
            .receive_psq_initiator_message(
                &remote_peer,
                (&dec_key, &enc_key),
                &salt,
                &session_id_bytes,
            )
            .await
            .map_err(|source| IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: None,
                source,
            })?;

        // 6. prepare and send PSQ msg2
        debug!("sending PSQ msg2");
        if let Err(source) = self
            .send_psq_responder_message(
                session_id,
                &psk_handle,
                &outer_aead_key,
                &mut noise_protocol,
            )
            .await
        {
            return Err(IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: Some(outer_aead_key),
                source,
            });
        }

        // 7. receive and process PSQ msg3
        debug!("received PSQ msg3");
        if let Err(source) = self
            .receive_final_psq_message(&outer_aead_key, &mut noise_protocol)
            .await
        {
            return Err(IntermediateHandshakeFailure {
                session_id: Some(session_id),
                protocol_version: self.protocol_version,
                outer_aead_key: Some(outer_aead_key),
                source,
            });
        }

        // 8. [optionally] send ACK to finalise
        debug!("sending final ACK");
        if let Err(source) = self.send_final_ack(session_id, &outer_aead_key).await {
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

    pub async fn complete_as_responder(mut self) -> Result<LpSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        match self.complete_as_responder_inner().await {
            Ok(res) => Ok(res),
            Err(err) => Err(self.try_send_error_packet(err).await),
        }
    }
}
