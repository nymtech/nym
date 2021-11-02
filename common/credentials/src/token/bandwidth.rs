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

    pub fn verification_key(&self) -> PublicKey {
        self.verification_key
    }

    pub fn gateway_identity(&self) -> PublicKey {
        self.gateway_identity
    }

    pub fn bandwidth(&self) -> u64 {
        self.bandwidth
    }

    pub fn signature_bytes(&self) -> [u8; 64] {
        self.signature.to_bytes()
    }

    pub fn verify_signature(&self) -> bool {
        let message: Vec<u8> = self
            .verification_key
            .to_bytes()
            .iter()
            .chain(self.gateway_identity.to_bytes().iter())
            .copied()
            .collect();
        self.verification_key
            .verify(&message, &self.signature)
            .is_ok()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.verification_key
            .to_bytes()
            .iter()
            .chain(self.gateway_identity.to_bytes().iter())
            .chain(self.bandwidth.to_be_bytes().iter())
            .chain(self.signature.to_bytes().iter())
            .copied()
            .collect()
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
