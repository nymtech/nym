// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// | 0 | 1 | 2, 3, 4, 5 | 6 | 7
// [0] => KKT version (4 bits) + Message Sequence Count (4 bits)
// [1] => Status (3 bits) + Mode (3 bits) + Role (2 bits)
// [2..=5] => Ciphersuite
// [6] => Reserved

use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
use nym_kkt_ciphersuite::x25519::PUBLIC_KEY_LENGTH;
use rand09::{CryptoRng, RngCore};

use crate::{
    carrier::Carrier,
    context::{KKT_CONTEXT_LEN, KKTContext},
    error::KKTError,
    masked_byte::{MASKED_BYTE_LEN, MaskedByte},
};

const KKT_CARRIER_CONTEXT: &[u8] = b"CARRIER_V1_KKT_V1_KDF";

#[derive(Debug, PartialEq, Clone)]
pub struct KKTFrame {
    context: [u8; KKT_CONTEXT_LEN],
    body: Vec<u8>,
}

// if oneway and message coming from initiator => body is empty.
// if mutual and message coming from initiator => body has the initiator's kem public key.
// if coming from responder => body has the responder's kem public key.

impl KKTFrame {
    pub fn new(context: &KKTContext, body: &[u8]) -> Result<Self, KKTError> {
        let context_bytes = context.encode()?;
        Ok(Self {
            context: context_bytes,
            body: Vec::from(body),
        })
    }

    pub fn encrypt_initiator_frame<R>(
        &self,
        rng: &mut R,
        responder_public_key: &DHPublicKey,
        version_byte: u8,
    ) -> Result<(Carrier, Vec<u8>), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let ephemeral_keypair = DHKeyPair::new(rng);
        let shared_secret = ephemeral_keypair
            .sk()
            .diffie_hellman(responder_public_key)
            .map_err(|_| KKTError::X25519Error {
                info: "Key Derivation Error",
            })?;

        let mut mask = Vec::from(ephemeral_keypair.pk.as_ref());
        mask.extend_from_slice(responder_public_key.as_ref());

        let masked_byte = MaskedByte::new(version_byte, &mask);

        let mut context = Vec::from(masked_byte.as_slice());
        context.extend_from_slice(KKT_CARRIER_CONTEXT);
        context.extend_from_slice(ephemeral_keypair.pk.as_ref());
        context.extend_from_slice(responder_public_key.as_ref());

        let mut carrier = Carrier::from_secret_slice(shared_secret.as_ref(), &context);

        let mut full_kkt_message = Vec::from(ephemeral_keypair.pk.as_ref());
        full_kkt_message.extend_from_slice(masked_byte.as_slice());
        let encrypted_kkt_frame = carrier.encrypt(&self.to_bytes())?;
        full_kkt_message.extend_from_slice(&encrypted_kkt_frame);

        Ok((carrier, full_kkt_message))
    }

    pub fn decrypt_initiator_frame(
        responder_keypair: &DHKeyPair,
        message: &[u8],
        supported_versions: &[u8],
    ) -> Result<(Carrier, KKTFrame, KKTContext), KKTError> {
        let mut initiator_public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = [0; PUBLIC_KEY_LENGTH];
        initiator_public_key_bytes.clone_from_slice(&message[0..PUBLIC_KEY_LENGTH]);

        // check mask

        let masked_byte =
            MaskedByte::try_from(&message[PUBLIC_KEY_LENGTH..PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN])?;

        let mut mask = Vec::from(&initiator_public_key_bytes);
        mask.extend_from_slice(responder_keypair.pk.as_ref());

        // this could be used later when we have multiple versions
        // if this call fails, it does before the server has to run a DH
        let _outer_protocol_version =
            masked_byte.unmask_check_version(&mask, supported_versions)?;

        // now that the version is ok, we can try dh

        let initiator_public_key = DHPublicKey::from_bytes(&initiator_public_key_bytes);

        let shared_secret = responder_keypair
            .sk()
            .diffie_hellman(&initiator_public_key)
            .map_err(|_| KKTError::X25519Error {
                info: "Key Derivation Error",
            })?;

        let mut context = Vec::from(masked_byte.as_slice());
        context.extend_from_slice(KKT_CARRIER_CONTEXT);
        context.extend_from_slice(initiator_public_key.as_ref());
        context.extend_from_slice(responder_keypair.pk.as_ref());

        let mut carrier = Carrier::from_secret_slice(shared_secret.as_ref(), &context).flip_keys();

        let decrypted_message = carrier.decrypt(&message[PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN..])?;
        let (frame, context) = KKTFrame::from_bytes(&decrypted_message)?;

        Ok((carrier, frame, context))
    }

    pub fn context_ref(&self) -> &[u8] {
        &self.context
    }

    pub fn context(&self) -> Result<KKTContext, KKTError> {
        KKTContext::try_decode(self.context)
    }

    pub fn body_ref(&self) -> &[u8] {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.body
    }

    pub fn frame_length(&self) -> usize {
        self.context.len() + self.body.len()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.frame_length());
        bytes.extend_from_slice(&self.context);
        bytes.extend_from_slice(&self.body);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, KKTContext), KKTError> {
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

        let frame = KKTFrame::new(&context, &body)?;
        Ok((frame, context))
    }
}
