// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// this entire module is a big placeholder for whatever scheme we decide to use for the
// secure channel encryption scheme, but I would assume that the top-level API would
// remain more or less the same

use crate::error::DkgError;
use crate::Share;
use zeroize::Zeroize;

// To be determined
const PUBLIC_KEY_LENGTH: usize = 32;

pub struct KeyPair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PublicKey;

impl PublicKey {
    pub fn encrypt_share(&self, share: &Share) -> Ciphertext {
        todo!()
    }

    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_LENGTH] {
        Default::default()
        // self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, DkgError> {
        Err(DkgError::MalformedPublicKey)
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, DkgError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|_| DkgError::MalformedPublicKey)?;
        Self::from_bytes(&bytes)
    }
}

#[derive(Zeroize)]
pub struct SecretKey;

impl SecretKey {
    pub fn decrypt_share(&self, ciphertext: &Ciphertext) -> Result<Share, DkgError> {
        todo!()
    }
}

pub struct Ciphertext;

impl Ciphertext {
    pub fn decrypt_share(&self, sk: &SecretKey) -> Result<Share, DkgError> {
        sk.decrypt_share(self)
    }
}
