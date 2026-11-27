// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{FiveHopMixnetSession, MixnetEpochFiveHopLiveness};
use nym_network_monitor_route_records::NodeId;
use nym_network_monitor_route_records::scoring::ScoringStrategy;
use nym_network_monitor_route_records::test_message::{SessionTestMessages, merge_in_send_order};
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use std::collections::{BTreeMap, HashMap};
use time::OffsetDateTime;
use tracing::info;

/// How five-hop liveness is carried forward into the epochs that follow.
#[derive(Debug, Copy, Clone)]
pub(crate) struct FiveHopLivenessMaterialiserConfig {
    /// Rate per epoch at which a carried value fades towards full liveness.
    pub(crate) decay_per_epoch: f64,
}

/// Scores five-hop liveness as each epoch begins, from the five-hop mixnet sessions of the epoch
/// before, and carries the values of the mixnodes it did not score forward into it.
pub(crate) struct FiveHopLivenessMaterialiser {
    config: FiveHopLivenessMaterialiserConfig,

    strategy: Box<dyn ScoringStrategy + Send + Sync>,

    storage: NetworkMonitorStorage,
}

impl FiveHopLivenessMaterialiser {
    pub(crate) fn new(
        config: FiveHopLivenessMaterialiserConfig,
        strategy: Box<dyn ScoringStrategy + Send + Sync>,
        storage: NetworkMonitorStorage,
    ) -> Self {
        FiveHopLivenessMaterialiser {
            config,
            strategy,
            storage,
        }
    }

    /// Scores the five-hop mixnet sessions of the epoch before `mixnet_epoch` and stores each
    /// mixnode's value under `mixnet_epoch`, beside the values carried forward for the mixnodes it
    /// did not score.
    pub(super) async fn materialise(
        &self,
        mixnet_epoch: EpochId,
        epoch_start: OffsetDateTime,
    ) -> anyhow::Result<()> {
        // the first epoch has none before it
        let Some(measured_epoch) = mixnet_epoch.checked_sub(1) else {
            return Ok(());
        };

        let measured = self.measured_scores(measured_epoch).await?;
        let carried = self
            .carried_values(measured_epoch, epoch_start, &measured)
            .await?;
        info!(
            mixnet_epoch,
            measured_epoch,
            scored = measured.len(),
            carried = carried.len(),
            "materialised five-hop liveness"
        );

        let mut values: Vec<MixnetEpochFiveHopLiveness> = measured
            .into_iter()
            .map(|(node_id, score)| MixnetEpochFiveHopLiveness {
                mixnet_epoch: mixnet_epoch as i64,
                epoch_start,
                node_id: i64::from(node_id),
                score,
                measured: true,
            })
            .collect();
        values.extend(carried);
        self.storage
            .batch_insert_mixnet_epoch_five_hop_liveness(&values)
            .await?;
        Ok(())
    }

    /// Scores the kept sessions naming `measured_epoch`, and reports how many were kept and discarded.
    async fn measured_scores(
        &self,
        measured_epoch: EpochId,
    ) -> anyhow::Result<BTreeMap<NodeId, f64>> {
        let sessions = self
            .storage
            .get_five_hop_mixnet_sessions_of_epoch(measured_epoch)
            .await?;

        let kept: Vec<SessionTestMessages> = sessions
            .iter()
            .filter(|session| session.kept)
            .filter_map(FiveHopMixnetSession::test_messages)
            .collect();
        let discarded = sessions.len() - kept.len();

        PROMETHEUS_METRICS.set(
            PrometheusMetric::FiveHopLivenessKeptSessions,
            kept.len() as i64,
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::FiveHopLivenessDiscardedSessions,
            discarded as i64,
        );
        info!(
            measured_epoch,
            kept = kept.len(),
            discarded,
            "scoring the epoch's five-hop mixnet sessions"
        );

        let one_hop_scores = self.one_hop_scores().await?;
        Ok(self
            .strategy
            .score(&merge_in_send_order(kept), &one_hop_scores))
    }

    /// The values filed under `previous_epoch`, measured or carried, carried forward into the epoch
    /// starting at `epoch_start`, but for the mixnodes in `measured`.
    async fn carried_values(
        &self,
        previous_epoch: EpochId,
        epoch_start: OffsetDateTime,
        measured: &BTreeMap<NodeId, f64>,
    ) -> anyhow::Result<Vec<MixnetEpochFiveHopLiveness>> {
        let previous = self
            .storage
            .get_mixnet_epoch_five_hop_liveness(previous_epoch)
            .await?;

        Ok(previous
            .iter()
            // stored from a `NodeId`, so it fits
            .filter(|value| !measured.contains_key(&(value.node_id as NodeId)))
            .filter_map(|value| value.carried_forward(epoch_start, self.config.decay_per_epoch))
            .collect())
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
        FIXTURE_EPOCH, FIXTURE_SEEN_AT, NewTestRun, five_hop_mixnet_batch, minimal_measurement,
        minimal_test_run, mixnode,
    };
    use nym_network_monitor_orchestrator_requests::models::{
        InterfaceMeasurement, RunMeasurements,
    };
    use nym_network_monitor_route_records::scoring::FailSequenceRule;
    use nym_network_monitor_route_records::test_message::TestMessage;
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

    /// Gives the same scores whatever it is given.
    struct FixedStrategy(BTreeMap<NodeId, f64>);

    impl ScoringStrategy for FixedStrategy {
        fn score(&self, _: &[TestMessage], _: &HashMap<NodeId, f64>) -> BTreeMap<NodeId, f64> {
            self.0.clone()
        }
    }

    /// The shipped default.
    const CONFIG: FiveHopLivenessMaterialiserConfig = FiveHopLivenessMaterialiserConfig {
        decay_per_epoch: 0.1,
    };

