// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// | 0 | 1 | 2, 3, 4, 5 | 6 | 7
// [0] => KKT version (4 bits) + Message Sequence Count (4 bits)
// [1] => Status (3 bits) + Mode (3 bits) + Role (2 bits)
// [2..=5] => Ciphersuite
// [6] => Reserved

use crate::{
    context::{KKT_CONTEXT_LEN, KKTContext},
    error::KKTError,
};

pub const KKT_SESSION_ID_LEN: usize = 16;

pub type KKTSessionId = [u8; KKT_SESSION_ID_LEN];

#[derive(Debug, PartialEq, Clone)]
pub struct KKTFrame {
    context: [u8; KKT_CONTEXT_LEN],
    session_id: KKTSessionId,
    body: Vec<u8>,
    signature: Vec<u8>,
}

// if oneway and message coming from initiator => body is empty, signature contains signature of context + session id (64 bytes).
// if message coming from anonymous initiator => body is empty, there is no signature.
// if mutual and message coming from initiator => body has the initiator's kem public key and the signature is over the context + body + session_id.
// if coming from responder => body has the responder's kem public key and the signature is over the context + body + session_id.

impl KKTFrame {
    pub fn new(
        context: [u8; KKT_CONTEXT_LEN],
        body: &[u8],
        session_id: [u8; KKT_SESSION_ID_LEN],
        signature: &[u8],
    ) -> Self {
        Self {
            context,
            body: Vec::from(body),
            session_id,
            signature: Vec::from(signature),
        }
    }
    pub fn context_ref(&self) -> &[u8] {
        &self.context
    }

    pub fn signature_ref(&self) -> &[u8] {
        &self.signature
    }

    pub fn body_ref(&self) -> &[u8] {
        &self.body
    }

    pub fn session_id_ref(&self) -> &[u8] {
        &self.session_id
    }
    pub fn session_id(&self) -> [u8; KKT_SESSION_ID_LEN] {
        self.session_id
    }

    pub fn signature_mut(&mut self) -> &mut [u8] {
        &mut self.signature
    }
    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.body
    }

    pub fn session_id_mut(&mut self) -> &mut [u8] {
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
        let mut signature = Vec::new();

        // decode body
        if context.body_len() > 0 {
            let body_bytes = &bytes[KKT_CONTEXT_LEN..KKT_CONTEXT_LEN + context.body_len()];
            body.extend_from_slice(&body_bytes);
        }

        let session_bytes = &bytes[KKT_CONTEXT_LEN + context.body_len()
            ..KKT_CONTEXT_LEN + context.body_len() + KKT_SESSION_ID_LEN];
        // SAFETY: we're using exactly KKT_SESSION_ID_LEN bytes and we checked for sufficient bytes
        #[allow(clippy::unwrap_used)]
        let session_id = session_bytes.try_into().unwrap();

        // // old code left for reference if session id becomes variable in length:
        // if context.session_id_len() > 0 {
        //     session_id.extend_from_slice(
        //         &bytes[KKT_CONTEXT_LEN + context.body_len()
        //             ..KKT_CONTEXT_LEN + context.body_len() + context.session_id_len()],
        //     );
        // }

        // decode signature
        if context.signature_len() > 0 {
            let signature_bytes = &bytes[KKT_CONTEXT_LEN + context.body_len() + KKT_SESSION_ID_LEN
                ..KKT_CONTEXT_LEN
                    + context.body_len()
                    + KKT_SESSION_ID_LEN
                    + context.signature_len()];
            signature.extend_from_slice(signature_bytes);
        }

        Ok((
            KKTFrame::new(context_bytes, &body, session_id, &signature),
            context,
        ))
    }
}
