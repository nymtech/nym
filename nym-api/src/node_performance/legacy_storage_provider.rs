// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::provider::PerformanceRetrievalFailure;
use crate::support::caching::cache::UninitialisedCache;
use crate::support::storage::NymApiStorage;
use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};
use tracing::{trace, warn};

pub(crate) struct LegacyStoragePerformanceProvider {
    storage: NymApiStorage,
    mixnet_contract_cache: MixnetContractCache,
}

impl LegacyStoragePerformanceProvider {
    async fn map_epoch_id_to_end_unix_timestamp(
        &self,
        epoch_id: EpochId,
    ) -> Result<i64, UninitialisedCache> {
        let interval_details = self.mixnet_contract_cache.current_interval().await?;
        let duration = interval_details.epoch_length();
        let current_end = interval_details.current_epoch_end();
        let current_id = interval_details.current_epoch_absolute_id();

        if current_id == epoch_id {
            return Ok(current_end.unix_timestamp());
        }

        if current_id < epoch_id {
            let diff = epoch_id - current_id;
            let end = current_end + diff * duration;
            return Ok(end.unix_timestamp());
        }

        // epoch_id > current_id
        let diff = current_id - epoch_id;
        let end = current_end - diff * duration;
        Ok(end.unix_timestamp())
    }

    pub(crate) async fn node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        let end_ts = self
            .map_epoch_id_to_end_unix_timestamp(epoch_id)
            .await
            .map_err(|_| {
                PerformanceRetrievalFailure::new(
                    node_id,
                    epoch_id,
                    "mixnet contract cache has not been initialised yet",
                )
            })?;
        let reliability = self
            .storage
            .get_average_node_reliability_in_the_last_24hrs(node_id, end_ts)
            .await
            .map_err(|err| PerformanceRetrievalFailure::new(node_id, epoch_id, err.to_string()))?;

        // reliability: 0-100
        // score: 0-1
        let score = reliability / 100.;
        Ok(RoutingScore::new(score as f64))
    }
}
