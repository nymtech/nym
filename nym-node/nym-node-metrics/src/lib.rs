// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use crate::entry::EntryStats;
use crate::mixnet::MixingStats;
use crate::network::NetworkStats;
use crate::process::NodeStats;
use crate::wireguard::WireguardStats;
use std::ops::Deref;
use std::sync::Arc;

pub mod entry;
pub mod events;
pub mod mixnet;
pub mod network;
pub mod process;
pub mod prometheus_wrapper;
pub mod wireguard;

#[derive(Clone, Default)]
pub struct NymNodeMetrics {
    inner: Arc<NymNodeMetricsInner>,
}

impl NymNodeMetrics {
    pub fn new() -> Self {
        NymNodeMetrics::default()
    }
}

impl Deref for NymNodeMetrics {
    type Target = NymNodeMetricsInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Default)]
pub struct NymNodeMetricsInner {
    pub mixnet: MixingStats,
    pub entry: EntryStats,
    pub wireguard: WireguardStats,

    pub network: NetworkStats,
    pub process: NodeStats,
}
