// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::entry::EntryStats;
use crate::mixnet::MixingStats;
use crate::network::NetworkStats;
use std::ops::Deref;
use std::sync::Arc;

pub mod entry;
pub mod events;
pub mod mixnet;
pub mod network;

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

    pub network: NetworkStats,
}
