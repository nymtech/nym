// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// | 0 | 1 | 2, 3, 4, 5 | 6 | 7
// [0] => KKT version (4 bits) + Message Sequence Count (4 bits)
// [1] => Status (3 bits) + Mode (3 bits) + Role (2 bits)
// [2..=5] => Ciphersuite
// [6] => Reserved

use crate::context::{KKTMode, KKTRole};
use crate::message::{
    DecryptedRequestFrame, KKTRequest, KKTRequestEncryptionResult, KKTRequestPlaintext,
};
use crate::{
    context::{KKT_CONTEXT_LEN, KKTContext},
    error::KKTError,
};
use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
use nym_crypto::blake3;
use nym_kkt_ciphersuite::KEM;
use rand09::{CryptoRng, RngCore};

pub(crate) const KKT_CARRIER_CONTEXT: &[u8] = b"CARRIER_V1_KKT_V1_KDF";
pub(crate) const KKT_RECEIVER_INDEX_CONTEXT: &[u8] = b"KKT_RECEIVER_INDEX_DERIVATION_V1";

#[derive(Debug, PartialEq, Clone)]
pub struct KKTFrame {
    context: KKTContext,
    body: Vec<u8>,
    payload: Vec<u8>,
}

// if oneway and message coming from initiator => body is empty.
// if mutual and message coming from initiator => body has the initiator's kem public key.
// if coming from responder => body has the responder's kem public key.

impl KKTFrame {
    pub fn new(context: KKTContext, body: &[u8], payload: Vec<u8>) -> Self {
        Self {
            context,
            body: Vec::from(body),
            payload,
        }
    }

    pub const fn size_excluding_payload(role: KKTRole, mode: KKTMode, kem: KEM) -> usize {
        match role {
            KKTRole::Initiator => {
                match mode {
                    KKTMode::OneWay => {
                        // if oneway and message coming from initiator => body is empty.
                        KKT_CONTEXT_LEN
                    }
                    KKTMode::Mutual => {
                        // if mutual and message coming from initiator => body has the initiator's kem public key.
                        KKT_CONTEXT_LEN + kem.encapsulation_key_length()
                    }
                }
            }
            KKTRole::Responder => {
                // if coming from responder => body has the responder's kem public key.
                KKT_CONTEXT_LEN + kem.encapsulation_key_length()
            }
        }
    }

    pub fn size(&self) -> usize {
        self.payload.len()
            + Self::size_excluding_payload(
                self.context.role(),
                self.context.mode(),
                self.context.ciphersuite().kem(),
            )
    }

    pub fn context(&self) -> &KKTContext {
        &self.context
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }

    pub fn encrypt_initiator_frame<R>(
        self,
        rng: &mut R,
        responder_public_key: &DHPublicKey,
        version_byte: u8,
    ) -> Result<KKTRequestEncryptionResult, KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let ephemeral_keypair = DHKeyPair::new(rng);

        let plaintext =
            KKTRequestPlaintext::new(ephemeral_keypair.pk, responder_public_key, version_byte);

        let receiver_index = self.derive_receiver_index(&plaintext)?;

        let mut carrier =
            plaintext.derive_initiator_carrier(ephemeral_keypair.sk(), responder_public_key)?;
        let full_kkt_message = plaintext.into_request(&mut carrier, self)?;

        Ok(KKTRequestEncryptionResult {
            carrier,
            receiver_index,
            request: full_kkt_message,
        })
    }

    pub fn decrypt_initiator_frame(
        responder_keypair: &DHKeyPair,
        message: KKTRequest,
        supported_versions: &[u8],
        request_payload_len: usize,
    ) -> Result<DecryptedRequestFrame, KKTError> {
        let mask = message.plaintext.version_mask(&responder_keypair.pk);

        // check mask
        // this could be used later when we have multiple versions
        // if this call fails, it does before the server has to run a DH
        let outer_protocol_version = message
            .plaintext
            .masked_version_bytes
            .unmask_check_version(&mask, supported_versions)?;

        // after verifying the version, we can perform the DH and continue processing the request
        let mut carrier = message
            .plaintext
            .derive_responder_carrier(responder_keypair)?;

        let decrypted_message = carrier.decrypt(&message.encrypted_frame)?;
        let frame = KKTFrame::from_bytes(&decrypted_message, request_payload_len)?;

        let receiver_index: u64 = frame.derive_receiver_index(&message.plaintext)?;

        Ok(DecryptedRequestFrame {
            carrier,
            remote_frame: frame,
            outer_protocol_version,
            receiver_index,
        })
    }

    // HASH(context || pub_key || masked_version || decrypted frame)
    fn derive_receiver_index(
        &self,
        kkt_outer_headers: &KKTRequestPlaintext,
    ) -> Result<u64, KKTError> {
        let mut receiver_index_bytes = [0u8; 8];

        let mut hasher = blake3::Hasher::new();

        hasher.update(KKT_RECEIVER_INDEX_CONTEXT);
        hasher.update(&kkt_outer_headers.to_bytes());
        hasher.update(&self.try_to_bytes()?);

        hasher.finalize_xof().fill(&mut receiver_index_bytes);

        Ok(u64::from_le_bytes(receiver_index_bytes))
    }

    pub fn body_ref(&self) -> &[u8] {
        &self.body
    }

    pub fn body(self) -> Vec<u8> {
        self.body
    }

    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.body
    }

    pub fn frame_length(&self) -> usize {
        KKT_CONTEXT_LEN + self.body.len()
    }

    pub fn try_to_bytes(&self) -> Result<Vec<u8>, KKTError> {
        let mut bytes = Vec::with_capacity(self.frame_length());
        bytes.extend_from_slice(&self.context.encode()?);
        bytes.extend_from_slice(&self.body);
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8], payload_len: usize) -> Result<Self, KKTError> {
        let len = bytes.len();
        if bytes.len() < KKT_CONTEXT_LEN {
            return Err(KKTError::FrameDecodingError {
                info: format!(
                    "Frame is shorter than expected context length: actual {len} != expected {KKT_CONTEXT_LEN}",
                ),
            });
        }

        // SAFETY: we're using exactly KKT_CONTEXT_LEN bytes
        #[allow(clippy::unwrap_used)]
        let context_bytes = bytes[0..KKT_CONTEXT_LEN].try_into().unwrap();
        let context = KKTContext::try_decode(context_bytes)?;

        if bytes.len() != context.full_message_len() + payload_len {
            return Err(KKTError::FrameDecodingError {
                info: format!(
                    "Frame is shorter than expected: actual {len} != expected {}",
                    context.full_message_len()
                ),
            });
        }

        let mut body = Vec::new();

        // decode body
        if context.body_len() > 0 {
            let body_bytes = &bytes[KKT_CONTEXT_LEN..KKT_CONTEXT_LEN + context.body_len()];
            body.extend_from_slice(body_bytes);
        }

        // decode payload. this could be empty.
        let payload: Vec<u8> = Vec::from(&bytes[KKT_CONTEXT_LEN + context.body_len()..]);

        Ok(KKTFrame::new(context, &body, payload))
    }
}
