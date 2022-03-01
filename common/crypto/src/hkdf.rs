// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hkdf::{hmac::digest::Digest, Hkdf};

/// Perform HKDF `extract` then `expand` as a single step.
pub fn extract_then_expand<D>(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: Option<&[u8]>,
    okm_length: usize,
) -> Result<Vec<u8>, hkdf::InvalidLength>
where
    D: Digest,
{
    // TODO: this would need to change if we ever needed the generated pseudorandom key, but
    // realistically I don't see any reasons why we might need it

    let hkdf = Hkdf::<D>::new(salt, ikm);
    let mut okm = vec![0u8; okm_length];
    hkdf.expand(info.unwrap_or(&[]), &mut okm)?;

    Ok(okm)
}
