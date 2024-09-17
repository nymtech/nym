// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::symmetric::stream_cipher::{generate_key, CipherKey, KeySizeUser};
use nym_pemstore::traits::PemStorableKey;
use nym_sphinx_params::AckEncryptionAlgorithm;
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AckKey(CipherKey<AckEncryptionAlgorithm>);

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
        if bytes.len() != AckEncryptionAlgorithm::key_size() {
            return Err(AckKeyConversionError::BytesOfInvalidLengthError);
        }

        // Ok(AckKey(GenericArray::clone_from_slice(bytes)))
        Ok(AckKey(
            CipherKey::<AckEncryptionAlgorithm>::clone_from_slice(bytes),
        ))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &CipherKey<AckEncryptionAlgorithm> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    fn assert_zeroize<T: Zeroize>() {}

    #[test]
    fn ack_key_is_zeroized() {
        assert_zeroize::<AckKey>();
        assert_zeroize_on_drop::<AckKey>();
    }
}
