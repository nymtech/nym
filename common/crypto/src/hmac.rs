// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use digest::{BlockInput, FixedOutput, Reset, Update};
use generic_array::{typenum::Unsigned, ArrayLength, GenericArray};
use hmac::{crypto_mac, Hmac, Mac, NewMac};

pub use hmac;

// Type alias for ease of use so that it would not require explicit import of crypto_mac or Hmac
pub type HmacOutput<D> = crypto_mac::Output<Hmac<D>>;

/// Compute keyed hmac
pub fn compute_keyed_hmac<D>(key: &[u8], data: &[u8]) -> HmacOutput<D>
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    let mut hmac =
        Hmac::<D>::new_from_slice(key).expect("HMAC should be able to take key of any size!");
    hmac.update(data);
    hmac.finalize()
}

/// Compute keyed hmac and performs constant time equality check with the provided tag value.
pub fn recompute_keyed_hmac_and_verify_tag<D>(key: &[u8], data: &[u8], tag: &[u8]) -> bool
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    let mut hmac =
        Hmac::<D>::new_from_slice(key).expect("HMAC should be able to take key of any size!");
    hmac.update(data);
    // note, under the hood ct_eq is called
    hmac.verify(tag).is_ok()
}

/// Verifies tag of an hmac output.
pub fn verify_tag<D>(tag: &[u8], out: HmacOutput<D>) -> bool
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    if tag.len() != D::OutputSize::to_usize() {
        return false;
    }

    let tag_bytes = GenericArray::clone_from_slice(tag);
    let tag_out = HmacOutput::new(tag_bytes);
    // note, under the hood ct_eq is called
    out == tag_out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blake3;

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
