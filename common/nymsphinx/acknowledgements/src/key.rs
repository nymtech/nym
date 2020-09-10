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

use crypto::generic_array::{typenum::Unsigned, GenericArray};
use crypto::symmetric::stream_cipher::{generate_key, Key, NewStreamCipher};
use nymsphinx_params::AckEncryptionAlgorithm;
use pemstore::traits::PemStorableKey;
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};

pub struct AckKey(Key<AckEncryptionAlgorithm>);

#[derive(Debug)]
pub enum AckKeyConversionError {
    BytesOfInvalidLengthError,
}

impl Display for AckKeyConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AckKeyConversionError::BytesOfInvalidLengthError => {
                write!(f, "provided bytes have invalid length")
            }
        }
    }
}

impl std::error::Error for AckKeyConversionError {}

impl AckKey {
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        AckKey(generate_key::<AckEncryptionAlgorithm, _>(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, AckKeyConversionError> {
        if bytes.len() != <AckEncryptionAlgorithm as NewStreamCipher>::KeySize::to_usize() {
            return Err(AckKeyConversionError::BytesOfInvalidLengthError);
        }

        Ok(AckKey(GenericArray::clone_from_slice(bytes)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &Key<AckEncryptionAlgorithm> {
        &self.0
    }
}

impl PemStorableKey for AckKey {
    type Error = AckKeyConversionError;

    fn pem_type() -> &'static str {
        "AES-128-CTR ACKNOWLEDGEMENTS KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}
