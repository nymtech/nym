// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity::{PublicKey, Signature};

#[cfg(not(feature = "coconut"))]
pub struct TokenCredential {
    verification_key: PublicKey,
    gateway_identity: PublicKey,
    signature: Signature,
}

#[cfg(not(feature = "coconut"))]
impl TokenCredential {
    pub fn new(
        verification_key: PublicKey,
        gateway_identity: PublicKey,
        signature: Signature,
    ) -> Self {
        TokenCredential {
            verification_key,
            gateway_identity,
            signature,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.append(&mut self.verification_key.to_bytes().to_vec());
        message.append(&mut self.gateway_identity.to_bytes().to_vec());
        message.append(&mut self.signature.to_bytes().to_vec());
        message
    }
}
