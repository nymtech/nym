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
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::RwLock;

#[derive(Clone)]
pub(super) struct VPNManager {
    inner: Arc<Inner>,
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

impl VPNManager {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn new<R>(mut rng: R, secret_reuse_limit: usize) -> Self
    where
        R: CryptoRng + Rng,
    {
        let initial_secret = EphemeralSecret::new_with_rng(&mut rng);
        VPNManager {
            inner: Arc::new(Inner {
                secret_reuse_limit,
                current_initial_secret: RwLock::new(initial_secret),
                packets_with_current_secret: AtomicUsize::new(0),
            }),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn new(secret_reuse_limit: usize) -> Self {}
}
