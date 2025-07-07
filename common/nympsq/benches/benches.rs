// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use criterion::{criterion_group, criterion_main, Criterion};
use nym_crypto::asymmetric::ed25519;
use rand::prelude::*;

pub fn gen_ed25519_keypair(c: &mut Criterion) {
    c.bench_function("Generate Ed25519 Keypair", |b| {
        b.iter(|| {
            let mut s: [u8; 32] = [0u8; 32];
            rand::rng().fill_bytes(&mut s);
            ed25519::KeyPair::from_secret(s, 0)
        });
    });
}

pub fn gen_mlkem768_keypair(c: &mut Criterion) {
    c.bench_function("Generate MlKem768 Keypair", |b| {
        b.iter(|| {
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand::rng()).unwrap()
        });
    });
}

pub fn gen_x25519_keypair(c: &mut Criterion) {
    c.bench_function("Generate DHKem Keypair", |b| {
        b.iter(|| libcrux_kem::key_gen(libcrux_kem::Algorithm::X25519, &mut rand::rng()).unwrap());
    });
}

pub fn gen_xwing_keypair(c: &mut Criterion) {
    c.bench_function("Generate XWingKem Keypair", |b| {
        b.iter(|| {
            libcrux_kem::key_gen(libcrux_kem::Algorithm::XWingKemDraft06, &mut rand::rng()).unwrap()
        });
    });
}

criterion_group!(
    benches,
    gen_ed25519_keypair,
    gen_mlkem768_keypair,
    gen_x25519_keypair,
    gen_xwing_keypair
);
criterion_main!(benches);
