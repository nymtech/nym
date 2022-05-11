// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::CHUNK_SIZE;
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve, HashToField};
use bls12_381::G1Projective;
use bls12_381::{G2Projective, Scalar};
use group::GroupEncoding;
use sha2::{Digest, Sha256};

#[macro_export]
macro_rules! ensure_len {
    ($a:expr, $b:expr) => {
        if $a.len() != $b {
            return false;
        }
    };
}

pub(crate) struct RandomOracleBuilder {
    inner_state: Sha256,
}

impl RandomOracleBuilder {
    pub(crate) fn new(domain: &[u8]) -> Self {
        let mut inner_state = Sha256::new();
        inner_state.update(domain);

        RandomOracleBuilder { inner_state }
    }

    pub(crate) fn update(&mut self, data: impl AsRef<[u8]>) {
        self.inner_state.update(data)
    }

    pub(crate) fn update_with_g1_elements<'a, I>(&mut self, items: I)
    where
        I: Iterator<Item = &'a G1Projective>,
    {
        items.for_each(|item| self.update(item.to_bytes()))
    }

    pub(crate) fn finalize(self) -> [u8; 32] {
        self.inner_state.finalize().into()
    }
}

// those will most likely need to somehow get re-combined with coconut (or maybe extracted to a completely different module)
pub(crate) fn hash_to_scalar<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> Scalar {
    // the unwrap here is fine as the result vector will have 1 element (as specified) and will not be empty
    hash_to_scalars(msg, domain, 1).pop().unwrap()
}

pub(crate) fn hash_to_scalars<M: AsRef<[u8]>>(msg: M, domain: &[u8], n: usize) -> Vec<Scalar> {
    let mut output = vec![Scalar::zero(); n];

    Scalar::hash_to_field::<ExpandMsgXmd<Sha256>>(msg.as_ref(), domain, &mut output);
    output
}

pub(crate) fn hash_g2<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> G2Projective {
    <G2Projective as HashToCurve<ExpandMsgXmd<Sha256>>>::hash_to_curve(msg, domain)
}

pub(crate) fn combine_scalar_chunks(chunks: &[Scalar]) -> Scalar {
    let chunk_size_scalar = Scalar::from(CHUNK_SIZE as u64);
    chunks.iter().rev().fold(Scalar::zero(), |mut acc, chunk| {
        acc *= chunk_size_scalar;
        acc += chunk;
        acc
    })
}

pub(crate) fn combine_g1_chunks(chunks: &[G1Projective]) -> G1Projective {
    let chunk_size_scalar = Scalar::from(CHUNK_SIZE as u64);
    chunks
        .iter()
        .rev()
        .fold(G1Projective::identity(), |mut acc, chunk| {
            acc *= chunk_size_scalar;
            acc += chunk;
            acc
        })
}

pub(crate) fn deserialize_scalar(b: &[u8]) -> Option<Scalar> {
    if b.len() != 32 {
        None
    } else {
        let mut repr: [u8; 32] = Default::default();
        repr.as_mut().copy_from_slice(b);
        Scalar::from_bytes(&repr).into()
    }
}

pub(crate) fn deserialize_g1(b: &[u8]) -> Option<G1Projective> {
    if b.len() != 48 {
        None
    } else {
        let mut encoding = <G1Projective as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(b);
        G1Projective::from_bytes(&encoding).into()
    }
}

pub(crate) fn deserialize_g2(b: &[u8]) -> Option<G2Projective> {
    if b.len() != 96 {
        None
    } else {
        let mut encoding = <G2Projective as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(b);
        G2Projective::from_bytes(&encoding).into()
    }
}
