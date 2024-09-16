// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::{SharedGatewayKey, SharedKeyUsageError};
use nym_sphinx::DestinationAddressBytes;
use thiserror::Error;

/// Replacement for what used to be an `AuthToken`. We used to be generating an `AuthToken` based on
/// local secret and remote address in order to allow for authentication. Due to changes in registration
/// and the fact we are deriving a shared key, we are encrypting remote's address with the previously
/// derived shared key. If the value is as expected, then authentication is successful.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
// this is no longer constant size due to the differences in ciphertext between aes128ctr and aes256gcm-siv (inclusion of tag)
pub struct EncryptedAddressBytes(Vec<u8>);

#[derive(Debug, Error)]
pub enum EncryptedAddressConversionError {
    #[error("Failed to decode the encrypted address - {0}")]
    DecodeError(#[from] bs58::decode::Error),
}

impl EncryptedAddressBytes {
    pub fn new(
        address: &DestinationAddressBytes,
        key: &SharedGatewayKey,
        nonce: &[u8],
    ) -> Result<Self, SharedKeyUsageError> {
        let ciphertext = key.encrypt_naive(address.as_bytes_ref(), Some(nonce))?;

        Ok(EncryptedAddressBytes(ciphertext))
    }

    pub fn verify(
        &self,
        address: &DestinationAddressBytes,
        key: &SharedGatewayKey,
        nonce: &[u8],
    ) -> bool {
        let Ok(reconstructed) = Self::new(address, key, nonce) else {
            return false;
        };
        self == &reconstructed
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, EncryptedAddressConversionError> {
        let decoded = bs58::decode(val.into()).into_vec()?;
        Ok(EncryptedAddressBytes(decoded))
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.0).into_string()
    }
}

impl From<EncryptedAddressBytes> for String {
    fn from(val: EncryptedAddressBytes) -> Self {
        val.to_base58_string()
    }
}
