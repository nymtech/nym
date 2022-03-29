use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dkg::bte::BabyStepGiantStepLookup;

pub fn precompute_default_bsgs_table(c: &mut Criterion) {
    c.bench_function("bsgs default table", |b| {
        b.iter(|| black_box(BabyStepGiantStepLookup::default()))
    });
}

criterion_group!(benches, precompute_default_bsgs_table);
criterion_main!(benches);

// TODO: benchmark using affine vs projective representation throughout the crate
// (when conversion / serialization / computation is involved)
