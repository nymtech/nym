// Copyright 2023-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::load::CachedNodeLoad;
use crate::node::http::state::metrics::MetricsAppState;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use nym_crypto::asymmetric::ed25519;
use nym_node_metrics::NymNodeMetrics;
use nym_noise_keys::VersionedNoiseKey;
use nym_verloc::measurements::SharedVerlocStats;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

pub mod load;
pub mod metrics;

pub(crate) struct StaticNodeInformation {
    pub(crate) ed25519_identity_keys: Arc<ed25519::KeyPair>,
    pub(crate) x25519_versioned_noise_key: Option<VersionedNoiseKey>,
    pub(crate) ip_addresses: Vec<IpAddr>,
    pub(crate) hostname: Option<String>,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) startup_time: Instant,

    pub(crate) static_information: Arc<StaticNodeInformation>,

    pub(crate) x25519_sphinx_keys: ActiveSphinxKeys,

    pub(crate) cached_load: CachedNodeLoad,

    pub(crate) metrics: MetricsAppState,
}

impl AppState {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new(
        static_information: StaticNodeInformation,
        x25519_sphinx_keys: ActiveSphinxKeys,
        metrics: NymNodeMetrics,
        verloc: SharedVerlocStats,
        load_cache_ttl: Duration,
    ) -> Self {
        AppState {
            static_information: Arc::new(static_information),
            x25519_sphinx_keys,

            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
            cached_load: CachedNodeLoad::new(load_cache_ttl),
            metrics: MetricsAppState { metrics, verloc },
        }
    }
}
