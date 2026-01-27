// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::traits::Timeboxed;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use std::future::Future;
use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;

use nym_bin_common::logging::tracing_subscriber::EnvFilter;
use nym_bin_common::logging::tracing_subscriber::layer::SubscriberExt;
use nym_bin_common::logging::tracing_subscriber::util::SubscriberInitExt;
use nym_bin_common::logging::{default_tracing_fmt_layer, tracing_subscriber};
pub use rand_chacha::rand_core::{CryptoRng, RngCore};

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

// test logger to use during debugging
#[allow(clippy::unwrap_used)]
pub fn setup_test_logger() {
    tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(
            EnvFilter::new("trace"),
            // .add_directive("nym_sdk::client_pool=info".parse().unwrap())
            // .add_directive("nym_sdk::tcp_proxy_client=debug".parse().unwrap()),
        )
        .init();
}
