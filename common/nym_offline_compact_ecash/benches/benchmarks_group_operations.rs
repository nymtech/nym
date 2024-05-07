// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Neg;
use std::time::Duration;

use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt, Scalar,
};
use criterion::{criterion_group, criterion_main, Criterion};
use ff::Field;
use group::{Curve, Group};
use nym_compact_ecash::utils::check_bilinear_pairing;

#[allow(unused)]
fn double_pairing(g11: &G1Affine, g21: &G2Affine, g12: &G1Affine, g22: &G2Affine) {
    let gt1 = bls12_381::pairing(g11, g21);
    let gt2 = bls12_381::pairing(g12, g22);
    assert_eq!(gt1, gt2)
}

#[allow(unused)]
fn single_pairing(g11: &G1Affine, g21: &G2Affine) {
    let gt1 = bls12_381::pairing(g11, g21);
}

#[allow(unused)]
fn exponent_in_g1(g1: G1Projective, r: Scalar) {
    let g11 = (g1 * r);
}

#[allow(unused)]
fn exponent_in_g2(g2: G2Projective, r: Scalar) {
    let g22 = (g2 * r);
}

#[allow(unused)]
fn exponent_in_gt(gt: Gt, r: Scalar) {
    let gtt = (gt * r);
}

#[allow(unused)]
fn multi_miller_pairing_affine(g11: &G1Affine, g21: &G2Affine, g12: &G1Affine, g22: &G2Affine) {
    let miller_loop_result = multi_miller_loop(&[
        (g11, &G2Prepared::from(*g21)),
        (&g12.neg(), &G2Prepared::from(*g22)),
    ]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

#[allow(unused)]
fn multi_miller_pairing_with_prepared(
    g11: &G1Affine,
    g21: &G2Prepared,
    g12: &G1Affine,
    g22: &G2Prepared,
) {
    let miller_loop_result = multi_miller_loop(&[(g11, g21), (&g12.neg(), g22)]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

// the case of being able to prepare G2 generator
#[allow(unused)]
fn multi_miller_pairing_with_semi_prepared(
    g11: &G1Affine,
    g21: &G2Affine,
    g12: &G1Affine,
    g22: &G2Prepared,
) {
    let miller_loop_result =
        multi_miller_loop(&[(g11, &G2Prepared::from(*g21)), (&g12.neg(), g22)]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

#[allow(unused)]
fn bench_group_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_group_operations");
    group.measurement_time(Duration::from_secs(200));

    let mut rng = rand::thread_rng();

    let g1 = G1Affine::generator();
    let g2 = G2Affine::generator();
    let r = Scalar::random(&mut rng);
    let s = Scalar::random(&mut rng);

    let g11 = (g1 * r).to_affine();
    let g21 = (g2 * s).to_affine();
    let g21_prep = G2Prepared::from(g21);

    let g12 = (g1 * s).to_affine();
    let g22 = (g2 * r).to_affine();
    let g22_prep = G2Prepared::from(g22);

    let gt = bls12_381::pairing(&g11, &g21);
    let gen1 = G1Projective::generator();
    let gen2 = G2Projective::generator();

    group.bench_function("exponent operation in G1", |b| {
        b.iter(|| exponent_in_g1(gen1, r))
    });

    group.bench_function("exponent operation in G2", |b| {
        b.iter(|| exponent_in_g2(gen2, r))
    });

    group.bench_function("exponent operation in Gt", |b| {
        b.iter(|| exponent_in_gt(gt, r))
    });

    group.bench_function("single pairing", |b| b.iter(|| single_pairing(&g11, &g21)));

    group.bench_function("double pairing", |b| {
        b.iter(|| double_pairing(&g11, &g21, &g12, &g22))
    });

    group.bench_function("multi miller in affine", |b| {
        b.iter(|| multi_miller_pairing_affine(&g11, &g21, &g12, &g22))
    });

    group.bench_function("multi miller with prepared g2", |b| {
        b.iter(|| multi_miller_pairing_with_prepared(&g11, &g21_prep, &g12, &g22_prep))
    });

    group.bench_function("multi miller with semi-prepared g2", |b| {
        b.iter(|| multi_miller_pairing_with_semi_prepared(&g11, &g21, &g12, &g22_prep))
    });

    // bench_checking_vk_pairing
    // assume key of size 5
    let scalars = [
        Scalar::random(&mut rng),
        Scalar::random(&mut rng),
        Scalar::random(&mut rng),
        Scalar::random(&mut rng),
        Scalar::random(&mut rng),
    ];
    let gen1 = G1Affine::generator();
    let gen2_prep = G2Prepared::from(G2Affine::generator());

    let g1 = scalars
        .iter()
        .map(|s| G1Affine::generator() * s)
        .collect::<Vec<_>>();
    let g2 = scalars
        .iter()
        .map(|s| G2Affine::generator() * s)
        .collect::<Vec<_>>();

    group.bench_function("individual pairings", |b| {
        b.iter(|| {
            for (g1, g2) in g1.iter().zip(g2.iter()) {
                let _ = check_bilinear_pairing(
                    &gen1,
                    &G2Prepared::from(g2.to_affine()),
                    &g1.to_affine(),
                    &gen2_prep,
                );
            }
        })
    });

    group.bench_function("miller loop with duplicate elements", |b| {
        b.iter(|| {
            let mut terms = vec![];
            let neg_g1 = gen1.neg();
            for (g1, g2) in g1.iter().zip(g2.iter()) {
                // TODO: optimise refs
                terms.push((neg_g1, G2Prepared::from(g2.to_affine())));
                terms.push((g1.to_affine(), gen2_prep.clone()));
            }
            let terms_refs = terms.iter().map(|(g1, g2)| (g1, g2)).collect::<Vec<_>>();

            let _: bool = multi_miller_loop(&terms_refs)
                .final_exponentiation()
                .is_identity()
                .into();
        })
    });
}

criterion_group!(benches, bench_group_operations);
criterion_main!(benches);