    /// Scores [`FIXTURE_EPOCH`] over `storage` with `strategy`.
    async fn materialise(
        storage: &NetworkMonitorStorage,
        strategy: Box<dyn ScoringStrategy + Send + Sync>,
    ) {
        FiveHopLivenessMaterialiser::new(CONFIG, strategy, storage.clone())
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
        let storage = NetworkMonitorStorage::in_memory().await;
        let first_kept = five_hop_mixnet_batch(100, MEASURED_EPOCH, 1000..1010, 0);
        let second_kept = five_hop_mixnet_batch(100, MEASURED_EPOCH, 3000..3010, 0);
        storage
            .store_submitted_five_hop_mixnet_run(
                &[&first_kept],
                &[&five_hop_mixnet_batch(101, MEASURED_EPOCH, 2000..2010, 0)],
            )
            .await;
        storage
            .store_submitted_five_hop_mixnet_run(&[&second_kept], &[])
            .await;
        // kept sessions of the epochs on either side
        for mixnet_epoch in [MEASURED_EPOCH - 1, FIXTURE_EPOCH] {
            let batch = five_hop_mixnet_batch(100, mixnet_epoch, 4000..4010, 0);
            storage
                .store_submitted_five_hop_mixnet_run(&[&batch], &[])
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

    // An epoch whose sessions were all discarded has no five-hop liveness.
    #[tokio::test]
    async fn an_epoch_without_a_kept_session_stores_nothing() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let batch = five_hop_mixnet_batch(100, MEASURED_EPOCH, 0..10, 0);
        storage
            .store_submitted_five_hop_mixnet_run(&[], &[&batch])
            .await;

        materialise(&storage, Box::new(FailSequenceRule)).await;

        assert!(stored_values(&storage).await.is_empty());
    }

    // A session that returned nothing, discarded at submission, does not spoil its epoch: the other
    // session is scored, and the losses it would have charged to mixnodes 1, 2 and 3 are not.
    #[tokio::test]
    async fn a_discarded_session_among_several_leaves_the_rest_scored() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let delivered = five_hop_mixnet_batch(100, MEASURED_EPOCH, 0..10, 0);
        let returned_nothing = five_hop_mixnet_batch(101, MEASURED_EPOCH, 10..20, 10);
        storage
            .store_submitted_five_hop_mixnet_run(&[&delivered], &[&returned_nothing])
            .await;

        materialise(&storage, Box::new(FailSequenceRule)).await;

        let every_mixnode_at_one: Vec<(i64, f64)> = (1..=6).map(|node_id| (node_id, 1.0)).collect();
        assert_eq!(stored_values(&storage).await, every_mixnode_at_one);
    }

    // A mixnode the epoch's scoring left out has the value filed under the epoch before, faded by
    // one epoch, whether that value was measured or itself carried: 0.6 becomes about 0.638. Node
    // 3's fades to just past 0.995, so it is no longer filed, and node 4, which the scoring gave a
    // value, has that value alone.
    #[tokio::test]
    async fn an_unscored_mixnode_carries_the_value_filed_under_the_epoch_before() {
        let filed_before = |node_id: i64, score: f64, measured: bool| MixnetEpochFiveHopLiveness {
            mixnet_epoch: i64::from(MEASURED_EPOCH),
            epoch_start: FIXTURE_SEEN_AT,
            node_id,
            score,
            measured,
        };
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .batch_insert_mixnet_epoch_five_hop_liveness(&[
                filed_before(1, 0.6, true),
                filed_before(2, 0.6, false),
                filed_before(3, 0.9945, true),
                filed_before(4, 0.5, true),
            ])
            .await
            .unwrap();

        let strategy = FixedStrategy(BTreeMap::from([(4, 0.9)]));
        materialise(&storage, Box::new(strategy)).await;

        let stored: Vec<(i64, f64, bool)> = storage
            .get_mixnet_epoch_five_hop_liveness(FIXTURE_EPOCH)
            .await
            .unwrap()
            .into_iter()
            .map(|value| {
                (
                    value.node_id,
                    (value.score * 1000.0).round() / 1000.0,
                    value.measured,
                )
            })
            .collect();
        assert_eq!(
            stored,
            vec![(1, 0.638, false), (2, 0.638, false), (4, 0.9, true)]
        );
    }

    // The steps compound into the specification's figure: 0.6 measured 7 epochs before is about
    // 0.80 once every epoch since has been materialised in turn.
    #[tokio::test]
    async fn a_value_carried_epoch_after_epoch_fades_as_the_specification_gives() {
        const EPOCH: EpochId = 100;
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .batch_insert_mixnet_epoch_five_hop_liveness(&[MixnetEpochFiveHopLiveness {
                mixnet_epoch: i64::from(EPOCH - 7),
                epoch_start: FIXTURE_SEEN_AT,
                node_id: 1,
                score: 0.6,
                measured: true,
            }])
            .await
            .unwrap();

        let scores_nothing = FixedStrategy(BTreeMap::new());
        let materialiser =
            FiveHopLivenessMaterialiser::new(CONFIG, Box::new(scores_nothing), storage.clone());
        for mixnet_epoch in EPOCH - 6..=EPOCH {
            materialiser
                .materialise(mixnet_epoch, FIXTURE_SEEN_AT)
                .await
                .unwrap();
        }

        let stored: Vec<(i64, f64, bool)> = storage
            .get_mixnet_epoch_five_hop_liveness(EPOCH)
            .await
            .unwrap()
            .into_iter()
            // to the two decimals the specification gives
            .map(|value| {
                (
                    value.node_id,
                    (value.score * 100.0).round() / 100.0,
                    value.measured,
                )
            })
            .collect();
        assert_eq!(stored, vec![(1, 0.80, false)]);
    }
}
