// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{BlindedSignature, Signature};
use bls12_381::{G1Affine, G1Projective};
use group::GroupEncoding;

use crate::proofs::proof_spend::{SpendInstance, SpendProof};
use crate::proofs::proof_withdrawal::{WithdrawalReqInstance, WithdrawalReqProof};
use crate::scheme::withdrawal::RequestInfo;
use crate::scheme::{Payment, SerialNumber, Wallet};
use crate::{Attribute, CompactEcashError, PartialWallet, WithdrawalRequest};

#[macro_export]
macro_rules! impl_byteable_bs58 {
    ($typ:ident) => {
        impl $crate::traits::Bytable for $typ {
            fn to_byte_vec(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }

            fn try_from_byte_slice(slice: &[u8]) -> $crate::error::Result<Self> {
                Self::from_bytes(slice)
            }
        }

        impl $crate::traits::Base58 for $typ {}

        impl TryFrom<&[u8]> for $typ {
            type Error = CompactEcashError;

            fn try_from(bytes: &[u8]) -> $crate::error::Result<Self> {
                Self::from_bytes(bytes)
            }
        }
    };
}

macro_rules! impl_complex_binary_bytable {
    ($typ:ident) => {
        impl $typ {
            pub fn to_bytes(&self) -> Vec<u8> {
                use bincode::Options;

                // all of our manually derived types correctly serialise into bincode
                #[allow(clippy::unwrap_used)]
                crate::binary_serialiser().serialize(self).unwrap()
            }

            pub fn from_bytes(bytes: &[u8]) -> crate::error::Result<Self> {
                use bincode::Options;
                crate::binary_serialiser()
                    .deserialize(bytes)
                    .map_err(|source| CompactEcashError::BinaryDeserialisationFailure {
                        type_name: std::any::type_name::<$typ>().to_string(),
                        source,
                    })
            }
        }

        impl_byteable_bs58!($typ);
    };
}

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
        Self::try_from_byte_slice(&bs58::decode(x.as_ref()).into_vec()?)
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
        let bytes = slice
            .try_into()
            .map_err(|_| CompactEcashError::G1ProjectiveDeserializationFailure)?;

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

impl Bytable for Attribute {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        let maybe_attribute = Attribute::from_bytes(
            slice
                .try_into()
                .map_err(|_| CompactEcashError::ScalarDeserializationFailure)?,
        );
        if maybe_attribute.is_none().into() {
            Err(CompactEcashError::ScalarDeserializationFailure)
        } else {
            // safety: this unwrap is fine as we've just checked the element is not none
            #[allow(clippy::unwrap_used)]
            Ok(maybe_attribute.unwrap())
        }
    }
}

impl_byteable_bs58!(Signature);
impl_byteable_bs58!(BlindedSignature);
impl_byteable_bs58!(SerialNumber);
impl_byteable_bs58!(Wallet);
impl_byteable_bs58!(PartialWallet);

impl_complex_binary_bytable!(SpendProof);
impl_complex_binary_bytable!(SpendInstance);
impl_complex_binary_bytable!(WithdrawalReqProof);
impl_complex_binary_bytable!(WithdrawalReqInstance);
impl_complex_binary_bytable!(Payment);
impl_complex_binary_bytable!(WithdrawalRequest);
impl_complex_binary_bytable!(RequestInfo);
