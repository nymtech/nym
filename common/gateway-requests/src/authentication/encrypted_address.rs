// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::iv::IV;
use crate::registration::handshake::shared_key::SharedKeys;
use nym_crypto::symmetric::stream_cipher;
use nym_sphinx::params::GatewayEncryptionAlgorithm;
use nym_sphinx::{DestinationAddressBytes, DESTINATION_ADDRESS_LENGTH};
use thiserror::Error;

pub const ENCRYPTED_ADDRESS_SIZE: usize = DESTINATION_ADDRESS_LENGTH;

/// Replacement for what used to be an `AuthToken`.
///
/// Replacement for what used to be an `AuthToken`. We used to be generating an `AuthToken` based on
/// local secret and remote address in order to allow for authentication. Due to changes in registration
/// and the fact we are deriving a shared key, we are encrypting remote's address with the previously
/// derived shared key. If the value is as expected, then authentication is successful.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EncryptedAddressBytes([u8; ENCRYPTED_ADDRESS_SIZE]);

#[derive(Debug, Error)]
pub enum EncryptedAddressConversionError {
    #[error("Failed to decode the encrypted address - {0}")]
    DecodeError(#[from] bs58::decode::Error),
    #[error("The decoded address has invalid length")]
    StringOfInvalidLengthError,
}

impl EncryptedAddressBytes {
    pub fn new(address: &DestinationAddressBytes, key: &SharedKeys, iv: &IV) -> Self {
        let ciphertext = stream_cipher::encrypt::<GatewayEncryptionAlgorithm>(
            key.encryption_key(),
            iv.inner(),
            address.as_bytes_ref(),
        );

        let mut enc_address = [0u8; ENCRYPTED_ADDRESS_SIZE];
        enc_address.copy_from_slice(&ciphertext[..]);
        EncryptedAddressBytes(enc_address)
    }

    pub fn verify(&self, address: &DestinationAddressBytes, key: &SharedKeys, iv: &IV) -> bool {
        self == &Self::new(address, key, iv)
    }

    pub fn from_bytes(bytes: [u8; ENCRYPTED_ADDRESS_SIZE]) -> Self {
        EncryptedAddressBytes(bytes)
    }

    pub fn to_bytes(self) -> [u8; ENCRYPTED_ADDRESS_SIZE] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, EncryptedAddressConversionError> {
        let decoded = bs58::decode(val.into()).into_vec()?;

        if decoded.len() != ENCRYPTED_ADDRESS_SIZE {
            return Err(EncryptedAddressConversionError::StringOfInvalidLengthError);
        }

        let mut enc_address = [0u8; ENCRYPTED_ADDRESS_SIZE];
        enc_address.copy_from_slice(&decoded[..]);
        Ok(EncryptedAddressBytes(enc_address))
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
