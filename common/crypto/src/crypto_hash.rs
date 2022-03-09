// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use digest::{Digest, Output};

pub fn compute_digest<D>(data: &[u8]) -> Output<D>
where
    D: Digest,
{
    D::digest(data)
}
