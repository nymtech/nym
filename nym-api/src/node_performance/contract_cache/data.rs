// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::{BTreeMap, HashMap};

pub(crate) struct PerformanceContractEpochCacheData {
    pub(crate) epoch_id: EpochId,
    pub(crate) median_performance: HashMap<NodeId, Performance>,
}

pub(crate) struct PerformanceContractCacheData {
    pub(crate) epoch_performance: BTreeMap<EpochId, PerformanceContractEpochCacheData>,
}

impl PerformanceContractCacheData {
    pub(crate) fn update(
        &mut self,
        update: PerformanceContractEpochCacheData,
        values_to_retain: usize,
    ) {
        self.epoch_performance.insert(update.epoch_id, update);
        if self.epoch_performance.len() > values_to_retain {
            // remove the oldest entry, i.e. one with the lowest epoch id
            self.epoch_performance.pop_first();
        }
    }
}

// needed for cache initialisation
impl From<PerformanceContractEpochCacheData> for PerformanceContractCacheData {
    fn from(cache_data: PerformanceContractEpochCacheData) -> Self {
        let mut epoch_performance = BTreeMap::new();
        epoch_performance.insert(cache_data.epoch_id, cache_data);
        PerformanceContractCacheData { epoch_performance }
    }
}
