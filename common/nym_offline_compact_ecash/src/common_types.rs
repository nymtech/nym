// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash_group_parameters;
use crate::error::Result;
use crate::helpers::{g1_tuple_to_bytes, recover_g1_tuple};
use bls12_381::{G1Projective, Scalar};
use serde::{Deserialize, Serialize};

pub type SignerIndex = u64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    pub(crate) h: G1Projective,
    pub(crate) s: G1Projective,
}

pub type PartialSignature = Signature;

impl Signature {
    pub(crate) fn sig1(&self) -> &G1Projective {
        &self.h
    }

    pub(crate) fn sig2(&self) -> &G1Projective {
        &self.s
    }

    /// Function randomises the signature.
    ///
    /// # Returns
    ///
    /// A tuple containing the randomised signature and the blinding scalar.
    pub fn blind_and_randomise(&self) -> (Signature, Scalar) {
        let params = ecash_group_parameters();

        // Generate random blinding scalars
        let r = params.random_scalar();
        let r_prime = params.random_scalar();

        // Calculate h_prime and s_prime using the random scalars
        let h_prime = self.h * r_prime;
        let s_prime = (self.s * r_prime) + (h_prime * r);
        (
            Signature {
                h: h_prime,
                s: s_prime,
            },
            r,
        )
    }

    pub fn to_bytes(self) -> [u8; 96] {
        g1_tuple_to_bytes((self.h, self.s))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (h, s) = recover_g1_tuple::<Self>(bytes)?;
        Ok(Signature { h, s })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlindedSignature {
    pub(crate) h: G1Projective,
    pub(crate) c: G1Projective,
}

impl BlindedSignature {
    pub fn to_bytes(self) -> [u8; 96] {
        g1_tuple_to_bytes((self.h, self.c))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (h, c) = recover_g1_tuple::<Self>(bytes)?;
        Ok(BlindedSignature { h, c })
    }
}

pub struct SignatureShare {
    signature: Signature,
    index: SignerIndex,
}

impl SignatureShare {
    pub fn new(signature: Signature, index: SignerIndex) -> Self {
        SignatureShare { signature, index }
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn index(&self) -> SignerIndex {
        self.index
    }
}
