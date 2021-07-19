// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use digest::{BlockInput, Digest, FixedOutput, Reset, Update};
use generic_array::{ArrayLength, GenericArray};

pub fn compute_digest<D>(data: &[u8]) -> GenericArray<u8, <D as Digest>::OutputSize>
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    D::digest(data)
}
