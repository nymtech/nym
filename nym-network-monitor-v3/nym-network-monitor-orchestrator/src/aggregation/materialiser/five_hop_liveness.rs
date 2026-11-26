// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::five_hop_liveness::FiveHopMixnetSessionTestMessages;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::MixnetEpochFiveHopLiveness;
use nym_network_monitor_route_records::NodeId;
use nym_network_monitor_route_records::scoring::ScoringStrategy;
use nym_network_monitor_route_records::test_message::merge_in_send_order;
use nym_validator_client::nyxd::nym_mixnet_contract_common::{EpochId, Interval};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::info;

/// Scores five-hop liveness as each epoch begins, from the route sessions of the epoch before.
pub(crate) struct FiveHopLivenessMaterialiser {
    strategy: Box<dyn ScoringStrategy + Send + Sync>,

    storage: NetworkMonitorStorage,
}

impl FiveHopLivenessMaterialiser {
    pub(crate) fn new(
        strategy: Box<dyn ScoringStrategy + Send + Sync>,
        storage: NetworkMonitorStorage,
    ) -> Self {
        FiveHopLivenessMaterialiser { strategy, storage }
    }

    /// Scores every epoch the aggregates have yet to materialise, through to the one in progress.
    ///
    /// Must run before the aggregates step: its insert marks an epoch materialised, after which the
    /// epoch is no longer pending here.
    pub(crate) async fn materialise_pending(&self, interval: &Interval) -> anyhow::Result<()> {
        let current = interval.current_epoch_absolute_id();
        let last_materialised = self.storage.get_last_materialised_mixnet_epoch().await?;

        // the same range the aggregates step is about to materialise
        let first = last_materialised.map_or(current, |last| last + 1);
        for mixnet_epoch in first..=current {
            self.materialise(mixnet_epoch, interval.epoch_start(mixnet_epoch))
                .await?;
        }
        Ok(())
    }

    /// Scores the route sessions of the epoch before `mixnet_epoch` and stores each mixnode's value
    /// under `mixnet_epoch`.
    async fn materialise(
        &self,
        mixnet_epoch: EpochId,
        epoch_start: OffsetDateTime,
    ) -> anyhow::Result<()> {
        // the first epoch has none before it
        let Some(measured_epoch) = mixnet_epoch.checked_sub(1) else {
            return Ok(());
        };

        let sessions = self
            .storage
            .get_five_hop_mixnet_sessions_of_epoch(measured_epoch)
            .await?;

        let mut kept = Vec::new();
        let mut discarded = 0;
        for session in &sessions {
            match FiveHopMixnetSessionTestMessages::read(session) {
                FiveHopMixnetSessionTestMessages::Kept(test_messages) => kept.push(test_messages),
                FiveHopMixnetSessionTestMessages::Discarded(_)
                | FiveHopMixnetSessionTestMessages::Error(_) => discarded += 1,
            }
        }
        let kept_sessions = kept.len();

        let test_messages = merge_in_send_order(kept);
        let one_hop_scores = self.one_hop_scores().await?;
        let values: Vec<MixnetEpochFiveHopLiveness> = self
            .strategy
            .score(&test_messages, &one_hop_scores)
            .into_iter()
            .map(|(node_id, score)| MixnetEpochFiveHopLiveness {
                mixnet_epoch: mixnet_epoch as i64,
                epoch_start,
                node_id: i64::from(node_id),
                score,
            })
            .collect();
        self.storage
            .batch_insert_mixnet_epoch_five_hop_liveness(&values)
            .await?;

        PROMETHEUS_METRICS.set(
            PrometheusMetric::FiveHopLivenessKeptSessions,
            kept_sessions as i64,
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::FiveHopLivenessDiscardedSessions,
            discarded as i64,
        );
        info!(
            mixnet_epoch,
            measured_epoch,
            kept = kept_sessions,
            discarded,
            scored = values.len(),
            "materialised five-hop liveness"
        );
        Ok(())
    }

    /// Each mixnode's most recent one-hop liveness score, from its latest mixnode liveness run.
    async fn one_hop_scores(&self) -> anyhow::Result<HashMap<NodeId, f64>> {
        let runs = self.storage.get_latest_mixnode_liveness_testruns().await?;
        Ok(runs
            .into_iter()
            // stored from a `NodeId`, so it fits
            .map(|run| (run.run.node_id as NodeId, run.score()))
            .collect())
    }
}
