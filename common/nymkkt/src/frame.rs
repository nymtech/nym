// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// | 0 | 1 | 2, 3, 4, 5 | 6 | 7
// [0] => KKT version (4 bits) + Message Sequence Count (4 bits)
// [1] => Status (3 bits) + Mode (3 bits) + Role (2 bits)
// [2..=5] => Ciphersuite
// [6] => Reserved

use nym_crypto::asymmetric::x25519::PublicKey;

use crate::{
    context::{KKTContext, KKT_CONTEXT_LEN},
    error::KKTError,
};

pub const KKT_SESSION_ID_LEN: usize = 16;

pub struct KKTFrame {
    context: Vec<u8>,
    session_id: Vec<u8>,
    body: Vec<u8>,
    signature: Vec<u8>,
}

// if oneway and message coming from initiator => body is empty, signature contains signature of context + session id (64 bytes).
// if message coming from anonymous initiator => body is empty, there is no signature.
// if mutual and message coming from initiator => body has the initiator's kem public key and the signature is over the context + body + session_id.
// if coming from responder => body has the responder's kem public key and the signature is over the context + body + session_id.

impl KKTFrame {
    pub fn new(context: &[u8], body: &[u8], session_id: &[u8], signature: &[u8]) -> Self {
        Self {
            context: Vec::from(context),
            body: Vec::from(body),
            session_id: Vec::from(session_id),
            signature: Vec::from(signature),
        }
    }
    pub fn context_ref<'a>(&'a self) -> &'a [u8] {
        &self.context
    }
    pub fn signature_ref<'a>(&'a self) -> &'a [u8] {
        &self.signature
    }
    pub fn body_ref<'a>(&'a self) -> &'a [u8] {
        &self.body
    }

    pub fn session_id_ref<'a>(&'a self) -> &'a [u8] {
        &self.session_id
    }
    pub fn signature_mut<'a>(&'a mut self) -> &'a mut [u8] {
        &mut self.signature
    }
    pub fn body_mut<'a>(&'a mut self) -> &'a mut [u8] {
        &mut self.body
    }

    pub fn session_id_mut<'a>(&'a mut self) -> &'a mut [u8] {
        &mut self.session_id
    }

    pub fn frame_length(&self) -> usize {
        self.context.len() + self.session_id.len() + self.body.len() + self.signature.len()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.frame_length());
        bytes.extend_from_slice(&self.context);
        bytes.extend_from_slice(&self.body);
        bytes.extend_from_slice(&self.session_id);
        bytes.extend_from_slice(&self.signature);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, KKTContext), KKTError> {
        if bytes.len() < KKT_CONTEXT_LEN {
            return Err(KKTError::FrameDecodingError {
                info: format!(
                    "Frame is shorter than expected context length: actual {} != expected {}",
                    bytes.len(),
                    KKT_CONTEXT_LEN
                ),
            });
        } else {
            let context_bytes = Vec::from(&bytes[0..KKT_CONTEXT_LEN]);

            let context = KKTContext::try_decode(&context_bytes)?;

            let (mut session_id, mut body, mut signature): (Vec<u8>, Vec<u8>, Vec<u8>) =
                (vec![], vec![], vec![]);

            if bytes.len() == context.full_message_len() {
                if context.body_len() > 0 {
                    body.extend_from_slice(
                        &bytes[KKT_CONTEXT_LEN..KKT_CONTEXT_LEN + context.body_len()],
                    );
                }
                if context.session_id_len() > 0 {
                    session_id.extend_from_slice(
                        &bytes[KKT_CONTEXT_LEN + context.body_len()
                            ..KKT_CONTEXT_LEN + context.body_len() + context.session_id_len()],
                    );
                }
                if context.signature_len() > 0 {
                    signature.extend_from_slice(
                        &bytes[KKT_CONTEXT_LEN + context.body_len() + context.session_id_len()
                            ..KKT_CONTEXT_LEN
                                + context.body_len()
                                + context.session_id_len()
                                + context.signature_len()],
                    );
                }

                Ok((
                    KKTFrame::new(&context_bytes, &body, &session_id, &signature),
                    context,
                ))
            } else {
                return Err(KKTError::FrameDecodingError {
                    info: format!(
                        "Frame is shorter than expected: actual {} != expected {}",
                        bytes.len(),
                        context.full_message_len()
                    ),
                });
            }
        }
    }
}

pub struct EncryptedKKTRequest {
    ephermeral_key: PublicKey,
    body: Vec<u8>,
}
impl EncryptedKKTRequest {
    pub fn to_bytes(mut self) -> Vec<u8> {
        self.body.extend(self.ephermeral_key.to_bytes());
        self.body
    }
}

pub struct EncryptedKKTResponse {
    body: Vec<u8>,
}
