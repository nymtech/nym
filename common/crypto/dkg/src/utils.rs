// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve, HashToField};
use bls12_381::{G2Projective, Scalar};

// those will most likely need to somehow get re-combined with coconut (or maybe extracted to a completely different module)

pub(crate) fn hash_to_scalar<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> Scalar {
    let mut output = vec![Scalar::zero()];

    Scalar::hash_to_field::<ExpandMsgXmd<sha2::Sha256>>(msg.as_ref(), domain, &mut output);
    output[0]
}

pub(crate) fn hash_g2<M: AsRef<[u8]>>(msg: M, domain: &[u8]) -> G2Projective {
    <G2Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(msg, domain)
}
