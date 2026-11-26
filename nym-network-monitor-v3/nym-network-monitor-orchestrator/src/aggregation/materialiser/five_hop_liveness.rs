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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        FIXTURE_EPOCH, FIXTURE_SEEN_AT, NewTestRun, failed_five_hop_mixnet_session,
        five_hop_mixnet_batch, five_hop_mixnet_session, gateway, minimal_measurement,
        minimal_test_run, mixnode,
    };
    use nym_network_monitor_orchestrator_requests::models::{
        InterfaceMeasurement, RunMeasurements,
    };
    use nym_network_monitor_route_records::scoring::FailSequenceRule;
    use nym_network_monitor_route_records::test_message::{SessionTestMessages, TestMessage};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// The epoch whose sessions are scored as [`FIXTURE_EPOCH`] begins.
    const MEASURED_EPOCH: EpochId = FIXTURE_EPOCH - 1;

    /// What a [`RecordingStrategy`] was given at one scoring.
    struct Given {
        test_messages: Vec<TestMessage>,
        one_hop_scores: HashMap<NodeId, f64>,
    }

    /// Remembers what it is given and scores no node.
    #[derive(Clone, Default)]
    struct RecordingStrategy {
        given: Arc<Mutex<Vec<Given>>>,
    }

    impl ScoringStrategy for RecordingStrategy {
        fn score(
            &self,
            test_messages: &[TestMessage],
            one_hop_scores: &HashMap<NodeId, f64>,
        ) -> BTreeMap<NodeId, f64> {
            self.given.lock().unwrap().push(Given {
                test_messages: test_messages.to_vec(),
                one_hop_scores: one_hop_scores.clone(),
            });
            BTreeMap::new()
        }
    }

    impl RecordingStrategy {
        /// What it was given at its only scoring.
        fn only_given(&self) -> Given {
            let mut given = self.given.lock().unwrap();
            assert_eq!(given.len(), 1, "expected exactly one scoring");
            given.remove(0)
        }
    }

    /// Storage with gateways 100 and 101 bonded, to carry route runs.
    async fn storage_with_gateways() -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .store_refresh(&[gateway(100), gateway(101)], FIXTURE_SEEN_AT)
            .await
            .unwrap();
        storage
    }

    /// Scores [`FIXTURE_EPOCH`] over `storage` with `strategy`.
    async fn materialise(
        storage: &NetworkMonitorStorage,
        strategy: Box<dyn ScoringStrategy + Send + Sync>,
    ) {
        FiveHopLivenessMaterialiser::new(strategy, storage.clone())
            .materialise(FIXTURE_EPOCH, FIXTURE_SEEN_AT)
            .await
            .unwrap();
    }

    /// Each node's value filed under [`FIXTURE_EPOCH`].
    async fn stored_values(storage: &NetworkMonitorStorage) -> Vec<(i64, f64)> {
        storage
            .get_mixnet_epoch_five_hop_liveness(FIXTURE_EPOCH)
            .await
            .unwrap()
            .into_iter()
            .map(|value| (value.node_id, value.score))
            .collect()
    }

    /// Stores a mixnode liveness run against `node_id` finished `ago` before now, scoring `received / 10`.
    async fn mixnode_liveness_scored(
        storage: &NetworkMonitorStorage,
        node_id: i64,
        received: usize,
        ago: Duration,
    ) {
        let run = NewTestRun {
            test_timestamp: OffsetDateTime::now_utc() - ago,
            ..minimal_test_run(node_id)
        };
        let measurements = RunMeasurements::MixnodeLiveness {
            mix_forwarding: InterfaceMeasurement {
                packets_sent: 10,
                packets_received: received,
                ..minimal_measurement()
            },
        };
        storage
            .storage_manager
            .insert_test_run(&run, &measurements)
            .await
            .unwrap();
    }

    // The value filed under an epoch is scored from the sessions of the epoch before and no others,
    // and of those only the ones the discard rule kept: the strategy is given exactly the test
    // messages of epoch 6's kept sessions, in send order. The verdicts are stored ones, so a
    // lossless batch can be discarded here.
    #[tokio::test]
    async fn only_the_kept_sessions_of_the_epoch_before_are_scored() {
        let storage = storage_with_gateways().await;
        let first_kept = five_hop_mixnet_batch(100, MEASURED_EPOCH, 1000..1010, 0);
        let second_kept = five_hop_mixnet_batch(100, MEASURED_EPOCH, 3000..3010, 0);
        storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&first_kept, true),
                five_hop_mixnet_session(
                    &five_hop_mixnet_batch(101, MEASURED_EPOCH, 2000..2010, 0),
                    false,
                ),
            ])
            .await;
        storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&second_kept, true),
                failed_five_hop_mixnet_session(101),
            ])
            .await;
        // kept sessions of the epochs on either side
        for mixnet_epoch in [MEASURED_EPOCH - 1, FIXTURE_EPOCH] {
            let batch = five_hop_mixnet_batch(100, mixnet_epoch, 4000..4010, 0);
            storage
                .store_submitted_five_hop_mixnet_run(&[five_hop_mixnet_session(&batch, true)])
                .await;
        }

        let strategy = RecordingStrategy::default();
        materialise(&storage, Box::new(strategy.clone())).await;

        let expected: Vec<TestMessage> = [first_kept, second_kept]
            .iter()
            .flat_map(|batch| SessionTestMessages::from_batch(batch).test_messages)
            .collect();
        assert_eq!(strategy.only_given().test_messages, expected);
    }

    // A mixnode's most recent one-hop score is that of its latest mixnode liveness run, and a
    // mixnode without one has none.
    #[tokio::test]
    async fn the_strategy_is_given_each_mixnodes_latest_one_hop_score() {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .store_refresh(&[mixnode(1), mixnode(2)], FIXTURE_SEEN_AT)
            .await
            .unwrap();
        mixnode_liveness_scored(&storage, 1, 5, Duration::from_secs(30 * 60)).await;
        mixnode_liveness_scored(&storage, 1, 10, Duration::from_secs(10 * 60)).await;

        let strategy = RecordingStrategy::default();
        materialise(&storage, Box::new(strategy.clone())).await;

        assert_eq!(
            strategy.only_given().one_hop_scores,
            HashMap::from([(1, 1.0)])
        );
    }

    // An epoch whose sessions were all discarded or failed has no five-hop liveness.
    #[tokio::test]
    async fn an_epoch_without_a_kept_session_stores_nothing() {
        let storage = storage_with_gateways().await;
        let batch = five_hop_mixnet_batch(100, MEASURED_EPOCH, 0..10, 0);
        storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&batch, false),
                failed_five_hop_mixnet_session(101),
            ])
            .await;

        materialise(&storage, Box::new(FailSequenceRule)).await;

        assert!(stored_values(&storage).await.is_empty());
    }

    // A session that returned nothing, discarded at submission, does not spoil its epoch: the other
    // session is scored, and the losses it would have charged to mixnodes 1, 2 and 3 are not.
    #[tokio::test]
    async fn a_discarded_session_among_several_leaves_the_rest_scored() {
        let storage = storage_with_gateways().await;
        let delivered = five_hop_mixnet_batch(100, MEASURED_EPOCH, 0..10, 0);
        let returned_nothing = five_hop_mixnet_batch(101, MEASURED_EPOCH, 10..20, 10);
        storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&delivered, true),
                five_hop_mixnet_session(&returned_nothing, false),
            ])
            .await;

        materialise(&storage, Box::new(FailSequenceRule)).await;

        let every_mixnode_at_one: Vec<(i64, f64)> = (1..=6).map(|node_id| (node_id, 1.0)).collect();
        assert_eq!(stored_values(&storage).await, every_mixnode_at_one);
    }
}
