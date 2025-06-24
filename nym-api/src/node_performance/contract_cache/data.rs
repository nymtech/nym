// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::provider::PerformanceRetrievalFailure;
use nym_api_requests::models::RoutingScore;
use nym_contracts_common::NaiveFloat;
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

    pub(crate) fn node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        // TODO: somehow send a signal to refresh this epoch
        let epoch_scores = self.epoch_performance.get(&epoch_id).ok_or_else(|| {
            PerformanceRetrievalFailure::new(
                node_id,
                epoch_id,
                format!("no cached performance results for epoch {epoch_id}"),
            )
        })?;

        let node_score = epoch_scores
            .median_performance
            .get(&node_id)
            .ok_or_else(|| {
                PerformanceRetrievalFailure::new(
                    node_id,
                    epoch_id,
                    format!(
                        "no cached performance results for node {node_id} for epoch {epoch_id}"
                    ),
                )
            })?;

        Ok(RoutingScore::new(node_score.naive_to_f64()))
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
