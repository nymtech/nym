// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::traits::Timeboxed;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::future::Future;
use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;

pub fn leak<T>(val: T) -> &'static mut T {
    Box::leak(Box::new(val))
}

pub fn spawn_timeboxed<F>(fut: F) -> JoinHandle<Result<F::Output, Elapsed>>
where
    F: Future + Send + 'static,
    <F as Future>::Output: Send,
{
    tokio::spawn(async move { fut.timeboxed().await })
}

pub fn deterministic_rng() -> ChaCha20Rng {
    seeded_rng([42u8; 32])
}

pub fn seeded_rng(seed: [u8; 32]) -> ChaCha20Rng {
    ChaCha20Rng::from_seed(seed)
}

pub fn u64_seeded_rng(seed: u64) -> ChaCha20Rng {
    ChaCha20Rng::seed_from_u64(seed)
}
