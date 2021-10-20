// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

use crate::error::Error;
use std::convert::TryInto;

#[cfg(not(feature = "coconut"))]
pub struct TokenCredential {
    verification_key: PublicKey,
    gateway_identity: PublicKey,
    bandwidth: u64,
    signature: Signature,
}

#[cfg(not(feature = "coconut"))]
impl TokenCredential {
    pub fn new(
        verification_key: PublicKey,
        gateway_identity: PublicKey,
        bandwidth: u64,
        signature: Signature,
    ) -> Self {
        TokenCredential {
            verification_key,
            gateway_identity,
            bandwidth,
            signature,
        }
    }

    pub fn bandwidth(&self) -> u64 {
        self.bandwidth
    }

    pub fn verify(&self, _eth_endpoint: &str) -> bool {
        let mut message = Vec::new();
        message.append(&mut self.verification_key.to_bytes().to_vec());
        message.append(&mut self.gateway_identity.to_bytes().to_vec());
        self.verification_key
            .verify(&message, &self.signature)
            .is_ok()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.append(&mut self.verification_key.to_bytes().to_vec());
        message.append(&mut self.gateway_identity.to_bytes().to_vec());
        message.append(&mut self.bandwidth.to_be_bytes().to_vec());
        message.append(&mut self.signature.to_bytes().to_vec());
        message
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, Error> {
        if b.len() != 2 * PUBLIC_KEY_LENGTH + 8 + SIGNATURE_LENGTH {
            return Err(Error::BandwidthCredentialError);
        }
        let verification_key = PublicKey::from_bytes(&b[..PUBLIC_KEY_LENGTH])
            .map_err(|_| Error::BandwidthCredentialError)?;
        let gateway_identity = PublicKey::from_bytes(&b[PUBLIC_KEY_LENGTH..2 * PUBLIC_KEY_LENGTH])
            .map_err(|_| Error::BandwidthCredentialError)?;
        let bandwidth = u64::from_be_bytes(
            b[2 * PUBLIC_KEY_LENGTH..2 * PUBLIC_KEY_LENGTH + 8]
                .try_into()
                // unwrapping is safe because we know we have 8 bytes
                .unwrap(),
        );
        let signature = Signature::from_bytes(&b[2 * PUBLIC_KEY_LENGTH + 8..])
            .map_err(|_| Error::BandwidthCredentialError)?;
        Ok(TokenCredential {
            verification_key,
            gateway_identity,
            bandwidth,
            signature,
        })
    }
}
