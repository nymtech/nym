// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

use crate::error::Error;
use std::convert::TryInto;

pub struct TokenCredential {
    verification_key: PublicKey,
    gateway_identity: PublicKey,
    bandwidth: u64,
    signature: Signature,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_serde() {
        // pre-generated, valid values
        let verification_key = PublicKey::from_bytes(&[
            103, 105, 71, 177, 149, 245, 26, 32, 73, 121, 76, 50, 94, 88, 119, 231, 91, 229, 167,
            56, 39, 62, 185, 39, 83, 246, 153, 27, 17, 155, 109, 73,
        ])
        .unwrap();
        let gateway_identity = PublicKey::from_bytes(&[
            37, 113, 137, 189, 157, 82, 35, 2, 187, 136, 61, 119, 98, 5, 245, 82, 46, 124, 67, 45,
            165, 255, 53, 222, 185, 252, 6, 148, 128, 15, 206, 19,
        ])
        .unwrap();
        let signature = Signature::from_bytes(&[
            117, 251, 162, 217, 57, 2, 50, 210, 206, 81, 236, 90, 74, 201, 69, 237, 240, 247, 214,
            158, 220, 89, 235, 222, 85, 134, 73, 73, 8, 60, 25, 39, 183, 28, 83, 193, 31, 174, 25,
            24, 38, 215, 205, 228, 159, 135, 35, 4, 171, 59, 100, 157, 12, 249, 77, 52, 143, 4, 32,
            28, 147, 70, 182, 14,
        ])
        .unwrap();
        let credential = TokenCredential::new(verification_key, gateway_identity, 1024, signature);
        let serialized_credential = credential.to_bytes();
        let deserialized_credential = TokenCredential::from_bytes(&serialized_credential).unwrap();
        assert_eq!(
            credential.verification_key,
            deserialized_credential.verification_key
        );
        assert_eq!(
            credential.gateway_identity,
            deserialized_credential.gateway_identity
        );
        assert_eq!(credential.bandwidth, deserialized_credential.bandwidth);
        assert_eq!(
            credential.signature.to_bytes(),
            deserialized_credential.signature.to_bytes()
        );
    }
}
