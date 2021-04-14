// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::generic_array::{typenum::Unsigned, GenericArray};
use crypto::symmetric::stream_cipher::{random_iv, NewStreamCipher, IV};
use nymsphinx::params::GatewayEncryptionAlgorithm;
use rand::{CryptoRng, RngCore};

type NonceSize = <GatewayEncryptionAlgorithm as NewStreamCipher>::NonceSize;

// I think 'IV' looks better than 'Iv', feel free to change that.
#[allow(clippy::upper_case_acronyms)]
pub struct AuthenticationIV(IV<GatewayEncryptionAlgorithm>);

#[derive(Debug)]
// I think 'IV' looks better than 'Iv', feel free to change that.
#[allow(clippy::upper_case_acronyms)]
pub enum IVConversionError {
    DecodeError(bs58::decode::Error),
    BytesOfInvalidLengthError,
    StringOfInvalidLengthError,
}

impl AuthenticationIV {
    pub fn new_random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        AuthenticationIV(random_iv::<GatewayEncryptionAlgorithm, _>(rng))
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, IVConversionError> {
        if bytes.len() != NonceSize::to_usize() {
            return Err(IVConversionError::BytesOfInvalidLengthError);
        }

        Ok(AuthenticationIV(GenericArray::clone_from_slice(bytes)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn inner(&self) -> &IV<GatewayEncryptionAlgorithm> {
        &self.0
    }

    pub fn try_from_base58_string<S: Into<String>>(val: S) -> Result<Self, IVConversionError> {
        let decoded = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(err) => return Err(IVConversionError::DecodeError(err)),
        };

        if decoded.len() != NonceSize::to_usize() {
            return Err(IVConversionError::StringOfInvalidLengthError);
        }

        Ok(AuthenticationIV(
            GenericArray::from_exact_iter(decoded).expect("Invalid vector length!"),
        ))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }
}

impl From<AuthenticationIV> for String {
    fn from(iv: AuthenticationIV) -> Self {
        iv.to_base58_string()
    }
}
