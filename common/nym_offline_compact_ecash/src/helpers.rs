// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::try_deserialize_g1_projective;
use crate::{CompactEcashError, EncodedDate, EncodedTicketType};
use bls12_381::{G1Projective, Scalar};
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

pub(crate) fn date_scalar(date: EncodedDate) -> Scalar {
    Scalar::from(date as u64)
}

// TODO: this will not work for **all** scalars,
// but timestamps have extremely (relatively speaking) limited range,
// so this should be fine
pub(crate) fn scalar_date(scalar: &Scalar) -> EncodedDate {
    let b = scalar.to_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]) as EncodedDate
}

pub(crate) fn type_scalar(t_type: EncodedTicketType) -> Scalar {
    Scalar::from(t_type as u64)
}

// TODO: this will not work for **all** scalars,
// but ticket types have extremely (relatively speaking) limited range,
// so this should be fine
pub(crate) fn scalar_type(scalar: &Scalar) -> EncodedTicketType {
    let b = scalar.to_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]) as EncodedTicketType
}
