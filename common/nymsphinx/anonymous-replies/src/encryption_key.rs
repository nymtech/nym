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

// pub use crypto::symmetric::aes_ctr::{
//     generic_array::{
//         typenum::{Unsigned, U32},
//         ArrayLength, GenericArray,
//     },
//     Aes128Key, Aes128KeySize,
// };

pub use crypto::generic_array::typenum::Unsigned;
use crypto::{
    crypto_hash,
    generic_array::GenericArray,
    symmetric::stream_cipher::{generate_key, Key, NewStreamCipher},
    Digest,
};
use nymsphinx_params::{ReplySURBEncryptionAlgorithm, ReplySURBKeyDigestAlgorithm};
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};

pub type EncryptionKeyDigest =
    GenericArray<u8, <ReplySURBKeyDigestAlgorithm as Digest>::OutputSize>;

pub type SURBEncryptionKeySize = <ReplySURBEncryptionAlgorithm as NewStreamCipher>::KeySize;

#[derive(Clone, Debug)]
pub struct SURBEncryptionKey(Key<ReplySURBEncryptionAlgorithm>);

#[derive(Debug)]
pub enum SURBEncryptionKeyError {
    BytesOfInvalidLengthError,
}

impl Display for SURBEncryptionKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SURBEncryptionKeyError::BytesOfInvalidLengthError => {
                write!(f, "provided bytes have invalid length")
            }
        }
    }
}

impl std::error::Error for SURBEncryptionKeyError {}

impl SURBEncryptionKey {
    /// Generates fresh pseudorandom key that is going to be used by the recipient of the message
    /// to encrypt payload of the reply. It is only generated when reply-SURB is attached.
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        SURBEncryptionKey(generate_key::<ReplySURBEncryptionAlgorithm, _>(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, SURBEncryptionKeyError> {
        if bytes.len() != SURBEncryptionKeySize::to_usize() {
            return Err(SURBEncryptionKeyError::BytesOfInvalidLengthError);
        }

        Ok(SURBEncryptionKey(GenericArray::clone_from_slice(bytes)))
    }

    pub fn compute_digest(&self) -> EncryptionKeyDigest {
        crypto_hash::compute_digest::<ReplySURBKeyDigestAlgorithm>(&self.0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &Key<ReplySURBEncryptionAlgorithm> {
        &self.0
    }
}
