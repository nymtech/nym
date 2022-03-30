use bls12_381::{G2Affine, G2Prepared};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dkg::bte::encryption::BabyStepGiantStepLookup;

pub fn precompute_default_bsgs_table(c: &mut Criterion) {
    c.bench_function("bsgs default table", |b| {
        b.iter(|| black_box(BabyStepGiantStepLookup::default()))
    });
}

pub fn precomputing_g2_generator_for_miller_loop(c: &mut Criterion) {
    let g2 = G2Affine::generator();
    c.bench_function("bsgs default table", |b| {
        b.iter(|| black_box(G2Prepared::from(g2)))
    });
}

criterion_group!(
    benches,
    precompute_default_bsgs_table,
    precomputing_g2_generator_for_miller_loop
);
criterion_main!(benches);

// TODO: benchmark using affine vs projective representation throughout the crate
// (when conversion / serialization / computation is involved)
