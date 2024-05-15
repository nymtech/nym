// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::{G1Affine, G1Projective};
use group::GroupEncoding;

use crate::CompactEcashError;

pub trait Bytable
where
    Self: Sized,
{
    fn to_byte_vec(&self) -> Vec<u8>;

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError>;
}

pub trait Base58
where
    Self: Bytable,
{
    fn try_from_bs58<S: AsRef<str>>(x: S) -> Result<Self, CompactEcashError> {
        Self::try_from_byte_slice(&bs58::decode(x.as_ref()).into_vec().unwrap())
    }
    fn to_bs58(&self) -> String {
        bs58::encode(self.to_byte_vec()).into_string()
    }
}

impl Bytable for G1Projective {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().as_ref().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        let received = slice.len();
        let arr: Result<[u8; 48], _> = slice.try_into();
        let Ok(bytes) = arr else {
            return Err(CompactEcashError::UnexpectedArrayLength {
                typ: "G1Projective".to_string(),
                received,
                expected: 48,
            });
        };

        let maybe_g1 = G1Affine::from_compressed(&bytes);
        if maybe_g1.is_none().into() {
            Err(CompactEcashError::G1ProjectiveDeserializationFailure)
        } else {
            // safety: this unwrap is fine as we've just checked the element is not none
            #[allow(clippy::unwrap_used)]
            Ok(maybe_g1.unwrap().into())
        }
    }
}

impl Base58 for G1Projective {}
