// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// | 0 | 1 | 2, 3, 4, 5 | 6 | 7
// [0] => KKT version (4 bits) + Message Sequence Count (4 bits)
// [1] => Status (3 bits) + Mode (3 bits) + Role (2 bits)
// [2..=5] => Ciphersuite
// [6] => Reserved

use crate::message::{DecryptedRequestFrame, KKTRequest, KKTRequestPlaintext};
use crate::{
    carrier::Carrier,
    context::{KKT_CONTEXT_LEN, KKTContext},
    error::KKTError,
};
use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
use rand09::{CryptoRng, RngCore};

pub(crate) const KKT_CARRIER_CONTEXT: &[u8] = b"CARRIER_V1_KKT_V1_KDF";

#[derive(Debug, PartialEq, Clone)]
pub struct KKTFrame {
    context: KKTContext,
    body: Vec<u8>,
}

// if oneway and message coming from initiator => body is empty.
// if mutual and message coming from initiator => body has the initiator's kem public key.
// if coming from responder => body has the responder's kem public key.

impl KKTFrame {
    pub fn new(context: KKTContext, body: &[u8]) -> Self {
        Self {
            context,
            body: Vec::from(body),
        }
    }

    pub fn context(&self) -> &KKTContext {
        &self.context
    }

    pub fn encrypt_initiator_frame<R>(
        self,
        rng: &mut R,
        responder_public_key: &DHPublicKey,
        version_byte: u8,
    ) -> Result<(Carrier, KKTRequest), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let ephemeral_keypair = DHKeyPair::new(rng);

        let plaintext =
            KKTRequestPlaintext::new(ephemeral_keypair.pk, responder_public_key, version_byte);

        let mut carrier =
            plaintext.derive_initiator_carrier(ephemeral_keypair.sk(), responder_public_key)?;
        let full_kkt_message = plaintext.into_message(&mut carrier, self)?;

        Ok((carrier, full_kkt_message))
    }

    pub fn decrypt_initiator_frame(
        responder_keypair: &DHKeyPair,
        message: KKTRequest,
        supported_versions: &[u8],
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
        let frame = KKTFrame::from_bytes(&decrypted_message)?;

        Ok(DecryptedRequestFrame {
            carrier,
            remote_frame: frame,
            outer_protocol_version,
        })
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
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KKTError> {
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

        if bytes.len() != context.full_message_len() {
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

        Ok(KKTFrame::new(context, &body))
    }
}
