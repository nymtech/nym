// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hmac::{
    digest::{crypto_common::BlockSizeUser, CtOutput, Digest, Output},
    Mac, SimpleHmac,
};

pub use hmac;

// TODO: We should probably change it to use some sealed trait to allow for both `Hmac` and `SimpleHmac`
pub type HmacOutput<D> = CtOutput<SimpleHmac<D>>;

/// Compute keyed hmac
pub fn compute_keyed_hmac<D>(key: &[u8], data: &[u8]) -> HmacOutput<D>
where
    D: Digest + BlockSizeUser,
{
    let mut hmac = SimpleHmac::<D>::new_from_slice(key)
        .expect("HMAC was instantiated with a key of an invalid size!");
    hmac.update(data);
    hmac.finalize()
}

/// Compute keyed hmac and performs constant time equality check with the provided tag value.
pub fn recompute_keyed_hmac_and_verify_tag<D>(key: &[u8], data: &[u8], tag: &[u8]) -> bool
where
    D: Digest + BlockSizeUser,
{
    let mut hmac = SimpleHmac::<D>::new_from_slice(key)
        .expect("HMAC was instantiated with a key of an invalid size!");
    hmac.update(data);

    let tag_arr = Output::<D>::from_slice(tag);
    // note, under the hood ct_eq is called
    hmac.verify(tag_arr).is_ok()
}

/// Verifies tag of an hmac output.
pub fn verify_tag<D>(tag: &[u8], out: HmacOutput<D>) -> bool
where
    D: Digest + BlockSizeUser,
{
    if tag.len() != <D as Digest>::output_size() {
        return false;
    }

    let tag_arr = Output::<D>::from_slice(tag);
    out == tag_arr.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: is it somehow possible to make the test not depend on blake3 specifically and
    // make it more generic?
    #[test]
    fn verifying_tags_work_using_both_methods_with_blake3() {
        let key = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let msg = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam sodales ultricies scelerisque.";

        // expected
        let output = compute_keyed_hmac::<blake3::Hasher>(&key, msg);
        let output_tag = output.into_bytes().to_vec();

        assert!(recompute_keyed_hmac_and_verify_tag::<blake3::Hasher>(
            &key,
            msg,
            &output_tag
        ));

        assert!(verify_tag::<blake3::Hasher>(
            &output_tag,
            compute_keyed_hmac::<blake3::Hasher>(&key, msg)
        ));
    }
}
