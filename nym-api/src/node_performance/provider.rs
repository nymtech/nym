// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::support::caching::cache::SharedCache;
use crate::support::storage::NymApiStorage;
use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to retrieve performance score for node {node_id} for epoch {epoch_id}: {error}")]
pub(crate) struct PerformanceRetrievalFailure {
    node_id: NodeId,
    epoch_id: EpochId,
    error: String,
}

pub(crate) trait NodePerformanceProvider {
    async fn get_node_score(&self, node_id: NodeId, epoch: EpochId) -> RoutingScore;
}

// first impl for contract cache

// second impl for NM/storage

impl NodePerformanceProvider for SharedCache<PerformanceContractCacheData> {
    async fn get_node_score(&self, node_id: NodeId, epoch: EpochId) -> RoutingScore {
        todo!()
    }
}

// TODO: this will also need to be wrapped to contain... something... in order to map epoch id to timestamps
impl NodePerformanceProvider for NymApiStorage {
    async fn get_node_score(&self, node_id: NodeId, epoch: EpochId) -> RoutingScore {
        // self
        //     .get_average_node_reliability_in_the_last_24hrs(
        //         node_id,
        //         epoch.current_epoch_end_unix_timestamp(),
        //     )
        //     .await
        todo!()
    }
}
