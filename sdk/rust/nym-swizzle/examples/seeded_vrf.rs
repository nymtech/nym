// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Seeded (VRF-style) deterministic sampling: same seed, same result.
//!
//! All sampling in this crate defaults to OS entropy, but a fixed 32-byte seed
//! switches it to a deterministic ChaCha20 CSPRNG: the same seed and
//! configuration reproduce byte-identical plans and delay sequences.
//!
//! The crate has no VRF machinery on purpose. If you need *verifiable*
//! randomness, evaluate your VRF (ECVRF etc.) elsewhere and feed its output
//! in as the seed — the crate treats it as opaque seed material, exactly as
//! this example does with a stand-in "VRF output".
//!
//! Run with: `cargo run -p nym-swizzle --example seeded_vrf`

use nym_swizzle::Range;

/// Stand-in for a VRF output your application obtained (and could prove)
/// elsewhere.
const VRF_OUTPUT: [u8; 32] = [
    0x4e, 0x59, 0x4d, 0x21, 0x73, 0x77, 0x69, 0x7a, 0x7a, 0x6c, 0x65, 0x00, 0x01, 0x02, 0x03, 0x04,
    0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
];

fn plan_with_seed(seed: [u8; 32]) -> Vec<(u64, u64)> {
    Range::new(0, 500)
        .chunk_size(20..=80)
        .overlap(2..=10)
        .seed(seed)
        .plan()
        .collect()
}

fn main() {
    let first = plan_with_seed(VRF_OUTPUT);
    let second = plan_with_seed(VRF_OUTPUT);

    println!("run 1 (seeded from VRF output): {first:?}");
    println!();
    println!("run 2 (same seed):              {second:?}");
    assert_eq!(
        first, second,
        "identical seeds must produce identical plans"
    );
    println!();
    println!("=> identical, as promised");

    let mut different_seed = VRF_OUTPUT;
    different_seed[0] ^= 0xff;
    let third = plan_with_seed(different_seed);
    assert_ne!(first, third, "a different seed must diverge");
    println!("run 3 (different seed) diverges: {third:?}");
}
