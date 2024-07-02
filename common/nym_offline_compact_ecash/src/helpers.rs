// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::try_deserialize_g1_projective;
use crate::CompactEcashError;
use bls12_381::G1Projective;
use group::Curve;
use std::any::{type_name, Any};

pub(crate) fn g1_tuple_to_bytes(el: (G1Projective, G1Projective)) -> [u8; 96] {
    let mut bytes = [0u8; 96];
    bytes[..48].copy_from_slice(&el.0.to_affine().to_compressed());
    bytes[48..].copy_from_slice(&el.1.to_affine().to_compressed());
    bytes
}

pub(crate) fn recover_g1_tuple<T: Any>(
    bytes: &[u8],
) -> crate::error::Result<(G1Projective, G1Projective)> {
    if bytes.len() != 96 {
        return Err(CompactEcashError::DeserializationLengthMismatch {
            type_name: type_name::<T>().into(),
            expected: 96,
            actual: bytes.len(),
        });
    }
    //SAFETY : [0..48] into 48 sized array and [48..96] into 48 sized array
    #[allow(clippy::unwrap_used)]
    let first_bytes: &[u8; 48] = &bytes[..48].try_into().unwrap();
    #[allow(clippy::unwrap_used)]
    let second_bytes: &[u8; 48] = &bytes[48..].try_into().unwrap();

    let first = try_deserialize_g1_projective(first_bytes)?;
    let second = try_deserialize_g1_projective(second_bytes)?;

    Ok((first, second))
}
