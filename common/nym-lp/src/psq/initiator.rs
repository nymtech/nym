// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::OuterAeadKey;
use crate::message::{HandshakeData, KKTRequestData, MessageType};
use crate::noise_protocol::NoiseProtocol;
use crate::psk::psq_initiator_create_message;
use crate::psq::helpers::current_timestamp;
use crate::psq::{LPSession, PSQHandshakeState};
use crate::{LpError, LpMessage};
use nym_kkt::KKT_RESPONSE_AAD;
use nym_kkt::encryption::{decrypt_kkt_frame, encrypt_initial_kkt_frame};
use nym_kkt::session::{anonymous_initiator_process, initiator_ingest_response};
use nym_lp_transport::traits::LpTransport;

impl<'a, S> PSQHandshakeState<'a, S>
where
    S: LpTransport + Unpin,
{
    // TODO: missing: receive counter check
    pub async fn psq_handshake_initiator(
        mut self,
        remote_protocol: u8,
    ) -> Result<LPSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        // TODO: pass rng as argument
        let mut rng = rand09::rng();

        // 0. retrieve the expected kem key hash. if we don't know if,
        // there's no point in even trying to start the handshake
        let expected_kem_key_digest = self.remote_peer.expected_kem_key_hash(self.ciphersuite)?;

        // 1. Generate and send ClientHelloData with fresh salt and both public keys
        let timestamp = current_timestamp()?;

        let client_hello_data = self.local_peer.build_client_hello_data(timestamp);
        let salt = client_hello_data.salt;
        let session_id = client_hello_data.receiver_index;
        let session_id_bytes = session_id.to_le_bytes();

        // 2. receive ack and verify we received Ack
        // this confirms that gateway is fine with our suggested protocol version
        // in the future we probably have some fancier negotiation
        match self
            .send_and_receive_packet(client_hello_data.into_lp_packet(remote_protocol), None)
            .await?
            .message
        {
            LpMessage::Ack => (),
            other => {
                // TODO: retry on collision

                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Ack,
                ));
            }
        }

        // 3. prepare and send KKT request
        let (kkt_context, kkt_frame) = anonymous_initiator_process(&mut rng, self.ciphersuite)?;
        let (session_secret, encrypted_frame) =
            encrypt_initial_kkt_frame(&mut rng, &self.remote_peer.x25519_public, &kkt_frame)?;
        let lp_message = KKTRequestData::new(encrypted_frame).into();
        let lp_packet = self.next_packet(session_id, remote_protocol, lp_message)?;

        // 4. receive and process KKT response
        let kkt_response = match self.send_and_receive_packet(lp_packet, None).await?.message {
            LpMessage::KKTResponse(response) => response,
            other => {
                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::KKTResponse,
                ));
            }
        };
        let (response_frame, remote_context) =
            decrypt_kkt_frame(&session_secret, &kkt_response.0, KKT_RESPONSE_AAD)?;
        let encapsulation_key = initiator_ingest_response(
            &kkt_context,
            &response_frame,
            &remote_context,
            &self.remote_peer.ed25519_public,
            &expected_kem_key_digest,
        )?;

        // 5. prepare and send PSQ msg1
        let psq_initiator = psq_initiator_create_message(
            self.local_peer.x25519.private_key(),
            &self.remote_peer.x25519_public,
            &encapsulation_key,
            self.local_peer.ed25519.private_key(),
            self.local_peer.ed25519.public_key(),
            &salt,
            &session_id_bytes,
        )?;
        let psk = psq_initiator.psk;
        let psq_payload = psq_initiator.payload;

        // TEMP \/
        let outer_aead_key = OuterAeadKey::from_psk(&psk);
        // TEMP /\

        // prepare noise state and msg1
        let noise_state = snow::Builder::new(crate::NOISE_PATTERN.parse()?)
            .local_private_key(self.local_peer.x25519().private_key().as_bytes())
            .remote_public_key(self.remote_peer.x25519_public.as_bytes())
            .psk(crate::NOISE_PSK_INDEX, &psk)
            .build_initiator()?;
        let mut noise_protocol = NoiseProtocol::new(noise_state);

        // prepare noise msg1
        let noise_msg1 = noise_protocol
            .get_bytes_to_send()
            .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg1"))??;
        let psq_len = psq_payload.len() as u16;
        let mut combined = Vec::with_capacity(2 + psq_payload.len() + noise_msg1.len());
        combined.extend_from_slice(&psq_len.to_le_bytes());
        combined.extend_from_slice(&psq_payload);
        combined.extend_from_slice(&noise_msg1);

        let lp_message = HandshakeData::new(combined).into();
        let lp_packet = self.next_packet(session_id, remote_protocol, lp_message)?;

        // 6. receive and process PSQ msg2
        let psq_msg2 = match self
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

        // Extract PSK handle: [u16 handle_len][handle_bytes][noise_msg]
        if psq_msg2.len() < 2 {
            return Err(LpError::kkt_psq_handshake("too short msg2 received"));
        }
        let handle_len = u16::from_le_bytes([psq_msg2[0], psq_msg2[1]]) as usize;
        if psq_msg2.len() < 2 + handle_len {
            return Err(LpError::kkt_psq_handshake("too short msg2 received"));
        }
        // Extract and "store" the PSK handle
        let _psq_handle_bytes = &psq_msg2[2..2 + handle_len];
        let noise_payload = &psq_msg2[2 + handle_len..];

        // *sigh* ignore the message
        let _noise_msg2 = noise_protocol.read_message(noise_payload)?;

        // 7. send PSQ msg3
        let noise_msg3 = noise_protocol
            .get_bytes_to_send()
            .ok_or_else(|| LpError::kkt_psq_handshake("failed to generate noise msg3"))??;

        let lp_message = HandshakeData::new(noise_msg3).into();
        let lp_packet = self.next_packet(session_id, remote_protocol, lp_message)?;

        match self
            .send_and_receive_packet(lp_packet, Some(&outer_aead_key))
            .await?
            .message
        {
            LpMessage::Ack => (),
            other => {
                // TODO: retry on collision

                return Err(LpError::unexpected_handshake_response(
                    other.typ(),
                    MessageType::Ack,
                ));
            }
        }

        todo!()
    }
}
