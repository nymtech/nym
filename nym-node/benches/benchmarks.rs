// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use bloomfilter::Bloom;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use nym_sphinx_types::REPLAY_TAG_SIZE;
use rand::{thread_rng, Rng};
use std::sync::Mutex;

pub fn uncontested_bloomfilter_check(c: &mut Criterion) {
    let mut bloomfilter = Bloom::new_for_fp_rate(725760000, 1e-5).unwrap();
    c.bench_function("bf_725760000_1e-5_check", |b| {
        b.iter_batched(
            || {
                let mut rng = thread_rng();
                let mut reply_tag = [0; REPLAY_TAG_SIZE];
                rng.fill(&mut reply_tag);
                reply_tag
            },
            |replay_tag| {
                black_box(bloomfilter.check_and_set(&replay_tag));
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn uncontested_bloomfilter_check_with_exclusive_mutex(c: &mut Criterion) {
    let bloomfilter = Mutex::new(Bloom::new_for_fp_rate(725760000, 1e-5).unwrap());
    c.bench_function("bf_725760000_1e-5_uncontested_std_mutex_check", |b| {
        b.iter_batched(
            || {
                let mut rng = thread_rng();
                let mut reply_tag = [0; REPLAY_TAG_SIZE];
                rng.fill(&mut reply_tag);
                reply_tag
            },
            |replay_tag| {
                black_box(bloomfilter.lock().unwrap().check_and_set(&replay_tag));
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn uncontested_bloomfilter_check_with_exclusive_tokio_mutex(c: &mut Criterion) {
    let bloomfilter = tokio::sync::Mutex::new(Bloom::new_for_fp_rate(725760000, 1e-5).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("bf_725760000_1e-5_uncontested_tokio_mutex_check", |b| {
        b.to_async(&runtime).iter_batched(
            || {
                let mut rng = thread_rng();
                let mut reply_tag = [0; REPLAY_TAG_SIZE];
                rng.fill(&mut reply_tag);
                reply_tag
            },
            async |replay_tag| {
                black_box(bloomfilter.lock().await.check_and_set(&replay_tag));
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    nym_node_benches,
    uncontested_bloomfilter_check,
    uncontested_bloomfilter_check_with_exclusive_mutex,
    uncontested_bloomfilter_check_with_exclusive_tokio_mutex
);

// TODO: somehow bench heavily contested cases...

criterion_main!(nym_node_benches);
