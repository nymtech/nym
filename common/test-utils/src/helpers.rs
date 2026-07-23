// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]

use crate::traits::Timeboxed;
use nym_bin_common::logging::tracing_subscriber::EnvFilter;
use nym_bin_common::logging::tracing_subscriber::layer::SubscriberExt;
use nym_bin_common::logging::tracing_subscriber::util::SubscriberInitExt;
use nym_bin_common::logging::{default_tracing_fmt_layer, tracing_subscriber};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha010::rand_core::{SeedableRng as SeedableRng010, TryCryptoRng, TryRng};
use std::convert::Infallible;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;

// 'current' rand crate
pub use rand_chacha::ChaCha20Rng as DeterministicRng;
pub use rand_chacha::rand_core::{CryptoRng, RngCore};

// rand010 compat
pub use rand_chacha010::ChaChaRng as DeterministicRng010;
pub use rand_chacha010::rand_core::{CryptoRng as CryptoRng010, Rng as Rng010};

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

pub struct DeterministicRng010Send(Arc<Mutex<DeterministicRng010>>);

impl DeterministicRng010Send {
    pub fn new(deterministic_rng010: DeterministicRng010) -> Self {
        Self(Arc::new(Mutex::new(deterministic_rng010)))
    }
}

impl TryCryptoRng for DeterministicRng010Send {}

// unwraps are perfectly fine in test code
impl TryRng for DeterministicRng010Send {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.lock().unwrap().next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.lock().unwrap().next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.lock().unwrap().fill_bytes(dst);
        Ok(())
    }
}

pub fn deterministic_rng_09() -> DeterministicRng010 {
    seeded_rng_09([42u8; 32])
}

pub fn deterministic_rng() -> DeterministicRng {
    seeded_rng([42u8; 32])
}

pub fn seeded_rng(seed: [u8; 32]) -> DeterministicRng {
    DeterministicRng::from_seed(seed)
}

pub fn seeded_rng_09(seed: [u8; 32]) -> DeterministicRng010 {
    DeterministicRng010::from_seed(seed)
}

pub fn u64_seeded_rng(seed: u64) -> DeterministicRng {
    DeterministicRng::seed_from_u64(seed)
}

pub fn u64_seeded_rng_09(seed: u64) -> DeterministicRng010 {
    DeterministicRng010::seed_from_u64(seed)
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
