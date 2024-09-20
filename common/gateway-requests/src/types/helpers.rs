// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    BinaryRequest, BinaryRequestKind, BinaryResponse, BinaryResponseKind, GatewayRequestsError,
    SharedGatewayKey,
};
use std::iter::once;

// each binary message consists of the following structure (for non-legacy messages)
// KIND || ENC_FLAG || MAYBE_NONCE || CIPHERTEXT/PLAINTEXT
// first byte is the kind of data to influence further serialisation/deseralisation
// second byte is a flag indicating whether the content is encrypted
// then it's followed by a pseudorandom nonce, assuming encryption is used
// finally, the rest of the message is the associated ciphertext or just plaintext (if message wasn't encrypted)
pub struct BinaryData<'a> {
    kind: u8,
    encrypted: bool,
    maybe_nonce: Option<&'a [u8]>,
    data: &'a [u8],
}

impl<'a> BinaryData<'a> {
    // serialises possibly encrypted data into bytes to be put on the wire
    pub fn into_raw(self, legacy: bool) -> Vec<u8> {
        if legacy {
            return self.data.to_vec();
        }

        let i = once(self.kind).chain(once(if self.encrypted { 1 } else { 0 }));
        if let Some(nonce) = self.maybe_nonce {
            i.chain(nonce.iter().copied())
                .chain(self.data.iter().copied())
                .collect()
        } else {
            i.chain(self.data.iter().copied()).collect()
        }
    }

    // attempts to perform basic parsing on bytes received from the wire
    pub fn from_raw(
        raw: &'a [u8],
        available_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        // if we're using legacy key, it's quite simple:
        // it's always encrypted with no nonce and the request/response kind is always 1
        if available_key.is_legacy() {
            return Ok(BinaryData {
                kind: 1,
                encrypted: true,
                maybe_nonce: None,
                data: raw,
            });
        }

        if raw.len() < 2 {
            return Err(GatewayRequestsError::TooShortRequest);
        }

        let kind = raw[0];
        let encrypted = if raw[1] == 1 {
            true
        } else if raw[1] == 0 {
            false
        } else {
            return Err(GatewayRequestsError::InvalidEncryptionFlag);
        };

        // if data is encrypted, there MUST be a nonce present for non-legacy keys
        if encrypted && raw.len() < available_key.nonce_size() + 2 {
            return Err(GatewayRequestsError::TooShortRequest);
        }

        Ok(BinaryData {
            kind,
            encrypted,
            maybe_nonce: Some(&raw[2..2 + available_key.nonce_size()]),
            data: &raw[2 + available_key.nonce_size()..],
        })
    }

    // attempt to encrypt plaintext of provided response/request and serialise it into wire format
    pub fn make_encrypted_blob(
        kind: u8,
        plaintext: &[u8],
        key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let maybe_nonce = key.random_nonce_or_zero_iv();

        let ciphertext = key.encrypt(plaintext, maybe_nonce.as_deref())?;
        Ok(BinaryData {
            kind,
            encrypted: true,
            maybe_nonce: maybe_nonce.as_deref(),
            data: &ciphertext,
        }
        .into_raw(key.is_legacy()))
    }

    // attempts to parse previously recovered bytes into a [`BinaryRequest`]
    pub fn into_request(
        self,
        key: &SharedGatewayKey,
    ) -> Result<BinaryRequest, GatewayRequestsError> {
        let kind = BinaryRequestKind::from_repr(self.kind)
            .ok_or(GatewayRequestsError::UnknownRequestKind { kind: self.kind })?;

        let plaintext = if self.encrypted {
            &*key.decrypt(self.data, self.maybe_nonce)?
        } else {
            self.data
        };

        BinaryRequest::from_plaintext(kind, plaintext)
    }

    // attempts to parse previously recovered bytes into a [`BinaryResponse`]
    pub fn into_response(
        self,
        key: &SharedGatewayKey,
    ) -> Result<BinaryResponse, GatewayRequestsError> {
        let kind = BinaryResponseKind::from_repr(self.kind)
            .ok_or(GatewayRequestsError::UnknownResponseKind { kind: self.kind })?;

        let plaintext = if self.encrypted {
            &*key.decrypt(self.data, self.maybe_nonce)?
        } else {
            self.data
        };

        BinaryResponse::from_plaintext(kind, plaintext)
    }
}
