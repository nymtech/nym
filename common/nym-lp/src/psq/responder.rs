// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTResponseData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::psq_responder_process_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp, kem_to_ciphersuite};
use crate::psq::{
    AAD_RESPONDER_V1, IntermediateHandshakeFailure, PSQHandshakeState, SESSION_CONTEXT_V1,
};
use crate::session::PqSharedSecret;
use crate::{ClientHelloData, LpError, LpMessage, LpSession};
use libcrux_psq::handshake::Responder;
use libcrux_psq::handshake::builders::{
    CiphersuiteBuilder, PrincipalBuilder, ResponderCiphersuite,
};
use libcrux_psq::handshake::ciphersuites::CiphersuiteName;
use nym_kkt::context::KKTContext;
use nym_kkt::keys::KEMKeys;
use nym_kkt_ciphersuite::KEM;
use nym_lp_transport::traits::LpTransport;
use rand09::rng;
use std::time::Duration;
use tracing::debug;

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
) -> Result<ResponderCiphersuite, LpError> {
    let Some(kem_keys) = peer.kem_keypairs.as_ref() else {
        return Err(LpError::ResponderWithMissingKEMKey { kem });
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
    pub(crate) fn encapsulated_kem_keys(&self) -> Result<((), ()), LpError> {
        todo!()
    }
    //
    // pub(crate) fn encapsulated_kem_keys(
    //     &self,
    // ) -> Result<(DecapsulationKey<'static>, EncapsulationKey<'static>), LpError> {
    //     let kem_keys = self
    //         .local_peer
    //         .kem_psq
    //         .as_ref()
    //         .ok_or(LpError::ResponderWithMissingKEMKey)?;
    //
    //     let libcrux_private_key = libcrux_kem::PrivateKey::decode(
    //         libcrux_kem::Algorithm::X25519,
    //         kem_keys.private_key().as_bytes(),
    //     )
    //     .map_err(|e| {
    //         LpError::KKTError(format!(
    //             "Failed to convert X25519 private key to libcrux PrivateKey: {e:?}",
    //         ))
    //     })?;
    //     let dec_key = DecapsulationKey::X25519(libcrux_private_key);
    //
    //     let libcrux_public_key = libcrux_kem::PublicKey::decode(
    //         libcrux_kem::Algorithm::X25519,
    //         kem_keys.public_key().as_bytes(),
    //     )
    //     .map_err(|e| {
    //         LpError::KKTError(format!(
    //             "Failed to convert X25519 public key to libcrux PublicKey: {e:?}",
    //         ))
    //     })?;
    //     let enc_key = EncapsulationKey::X25519(libcrux_public_key);
    //     Ok((dec_key, enc_key))
    // }

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
    pub(crate) async fn receive_kkt_request(&mut self) -> Result<(KKTContext, (), ()), LpError> {
        todo!()
    }
    // pub(crate) async fn receive_kkt_request(
    //     &mut self,
    // ) -> Result<(KKTContext, KKTSessionSecret, KKTSessionId), LpError> {
    //     let kkt_request = match self.receive_non_error(None).await?.message {
    //         LpMessage::KKTRequest(request) => request.0,
    //         other => {
    //             return Err(LpError::unexpected_handshake_response(
    //                 other.typ(),
    //                 MessageType::KKTRequest,
    //             ));
    //         }
    //     };
    //
    //     let (session_secret, request_frame, remote_context) =
    //         decrypt_initial_kkt_frame(self.local_peer.x25519.private_key(), &kkt_request)?;
    //     let (context, _) = responder_ingest_message(&remote_context, None, None, &request_frame)?;
    //
    //     Ok((context, session_secret, request_frame.session_id()))
    // }

    /// Attempt to send KKT response to the previously received request
    pub(crate) async fn send_kkt_response(
        &mut self,
        session_id: u32,
        // (kkt_context, session_secret, kkt_session_id): (KKTContext, KKTSessionSecret, KKTSessionId),
        (kkt_context, session_secret, kkt_session_id): (KKTContext, (), ()),
        // encapsulation_key: &EncapsulationKey<'_>,
        encapsulation_key: &(),
    ) -> Result<(), LpError> {
        todo!()
        // let protocol = self.protocol_version()?;
        //
        // let response_frame = responder_process(
        //     &kkt_context,
        //     kkt_session_id,
        //     self.local_peer.ed25519().private_key(),
        //     encapsulation_key,
        // )?;
        // let encrypted_frame = encrypt_kkt_frame(
        //     &mut rng(),
        //     &session_secret,
        //     &response_frame,
        //     KKT_RESPONSE_AAD,
        // )?;
        // let lp_message = KKTResponseData::new(encrypted_frame).into();
        // let lp_packet = self.next_packet(session_id, protocol, lp_message);
        //
        // self.connection.send_packet(lp_packet, None).await?;
        // Ok(())
    }

    /// Attempt to receive and process a PSQ msg1 request
    pub(crate) async fn receive_psq_initiator_message(
        &mut self,
        remote_peer: &LpRemotePeer,
        // local_kem_keypair: (&DecapsulationKey<'_>, &EncapsulationKey<'_>),
        local_kem_keypair: ((), ()),
        salt: &[u8; 32],
        session_id_bytes: &[u8; 4],
    ) -> Result<(OuterAeadKey, NoiseProtocol, PqSharedSecret, Vec<u8>), LpError> {
        todo!()
        // let psq_msg1 = match self.receive_non_error(None).await?.message {
        //     LpMessage::Handshake(response) => response.0,
        //     other => {
        //         return Err(LpError::unexpected_handshake_response(
        //             other.typ(),
        //             MessageType::Handshake,
        //         ));
        //     }
        // };
        //
        // // Extract PSQ payload: [u16 psq_len][psq_payload][noise_msg]
        // if psq_msg1.len() < 2 {
        //     return Err(LpError::kkt_psq_handshake("too short msg1 received"));
        // }
        // let handle_len = u16::from_le_bytes([psq_msg1[0], psq_msg1[1]]) as usize;
        // if psq_msg1.len() < 2 + handle_len {
        //     return Err(LpError::kkt_psq_handshake("too short msg1 received"));
        // }
        // let psq_payload = &psq_msg1[2..2 + handle_len];
        // let noise_payload = &psq_msg1[2 + handle_len..];
        //
        // // Decapsulate PSK from PSQ payload using X25519 as DHKEM
        // let psq_responder = psq_responder_process_message(
        //     self.local_peer.x25519.private_key(),
        //     &remote_peer.x25519_public,
        //     local_kem_keypair,
        //     &remote_peer.ed25519_public,
        //     psq_payload,
        //     salt,
        //     session_id_bytes,
        // )?;
        //
        // let psk = psq_responder.psk;
        // let psk_handle = psq_responder.psk_handle;
        //
        // // TEMP \/
        // let outer_aead_key = OuterAeadKey::from_psk(&psk);
        // // TEMP /\
        //
        // let mut noise_protocol = NoiseProtocol::build_new_responder(
        //     self.local_peer.x25519().private_key().as_bytes(),
        //     remote_peer.x25519_public.as_bytes(),
        //     &psk,
        // )?;
        // noise_protocol.read_message(noise_payload)?;
        //
        // Ok((
        //     outer_aead_key,
        //     noise_protocol,
        //     PqSharedSecret::new(psq_responder.pq_shared_secret),
        //     psk_handle,
        // ))
    }

    /// Attempt to prepare and generate a responder PSQ msg2
    pub(crate) async fn send_psq_responder_message(
        &mut self,
        session_id: u32,
        psk_handle: &[u8],
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        todo!()
        // let protocol = self.protocol_version()?;
        //
        // let msg2 = noise_protocol
        //     .get_bytes_to_send()
        //     .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg2"))??;
        // // Embed PSK handle in message: [u16 handle_len][handle_bytes][noise_msg]
        // let handle_len = psk_handle.len() as u16;
        // let mut combined = Vec::with_capacity(2 + psk_handle.len() + msg2.len());
        // combined.extend_from_slice(&handle_len.to_le_bytes());
        // combined.extend_from_slice(psk_handle);
        // combined.extend_from_slice(&msg2);
        //
        // let lp_message = HandshakeData::new(combined).into();
        // let lp_packet = self.next_packet(session_id, protocol, lp_message);
        // self.connection
        //     .send_packet(lp_packet, Some(outer_aead_key))
        //     .await?;
        // Ok(())
    }

    /// Attempt to receive and process final PSQ msg3
    pub(crate) async fn receive_final_psq_message(
        &mut self,
        outer_aead_key: &OuterAeadKey,
        noise_protocol: &mut NoiseProtocol,
    ) -> Result<(), LpError> {
        todo!()
        // let psq_msg3 = match self
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
        // noise_protocol.read_message(&psq_msg3)?;
        // if !noise_protocol.is_handshake_finished() {
        //     return Err(LpError::kkt_psq_handshake(
        //         "noise handshake not finished after msg3",
        //     ));
        // }
        // Ok(())
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

        todo!()
        // // TEMP: 'derive' KEM keys
        // let (dec_key, enc_key) =
        //     self.encapsulated_kem_keys()
        //         .map_err(|source| IntermediateHandshakeFailure {
        //             session_id: Some(session_id),
        //             protocol_version: self.protocol_version,
        //             outer_aead_key: None,
        //             source,
        //         })?;
        //
        // // 4. prepare and send KKT response
        // debug!("sending KKT response");
        // self.send_kkt_response(session_id, kkt_data, &enc_key)
        //     .await
        //     .map_err(|source| IntermediateHandshakeFailure {
        //         session_id: Some(session_id),
        //         protocol_version: self.protocol_version,
        //         outer_aead_key: None,
        //         source,
        //     })?;
        //
        // // 5. receive and process PSQ msg1
        // debug!("received PSQ msg1");
        // let (outer_aead_key, mut noise_protocol, pq_shared_secret, psk_handle) = self
        //     .receive_psq_initiator_message(
        //         &remote_peer,
        //         (&dec_key, &enc_key),
        //         &salt,
        //         &session_id_bytes,
        //     )
        //     .await
        //     .map_err(|source| IntermediateHandshakeFailure {
        //         session_id: Some(session_id),
        //         protocol_version: self.protocol_version,
        //         outer_aead_key: None,
        //         source,
        //     })?;
        //
        // // 6. prepare and send PSQ msg2
        // debug!("sending PSQ msg2");
        // if let Err(source) = self
        //     .send_psq_responder_message(
        //         session_id,
        //         &psk_handle,
        //         &outer_aead_key,
        //         &mut noise_protocol,
        //     )
        //     .await
        // {
        //     return Err(IntermediateHandshakeFailure {
        //         session_id: Some(session_id),
        //         protocol_version: self.protocol_version,
        //         outer_aead_key: Some(outer_aead_key),
        //         source,
        //     });
        // }
        //
        // // 7. receive and process PSQ msg3
        // debug!("received PSQ msg3");
        // if let Err(source) = self
        //     .receive_final_psq_message(&outer_aead_key, &mut noise_protocol)
        //     .await
        // {
        //     return Err(IntermediateHandshakeFailure {
        //         session_id: Some(session_id),
        //         protocol_version: self.protocol_version,
        //         outer_aead_key: Some(outer_aead_key),
        //         source,
        //     });
        // }
        //
        // // 8. [optionally] send ACK to finalise
        // debug!("sending final ACK");
        // if let Err(source) = self.send_final_ack(session_id, &outer_aead_key).await {
        //     return Err(IntermediateHandshakeFailure {
        //         session_id: Some(session_id),
        //         protocol_version: self.protocol_version,
        //         outer_aead_key: Some(outer_aead_key),
        //         source,
        //     });
        // }
        //
        // #[allow(clippy::expect_used)]
        // Ok(LpSession::new(
        //     session_id,
        //     self.protocol_version()
        //         .expect("protocol version is known at this point"),
        //     outer_aead_key,
        //     self.local_peer.clone(),
        //     remote_peer,
        //     pq_shared_secret,
        //     noise_protocol,
        // ))
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
