// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::authentication::iv::AuthenticationIV;
use crate::registration::handshake::shared_key::SharedKeys;
use crypto::symmetric::stream_cipher;
use nymsphinx::params::GatewayEncryptionAlgorithm;
use nymsphinx::{DestinationAddressBytes, DESTINATION_ADDRESS_LENGTH};

pub const ENCRYPTED_ADDRESS_SIZE: usize = DESTINATION_ADDRESS_LENGTH;

/// Replacement for what used to be an 'AuthToken'. We used to be generating an 'AuthToken' based on
/// local secret and remote address in order to allow for authentication. Due to changes in registration
/// and the fact we are deriving a shared key, we are encrypting remote's address with the previously
/// derived shared key. If the value is as expected, then authentication is successful.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EncryptedAddressBytes([u8; ENCRYPTED_ADDRESS_SIZE]);

#[derive(Debug)]
pub enum EncryptedAddressConversionError {
    DecodeError(bs58::decode::Error),
    StringOfInvalidLengthError,
}

impl EncryptedAddressBytes {
    pub fn new(address: &DestinationAddressBytes, key: &SharedKeys, iv: &AuthenticationIV) -> Self {
        let ciphertext = stream_cipher::encrypt::<GatewayEncryptionAlgorithm>(
            key.encryption_key(),
            iv.inner(),
            address.as_bytes(),
        );

        let mut enc_address = [0u8; ENCRYPTED_ADDRESS_SIZE];
        enc_address.copy_from_slice(&ciphertext[..]);
        EncryptedAddressBytes(enc_address)
    }

    pub fn verify(
        &self,
        address: &DestinationAddressBytes,
        key: &SharedKeys,
        iv: &AuthenticationIV,
    ) -> bool {
        self == &Self::new(address, key, iv)
    }

    pub fn from_bytes(bytes: [u8; ENCRYPTED_ADDRESS_SIZE]) -> Self {
        EncryptedAddressBytes(bytes)
    }

    pub fn to_bytes(&self) -> [u8; ENCRYPTED_ADDRESS_SIZE] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, EncryptedAddressConversionError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(err) => return Err(EncryptedAddressConversionError::DecodeError(err)),
        };

        if decoded.len() != ENCRYPTED_ADDRESS_SIZE {
            return Err(EncryptedAddressConversionError::StringOfInvalidLengthError);
        }

        let mut enc_address = [0u8; ENCRYPTED_ADDRESS_SIZE];
        enc_address.copy_from_slice(&decoded[..]);
        Ok(EncryptedAddressBytes(enc_address))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.0).into_string()
    }
}

impl Into<String> for EncryptedAddressBytes {
    fn into(self) -> String {
        self.to_base58_string()
    }
}
