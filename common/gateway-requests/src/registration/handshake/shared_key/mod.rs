// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GatewayRequestsError;
use nym_crypto::generic_array::{typenum::Unsigned, GenericArray};
use nym_crypto::symmetric::aead::{self, AeadKey, KeySizeUser, Nonce};
use nym_pemstore::traits::PemStorableKey;
use nym_sphinx::params::GatewayEncryptionAlgorithm;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub mod legacy;

#[derive(Debug, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SharedSymmetricKey(AeadKey<GatewayEncryptionAlgorithm>);

type KeySize = <GatewayEncryptionAlgorithm as KeySizeUser>::KeySize;

#[derive(Debug, Clone, Copy, Error)]
pub enum SharedKeyConversionError {
    #[error("the string representation of the shared key was malformed: {0}")]
    DecodeError(#[from] bs58::decode::Error),
    #[error(
        "the received shared keys had invalid size. Got: {received}, but expected: {expected}"
    )]
    InvalidSharedKeysSize { received: usize, expected: usize },
}

impl SharedSymmetricKey {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, SharedKeyConversionError> {
        if bytes.len() != KeySize::to_usize() {
            return Err(SharedKeyConversionError::InvalidSharedKeysSize {
                received: bytes.len(),
                expected: KeySize::to_usize(),
            });
        }

        Ok(SharedSymmetricKey(GenericArray::clone_from_slice(bytes)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.iter().copied().collect()
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, SharedKeyConversionError> {
        let bs58_str = Zeroizing::new(val.into());
        let decoded = Zeroizing::new(bs58::decode(bs58_str).into_vec()?);
        Self::try_from_bytes(&decoded)
    }

    pub fn to_base58_string(&self) -> String {
        let bytes = Zeroizing::new(self.to_bytes());
        bs58::encode(bytes).into_string()
    }

    pub fn encrypt(
        &self,
        plaintext: &[u8],
        nonce: &Nonce<GatewayEncryptionAlgorithm>,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        aead::encrypt::<GatewayEncryptionAlgorithm>(&self.0, &nonce, plaintext).map_err(Into::into)
    }

    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: &Nonce<GatewayEncryptionAlgorithm>,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        aead::decrypt::<GatewayEncryptionAlgorithm>(&self.0, &nonce, ciphertext).map_err(Into::into)
    }
}

impl PemStorableKey for SharedSymmetricKey {
    type Error = SharedKeyConversionError;

    fn pem_type() -> &'static str {
        "AES-256-GCM-SIV GATEWAY SHARED KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}
