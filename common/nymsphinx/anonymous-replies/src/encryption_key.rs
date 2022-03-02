// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::{
    crypto_hash,
    generic_array::{typenum::Unsigned, GenericArray},
    symmetric::stream_cipher::{generate_key, CipherKey, KeySizeUser},
    OutputSizeUser,
};
use nymsphinx_params::{ReplySurbEncryptionAlgorithm, ReplySurbKeyDigestAlgorithm};
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};

pub type EncryptionKeyDigest =
    GenericArray<u8, <ReplySurbKeyDigestAlgorithm as OutputSizeUser>::OutputSize>;

pub type SurbEncryptionKeySize = <ReplySurbEncryptionAlgorithm as KeySizeUser>::KeySize;

#[derive(Clone, Debug)]
pub struct SurbEncryptionKey(CipherKey<ReplySurbEncryptionAlgorithm>);

#[derive(Debug)]
pub enum SurbEncryptionKeyError {
    BytesOfInvalidLengthError,
}

impl Display for SurbEncryptionKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SurbEncryptionKeyError::BytesOfInvalidLengthError => {
                write!(f, "provided bytes have invalid length")
            }
        }
    }
}

impl std::error::Error for SurbEncryptionKeyError {}

impl SurbEncryptionKey {
    /// Generates fresh pseudorandom key that is going to be used by the recipient of the message
    /// to encrypt payload of the reply. It is only generated when reply-SURB is attached.
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        SurbEncryptionKey(generate_key::<ReplySurbEncryptionAlgorithm, _>(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, SurbEncryptionKeyError> {
        if bytes.len() != SurbEncryptionKeySize::USIZE {
            return Err(SurbEncryptionKeyError::BytesOfInvalidLengthError);
        }

        Ok(SurbEncryptionKey(GenericArray::clone_from_slice(bytes)))
    }

    pub fn compute_digest(&self) -> EncryptionKeyDigest {
        crypto_hash::compute_digest::<ReplySurbKeyDigestAlgorithm>(&self.0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &CipherKey<ReplySurbEncryptionAlgorithm> {
        &self.0
    }
}
