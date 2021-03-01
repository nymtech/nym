// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use nymsphinx_types::EphemeralSecret;
use rand::{CryptoRng, Rng};
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{RwLock, RwLockReadGuard};

#[cfg(not(target_arch = "wasm32"))]
pub(super) type SpinhxKeyRef<'a> = RwLockReadGuard<'a, EphemeralSecret>;

#[cfg(target_arch = "wasm32")]
pub(super) type SpinhxKeyRef<'a> = &'a EphemeralSecret;

#[cfg_attr(not(target_arch = "wasm32"), derive(Clone))]
pub(super) struct VpnManager {
    #[cfg(not(target_arch = "wasm32"))]
    inner: Arc<Inner>,

    #[cfg(target_arch = "wasm32")]
    inner: Inner,
}

struct Inner {
    /// Maximum number of times particular sphinx-secret can be re-used before being rotated.
    secret_reuse_limit: usize,

    /// Currently used initial sphinx-secret for the packets sent.
    #[cfg(not(target_arch = "wasm32"))]
    current_initial_secret: RwLock<EphemeralSecret>,

    #[cfg(target_arch = "wasm32")]
    // this is a temporary work-around for wasm (which currently does not have retransmission
    // and hence will not require multi-thread access) and also we can't import tokio's RWLock
    // in wasm.
    current_initial_secret: EphemeralSecret,

    /// If the client is running as VPN it's expected to keep re-using the same initial secret
    /// for a while so that the mixnodes could cache some secret derivation results. However,
    /// we should reset it every once in a while.
    packets_with_current_secret: AtomicUsize,
}

impl VpnManager {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn new<R>(mut rng: R, secret_reuse_limit: usize) -> Self
    where
        R: CryptoRng + Rng,
    {
        let initial_secret = EphemeralSecret::new_with_rng(&mut rng);
        VpnManager {
            inner: Arc::new(Inner {
                secret_reuse_limit,
                current_initial_secret: RwLock::new(initial_secret),
                packets_with_current_secret: AtomicUsize::new(0),
            }),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn new<R>(mut rng: R, secret_reuse_limit: usize) -> Self
    where
        R: CryptoRng + Rng,
    {
        let initial_secret = EphemeralSecret::new_with_rng(&mut rng);
        VpnManager {
            inner: Inner {
                secret_reuse_limit,
                current_initial_secret: initial_secret,
                packets_with_current_secret: AtomicUsize::new(0),
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn rotate_secret<R>(&mut self, mut rng: R)
    where
        R: CryptoRng + Rng,
    {
        let new_secret = EphemeralSecret::new_with_rng(&mut rng);
        let mut write_guard = self.inner.current_initial_secret.write().await;

        *write_guard = new_secret;
        // in here we have an exclusive lock so we don't have to have restrictive ordering as no
        // other thread will be able to get here
        self.inner
            .packets_with_current_secret
            .store(0, Ordering::Relaxed)
    }

    // this method is async for consistency with non-wasm version
    #[cfg(target_arch = "wasm32")]
    pub(super) async fn rotate_secret<R>(&mut self, mut rng: R)
    where
        R: CryptoRng + Rng,
    {
        let new_secret = EphemeralSecret::new_with_rng(&mut rng);
        self.inner.current_initial_secret = new_secret;

        // wasm is single-threaded so relaxed ordering is also fine here
        self.inner
            .packets_with_current_secret
            .store(0, Ordering::Relaxed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn current_secret(&self) -> SpinhxKeyRef<'_> {
        self.inner.current_initial_secret.read().await
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn current_secret(&self) -> SpinhxKeyRef<'_> {
        &self.inner.current_initial_secret
    }

    fn increment_key_usage(&mut self) {
        // TODO: is this the appropriate ordering?
        self.inner
            .packets_with_current_secret
            .fetch_add(1, Ordering::SeqCst);
    }

    fn current_key_usage(&self) -> usize {
        // TODO: is this the appropriate ordering?
        self.inner
            .packets_with_current_secret
            .load(Ordering::SeqCst)
    }

    pub(super) async fn use_secret<R>(&mut self, rng: R) -> SpinhxKeyRef<'_>
    where
        R: CryptoRng + Rng,
    {
        if self.current_key_usage() > self.inner.secret_reuse_limit {
            self.rotate_secret(rng).await;
        }
        self.increment_key_usage();
        self.current_secret().await
    }
}
