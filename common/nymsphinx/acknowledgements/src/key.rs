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

use crypto::symmetric::aes_ctr::{
    generic_array::{typenum::Unsigned, GenericArray},
    Aes128Key, Aes128KeySize,
};
use pemstore::traits::PemStorableKey;
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

pub type AckKeySize = Aes128KeySize;

pub struct AckAes128Key(Aes128Key);

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

impl AckAes128Key {
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        AckAes128Key(crypto::symmetric::aes_ctr::generate_key(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, AckKeyConversionError> {
        if bytes.len() != AckKeySize::to_usize() {
            return Err(AckKeyConversionError::BytesOfInvalidLengthError);
        }

        Ok(AckAes128Key(GenericArray::clone_from_slice(bytes)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Deref for AckAes128Key {
    type Target = Aes128Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PemStorableKey for AckAes128Key {
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
