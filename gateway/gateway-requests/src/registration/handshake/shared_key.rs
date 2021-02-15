// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::generic_array::{
    typenum::{Sum, Unsigned, U16},
    GenericArray,
};
use crypto::hmac::compute_keyed_hmac;
use crypto::symmetric::stream_cipher::{self, Key, NewStreamCipher, IV};
use nymsphinx::params::{GatewayEncryptionAlgorithm, GatewayIntegrityHmacAlgorithm};
use pemstore::traits::PemStorableKey;
use std::fmt::{self, Display, Formatter};

// shared key is as long as the encryption key and the MAC key combined.
pub type SharedKeySize = Sum<EncryptionKeySize, MacKeySize>;

// we're using 16 byte long key in sphinx, so let's use the same one here
type MacKeySize = U16;
type EncryptionKeySize = <GatewayEncryptionAlgorithm as NewStreamCipher>::KeySize;

/// Shared key used when computing MAC for messages exchanged between client and its gateway.
pub type MacKey = GenericArray<u8, MacKeySize>;

#[derive(Clone, Debug)]
pub struct SharedKeys {
    encryption_key: Key<GatewayEncryptionAlgorithm>,
    mac_key: MacKey,
}

#[derive(Debug)]
pub enum SharedKeyConversionError {
    DecodeError(bs58::decode::Error),
    BytesOfInvalidLengthError,
    StringOfInvalidLengthError,
}

impl Display for SharedKeyConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SharedKeyConversionError::DecodeError(err) => write!(
                f,
                "encountered error while decoding the byte sequence: {}",
                err
            ),
            SharedKeyConversionError::BytesOfInvalidLengthError => {
                write!(f, "provided bytes have invalid length")
            }
            SharedKeyConversionError::StringOfInvalidLengthError => {
                write!(f, "provided string has invalid length")
            }
        }
    }
}

impl std::error::Error for SharedKeyConversionError {}

impl SharedKeys {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, SharedKeyConversionError> {
        if bytes.len() != SharedKeySize::to_usize() {
            return Err(SharedKeyConversionError::BytesOfInvalidLengthError);
        }

        let encryption_key =
            GenericArray::clone_from_slice(&bytes[..EncryptionKeySize::to_usize()]);
        let mac_key = GenericArray::clone_from_slice(&bytes[EncryptionKeySize::to_usize()..]);

        Ok(SharedKeys {
            encryption_key,
            mac_key,
        })
    }

    /// Encrypts the provided data using the optionally provided initialisation vector,
    /// or a 0 value if nothing was given. Then it computes an integrity mac and concatenates it
    /// with the previously produced ciphertext.
    pub fn encrypt_and_tag(
        &self,
        data: &[u8],
        iv: Option<&IV<GatewayEncryptionAlgorithm>>,
    ) -> Vec<u8> {
        let encrypted_data = match iv {
            Some(iv) => stream_cipher::encrypt::<GatewayEncryptionAlgorithm>(
                self.encryption_key(),
                iv,
                data,
            ),
            None => {
                let zero_iv = stream_cipher::zero_iv::<GatewayEncryptionAlgorithm>();
                stream_cipher::encrypt::<GatewayEncryptionAlgorithm>(
                    self.encryption_key(),
                    &zero_iv,
                    data,
                )
            }
        };
        let mac =
            compute_keyed_hmac::<GatewayIntegrityHmacAlgorithm>(self.mac_key(), &encrypted_data);

        mac.into_bytes()
            .into_iter()
            .chain(encrypted_data.into_iter())
            .collect()
    }

    pub fn encryption_key(&self) -> &Key<GatewayEncryptionAlgorithm> {
        &self.encryption_key
    }

    pub fn mac_key(&self) -> &MacKey {
        &self.mac_key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.encryption_key
            .to_vec()
            .into_iter()
            .chain(self.mac_key.to_vec().into_iter())
            .collect()
    }

    pub fn try_from_base58_string<S: Into<String>>(
        val: S,
    ) -> Result<Self, SharedKeyConversionError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(err) => return Err(SharedKeyConversionError::DecodeError(err)),
        };

        if decoded.len() != SharedKeySize::to_usize() {
            return Err(SharedKeyConversionError::StringOfInvalidLengthError);
        }

        SharedKeys::try_from_bytes(&decoded)
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }
}

impl From<SharedKeys> for String {
    fn from(keys: SharedKeys) -> Self {
        keys.to_base58_string()
    }
}

impl PemStorableKey for SharedKeys {
    type Error = SharedKeyConversionError;

    fn pem_type() -> &'static str {
        // TODO: If common\nymsphinx\params\src\lib::GatewayIntegrityHmacAlgorithm changes
        // the pem type needs updating!
        "AES-128-CTR + HMAC-BLAKE3 GATEWAY SHARED KEYS"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}
