// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::provider::{NodesStressTestingScores, PerformanceRetrievalFailure};
use crate::support::caching::cache::UninitialisedCache;
use crate::support::storage::NymApiStorage;
use nym_api_requests::models::{RoutingScore, StressTestingScore};
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) struct LegacyStoragePerformanceProvider {
    /// Specifies the duration of the rolling average used for stress testing score.
    stress_testing_data_period: Duration,

    storage: NymApiStorage,
    mixnet_contract_cache: MixnetContractCache,
}

impl LegacyStoragePerformanceProvider {
    pub(crate) fn new(
        storage: NymApiStorage,
        mixnet_contract_cache: MixnetContractCache,
        stress_testing_data_period: Duration,
    ) -> Self {
        LegacyStoragePerformanceProvider {
            stress_testing_data_period,
            storage,
            mixnet_contract_cache,
        }
    }

    async fn map_epoch_id_to_end_timestamp(
        &self,
        epoch_id: EpochId,
    ) -> Result<OffsetDateTime, UninitialisedCache> {
        let interval_details = self.mixnet_contract_cache.current_interval().await?;
        let duration = interval_details.epoch_length();
        let current_end = interval_details.current_epoch_end();
        let current_id = interval_details.current_epoch_absolute_id();

        if current_id == epoch_id {
            return Ok(current_end);
        }

        if current_id < epoch_id {
            let diff = epoch_id - current_id;
            let end = current_end + diff * duration;
            return Ok(end);
        }

        // epoch_id > current_id
        let diff = current_id - epoch_id;
        let end = current_end - diff * duration;
        Ok(end)
    }

    pub(crate) async fn epoch_id_timestamp(
        &self,
        epoch_id: EpochId,
    ) -> Result<OffsetDateTime, PerformanceRetrievalFailure> {
        self.map_epoch_id_to_end_timestamp(epoch_id)
            .await
            .map_err(|_| {
                PerformanceRetrievalFailure::new(
                    0,
                    epoch_id,
                    "mixnet contract cache has not been initialised yet",
                )
            })
    }

    pub(crate) async fn node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        let end_ts = self.epoch_id_timestamp(epoch_id).await?.unix_timestamp();
        self.get_node_routing_score_with_unix_timestamp(node_id, epoch_id, end_ts)
            .await
    }

    pub(crate) async fn get_node_routing_score_with_unix_timestamp(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
        end_ts: i64,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
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

    pub(crate) async fn node_stress_testing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure> {
        let end_ts = self.epoch_id_timestamp(epoch_id).await?;
        let start_ts = end_ts - self.stress_testing_data_period;

        self.node_stress_testing_score_in_range(node_id, epoch_id, start_ts, end_ts)
            .await
    }

    pub(crate) async fn node_stress_testing_score_in_range(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
        start_ts: OffsetDateTime,
        end_ts: OffsetDateTime,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure> {
        let result = self
            .storage
            .get_average_node_stress_test_score(node_id, start_ts, end_ts)
            .await
            .map_err(|err| PerformanceRetrievalFailure::new(node_id, epoch_id, err.to_string()))?;

        match result {
            None => Ok(StressTestingScore::unreachable()),
            Some(result) => Ok(result.into()),
        }
    }

    pub(crate) async fn get_node_stress_testing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure> {
        let mut scores = HashMap::new();

        let end_ts = self.epoch_id_timestamp(epoch_id).await?;
        let start_ts = end_ts - self.stress_testing_data_period;

        for &node_id in node_ids {
            scores.insert(
                node_id,
                self.node_stress_testing_score_in_range(node_id, epoch_id, start_ts, end_ts)
                    .await,
            );
        }

        Ok(NodesStressTestingScores { inner: scores })
    }
}
