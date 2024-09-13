// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::generic_array::{typenum::Unsigned, GenericArray};
use nym_crypto::symmetric::stream_cipher::{random_iv, IvSizeUser, IV as CryptoIV};
use nym_sphinx::params::LegacyGatewayEncryptionAlgorithm;
use rand::{CryptoRng, RngCore};
use thiserror::Error;

type NonceSize = <LegacyGatewayEncryptionAlgorithm as IvSizeUser>::IvSize;

// I think 'IV' looks better than 'Iv', feel free to change that.
#[allow(clippy::upper_case_acronyms)]
pub struct IV(CryptoIV<LegacyGatewayEncryptionAlgorithm>);

#[derive(Error, Debug)]
// I think 'IV' looks better than 'Iv', feel free to change that.
#[allow(clippy::upper_case_acronyms)]
pub enum IVConversionError {
    #[error("Failed to decode the iv - {0}")]
    DecodeError(#[from] bs58::decode::Error),

    #[error("The decoded bytes iv has invalid length")]
    BytesOfInvalidLengthError,

    #[error("The decoded string iv has invalid length")]
    StringOfInvalidLengthError,
}

impl IV {
    pub fn new_random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        IV(random_iv::<LegacyGatewayEncryptionAlgorithm, _>(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, IVConversionError> {
        if bytes.len() != NonceSize::to_usize() {
            return Err(IVConversionError::BytesOfInvalidLengthError);
        }

        Ok(IV(GenericArray::clone_from_slice(bytes)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &CryptoIV<LegacyGatewayEncryptionAlgorithm> {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(val: S) -> Result<Self, IVConversionError> {
        let decoded = bs58::decode(val.into()).into_vec()?;

        if decoded.len() != NonceSize::to_usize() {
            return Err(IVConversionError::StringOfInvalidLengthError);
        }

        Ok(IV(
            GenericArray::from_exact_iter(decoded).expect("Invalid vector length!")
        ))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }
}

impl From<IV> for String {
    fn from(iv: IV) -> Self {
        iv.to_base58_string()
    }
}
