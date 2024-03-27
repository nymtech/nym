// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use crate::CoconutError;
use bls12_381::{G1Affine, G1Projective, Scalar};
use group::GroupEncoding;

pub trait Bytable
where
    Self: Sized,
{
    fn to_byte_vec(&self) -> Vec<u8>;

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError>;
}

pub trait Base58
where
    Self: Bytable,
{
    fn try_from_bs58<S: AsRef<str>>(x: S) -> Result<Self, CoconutError> {
        let bs58_decoded = &bs58::decode(x.as_ref()).into_vec()?;
        Self::try_from_byte_slice(bs58_decoded)
    }
    fn to_bs58(&self) -> String {
        bs58::encode(self.to_byte_vec()).into_string()
    }
}

impl Bytable for Scalar {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError> {
        let received = slice.len();
        let Ok(arr) = slice.try_into() else {
            return Err(CoconutError::UnexpectedArrayLength {
                typ: "Scalar".to_string(),
                received,
                expected: 32,
            });
        };

        let maybe_scalar = Scalar::from_bytes(arr);
        if maybe_scalar.is_none().into() {
            Err(CoconutError::ScalarDeserializationFailure)
        } else {
            // safety: this unwrap is fine as we've just checked the element is not none
            #[allow(clippy::unwrap_used)]
            Ok(maybe_scalar.unwrap())
        }
    }
}

impl Base58 for Scalar {}

impl Bytable for G1Projective {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().as_ref().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError> {
        let received = slice.len();
        let arr: Result<[u8; 48], _> = slice.try_into();
        let Ok(bytes) = arr else {
            return Err(CoconutError::UnexpectedArrayLength {
                typ: "G1Projective".to_string(),
                received,
                expected: 48,
            });
        };

        let maybe_g1 = G1Affine::from_compressed(&bytes);
        if maybe_g1.is_none().into() {
            Err(CoconutError::G1ProjectiveDeserializationFailure)
        } else {
            // safety: this unwrap is fine as we've just checked the element is not none
            #[allow(clippy::unwrap_used)]
            Ok(maybe_g1.unwrap().into())
        }
    }
}

impl Base58 for G1Projective {}
