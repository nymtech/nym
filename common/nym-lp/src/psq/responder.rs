// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTResponseData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::psk::psq_responder_process_message;
use crate::psq::helpers::{LpTransportHandshakeExt, current_timestamp};
use crate::psq::{LPSession, PSQHandshakeState};
use crate::{LpError, LpMessage};
use nym_kkt::KKT_RESPONSE_AAD;
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
use nym_kkt::encryption::{decrypt_initial_kkt_frame, encrypt_kkt_frame};
use nym_kkt::session::{responder_ingest_message, responder_process};
use nym_lp_transport::traits::LpTransport;
use std::time::Duration;

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

impl<'a, S> PSQHandshakeState<'a, S> {
    fn encapsulated_kem_keys(
        &self,
    ) -> Result<(DecapsulationKey<'_>, EncapsulationKey<'_>), LpError> {
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

    pub async fn psq_handshake_responder(mut self) -> Result<LPSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        // TODO: pass rng as argument
        let mut rng = rand09::rng();

        // 1. receive and validate ClientHello
        let client_hello_packet = self.connection.receive_packet(None).await?;
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

        let session_id = client_hello.receiver_index;
        let session_id_bytes = session_id.to_le_bytes();
        let version = client_hello_packet.header.protocol_version;

        // 2. send ack
        let ack = self.next_packet(session_id, version, LpMessage::Ack);
        let kkt_request = match self.send_and_receive_packet(ack, None).await?.message {
            LpMessage::KKTRequest(request) => request.0,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::KKTRequest,
                ));
            }
        };

        let kem_key = self.local_peer.get_kem_key_handle()?;
        // TEMP \/
        // Convert X25519 public key to KEM format for KKT response
        // infallible
        #[allow(clippy::unwrap_used)]
        let libcrux_kem =
            libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, kem_key.as_bytes())
                .unwrap();
        let encapsulation_key = EncapsulationKey::X25519(libcrux_kem);
        // TEMP /\

        // 3. process KKT request
        let (session_secret, request_frame, remote_context) =
            decrypt_initial_kkt_frame(self.local_peer.x25519.private_key(), &kkt_request)?;
        let (context, _) = responder_ingest_message(&remote_context, None, None, &request_frame)?;
        let response_frame = responder_process(
            &context,
            request_frame.session_id(),
            self.local_peer.ed25519().private_key(),
            &encapsulation_key,
        )?;
        let encrypted_frame =
            encrypt_kkt_frame(&mut rng, &session_secret, &response_frame, KKT_RESPONSE_AAD)?;
        let lp_message = KKTResponseData::new(encrypted_frame).into();
        let lp_packet = self.next_packet(session_id, version, lp_message);

        // 4. send KKT response and wait for PSQ msg1
        let psq_msg1 = match self.send_and_receive_packet(lp_packet, None).await?.message {
            LpMessage::Handshake(response) => response.0,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Handshake,
                ));
            }
        };
        // 5. process PSQ msg1

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

        let (dec_key, enc_key) = self.encapsulated_kem_keys()?;

        // Decapsulate PSK from PSQ payload using X25519 as DHKEM
        let psq_result = psq_responder_process_message(
            self.local_peer.x25519.private_key(),
            &self.remote_peer.x25519_public,
            (&dec_key, &enc_key),
            &self.remote_peer.ed25519_public,
            psq_payload,
            &client_hello.salt,
            &session_id_bytes,
        )?;
        drop(enc_key);
        drop(dec_key);

        let psk = psq_result.psk;
        let psk_handle = psq_result.psk_handle;

        // TEMP \/
        let outer_aead_key = OuterAeadKey::from_psk(&psk);
        // TEMP /\

        let noise_state = snow::Builder::new(crate::NOISE_PATTERN.parse()?)
            .local_private_key(self.local_peer.x25519().private_key().as_bytes())
            .remote_public_key(self.remote_peer.x25519_public.as_bytes())
            .psk(crate::NOISE_PSK_INDEX, &psk)
            .build_responder()?;
        let mut noise_protocol = NoiseProtocol::new(noise_state);
        noise_protocol.read_message(noise_payload)?;

        // 6. generate PSQ msg2
        let msg2 = noise_protocol
            .get_bytes_to_send()
            .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg2"))??;
        // Embed PSK handle in message: [u16 handle_len][handle_bytes][noise_msg]
        let handle_len = psk_handle.len() as u16;
        let mut combined = Vec::with_capacity(2 + psk_handle.len() + msg2.len());
        combined.extend_from_slice(&handle_len.to_le_bytes());
        combined.extend_from_slice(&psk_handle);
        combined.extend_from_slice(&msg2);

        let lp_message = HandshakeData::new(combined).into();
        let lp_packet = self.next_packet(session_id, version, lp_message);

        // 7. send msg2 and receive PSQ msg3
        let psq_msg3 = match self
            .send_and_receive_packet(lp_packet, Some(&outer_aead_key))
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
        assert!(noise_protocol.is_handshake_finished());

        // 8. [optionally] send ACK to finalise
        let ack = self.next_packet(session_id, version, LpMessage::Ack);
        self.connection
            .send_packet(ack, Some(&outer_aead_key))
            .await?;

        Ok(LPSession { outer_aead_key })
    }
}
