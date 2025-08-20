// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{AuthenticationFailure, GatewayRequestsError, SharedGatewayKey};
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::iter;
use std::time::Duration;
use subtle::ConstantTimeEq;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateRequest {
    #[serde(flatten)]
    pub content: AuthenticateRequestContent,

    pub request_signature: ed25519::Signature,

    #[serde(default)]
    pub debug_trace_id: Option<String>,
}

impl AuthenticateRequest {
    pub fn new(
        protocol_version: u8,
        shared_key: &SharedGatewayKey,
        identity_keys: &ed25519::KeyPair,
        debug_trace_id: Option<String>,
    ) -> Result<AuthenticateRequest, GatewayRequestsError> {
        let content = AuthenticateRequestContent::new(
            protocol_version,
            shared_key,
            *identity_keys.public_key(),
        )?;
        let plaintext = content.plaintext();
        let request_signature = identity_keys.private_key().sign(&plaintext);

        Ok(AuthenticateRequest {
            content,
            request_signature,
            debug_trace_id,
        })
    }

    pub fn verify_timestamp(
        &self,
        max_request_timestamp_skew: Duration,
    ) -> Result<(), AuthenticationFailure> {
        let now = OffsetDateTime::now_utc();
        if self.content.request_timestamp() < now - max_request_timestamp_skew {
            return Err(AuthenticationFailure::ExcessiveTimestampSkew {
                received: self.content.request_timestamp(),
                server: now,
            });
        }
        if self.content.request_timestamp() - max_request_timestamp_skew > now {
            return Err(AuthenticationFailure::ExcessiveTimestampSkew {
                received: self.content.request_timestamp(),
                server: now,
            });
        }
        Ok(())
    }

    pub fn ensure_timestamp_not_reused(
        &self,
        previous: OffsetDateTime,
    ) -> Result<(), AuthenticationFailure> {
        if self.content.request_timestamp() <= previous {
            return Err(AuthenticationFailure::RequestReuse);
        }
        Ok(())
    }

    pub fn verify_ciphertext(
        &self,
        shared_key: &SharedGatewayKey,
    ) -> Result<(), AuthenticationFailure> {
        let expected = shared_key.encrypt(
            self.content
                .client_identity
                .derive_destination_address()
                .as_bytes_ref(),
            Some(&self.content.nonce),
        )?;

        if !bool::from(expected.ct_eq(&self.content.address_ciphertext)) {
            return Err(AuthenticationFailure::MalformedCiphertext);
        }
        Ok(())
    }

    pub fn verify_signature(&self) -> Result<(), AuthenticationFailure> {
        let plaintext = self.content.plaintext();
        self.content
            .client_identity
            .verify(plaintext, &self.request_signature)
            .map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateRequestContent {
    pub protocol_version: u8,

    // this is identical to the client's address
    pub client_identity: ed25519::PublicKey,

    #[serde(with = "nym_serde_helpers::base64")]
    pub address_ciphertext: Vec<u8>,

    #[serde(with = "nym_serde_helpers::base64")]
    pub nonce: Vec<u8>,

    pub request_unix_timestamp: u64,
}

impl AuthenticateRequestContent {
    fn new(
        protocol_version: u8,
        shared_key: &SharedGatewayKey,
        client_identity: ed25519::PublicKey,
    ) -> Result<AuthenticateRequestContent, GatewayRequestsError> {
        let nonce = shared_key.random_nonce_or_iv();
        let destination_address = client_identity.derive_destination_address();

        let address_ciphertext =
            shared_key.encrypt(destination_address.as_bytes_ref(), Some(&nonce))?;
        let now = OffsetDateTime::now_utc();
        Ok(AuthenticateRequestContent {
            protocol_version,
            client_identity,
            address_ciphertext,
            nonce,
            request_unix_timestamp: now.unix_timestamp() as u64, // SAFETY: we're running this in post 1970...
        })
    }
}

impl AuthenticateRequestContent {
    pub fn plaintext(&self) -> Vec<u8> {
        iter::once(self.protocol_version)
            .chain(self.client_identity.to_bytes())
            .chain(self.address_ciphertext.iter().copied())
            .chain(self.nonce.iter().copied())
            .chain(self.request_unix_timestamp.to_be_bytes())
            .collect()
    }

    pub fn request_timestamp(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.request_unix_timestamp as i64)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
    }
}
