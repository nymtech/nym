// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{CoconutError, Result};
use crate::traits::{Base58, Bytable};
use crate::utils::try_deserialize_g2_projective;
use bls12_381::{G2Affine, G2Projective};
use group::Curve;


use std::fmt::{Debug, Formatter};
use std::ops::Deref;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct BlindedSerialNumber(G2Projective);

// use custom Debug implementation to show base58 encoding (rather than raw curve elements)
impl Debug for BlindedSerialNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BlindedSerialNumber")
            .field(&self.to_bs58())
            .finish()
    }
}

impl From<G2Projective> for BlindedSerialNumber {
    fn from(value: G2Projective) -> Self {
        BlindedSerialNumber(value)
    }
}

impl From<G2Affine> for BlindedSerialNumber {
    fn from(value: G2Affine) -> Self {
        BlindedSerialNumber(value.into())
    }
}

impl Deref for BlindedSerialNumber {
    type Target = G2Projective;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<&[u8]> for BlindedSerialNumber {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 96 {
            return Err(
                CoconutError::Deserialization(
                    format!("Tried to deserialize blinded serial number with incorrect number of bytes, expected 96, got {}", bytes.len()),
                ));
        }

        // safety: we've just made a check for 96 bytes
        #[allow(clippy::unwrap_used)]
        let inner = try_deserialize_g2_projective(
            &bytes.try_into().unwrap(),
            CoconutError::Deserialization(
                "failed to deserialize the blinded serial number (zeta)".to_string(),
            ),
        )?;

        Ok(BlindedSerialNumber(inner))
    }
}

impl Bytable for BlindedSerialNumber {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.0.to_affine().to_compressed().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Self::try_from(slice)
    }
}

impl Base58 for BlindedSerialNumber {}
