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

use crypto::hmac::compute_keyed_hmac;
use crypto::hmac::hmac::digest::{BlockInput, FixedOutput, Reset, Update};
use crypto::symmetric::aes_ctr::{
    self,
    generic_array::{
        typenum::{Sum, Unsigned, U16},
        ArrayLength, GenericArray,
    },
    Aes128IV, Aes128Key, Aes128KeySize,
};
use pemstore::traits::PemStorableKey;
use std::fmt::{self, Display, Formatter};

// shared key is as long as the Aes128 key and the MAC key combined.
pub type SharedKeySize = Sum<Aes128KeySize, MacKeySize>;

// we're using 16 byte long key in sphinx, so let's use the same one here
type MacKeySize = U16;

/// Shared key used when computing MAC for messages exchanged between client and its gateway.
pub type MacKey = GenericArray<u8, MacKeySize>;

#[derive(Clone, Debug)]
pub struct SharedKeys {
    encryption_key: Aes128Key,
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

        let encryption_key = GenericArray::clone_from_slice(&bytes[..Aes128KeySize::to_usize()]);
        let mac_key = GenericArray::clone_from_slice(&bytes[Aes128KeySize::to_usize()..]);

        Ok(SharedKeys {
            encryption_key,
            mac_key,
        })
    }

    /// Encrypts the provided data using the optionally provided initialisation vector,
    /// or a 0 value if nothing was given. Then it computes an integrity mac and concatenates it
    /// with the previously produced ciphertext.
    pub fn encrypt_and_tag<D>(&self, data: &[u8], iv: Option<&Aes128IV>) -> Vec<u8>
    where
        D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
        D::BlockSize: ArrayLength<u8>,
        D::OutputSize: ArrayLength<u8>,
    {
        let encrypted_data = match iv {
            Some(iv) => aes_ctr::encrypt(self.encryption_key(), iv, data),
            None => aes_ctr::encrypt(self.encryption_key(), &aes_ctr::zero_iv(), data),
        };
        let mac = compute_keyed_hmac::<D>(self.mac_key(), &encrypted_data);

        mac.into_bytes()
            .into_iter()
            .chain(encrypted_data.into_iter())
            .collect()
    }

    pub fn encryption_key(&self) -> &Aes128Key {
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

impl Into<String> for SharedKeys {
    fn into(self) -> String {
        self.to_base58_string()
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
