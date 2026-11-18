// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Addr;
use nym_performance_contract_common::{EpochId, EpochWeights, KindMedians, LastSubmission, NodeId};
use nym_validator_client::nyxd::contract_traits::PinnedPerformanceQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;
use time::OffsetDateTime;

/// What the annotations need beside a performance snapshot that the digest does not commit,
/// read at the snapshot's height and unproven.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PerformanceAuxiliary {
    /// The weights in force at the snapshot height.
    weights: EpochWeights,

    /// The block time of the snapshot height.
    block_time: OffsetDateTime,

    /// The per-node medians of the newest entry epoch and the one before it.
    medians: BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>,

    /// The last submission as of the snapshot height, `None` before the first one.
    last_submission: Option<SubmissionKey>,
}

/// The fields identifying a submission, held in place of `LastSubmission`, whose measurements
/// skip absent values when serialised and so cannot round-trip through bincode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SubmissionKey {
    block_height: u64,
    epoch_id: EpochId,
    sender: Addr,
    last_node_id: NodeId,
}

impl SubmissionKey {
    fn new(submission: &LastSubmission) -> Option<Self> {
        let data = submission.data.as_ref()?;
        Some(SubmissionKey {
            block_height: submission.block_height,
            epoch_id: data.epoch_id,
            sender: data.sender.clone(),
            last_node_id: data.data.node_id,
        })
    }
}

impl PerformanceAuxiliary {
    /// Every read is pinned at `height`. `newest_epoch` is the largest epoch among the snapshot's
    /// entries, and `previous` the value held for the newest snapshot below `height`.
    pub(crate) async fn read<C>(
        client: &C,
        height: Height,
        block_time: OffsetDateTime,
        newest_epoch: Option<EpochId>,
        previous: Option<PerformanceAuxiliary>,
    ) -> Result<Self, NyxdError>
    where
        C: PinnedPerformanceQueryClient + Sync,
    {
        let last_submission =
            SubmissionKey::new(&client.get_last_submission_at_height(height).await?);
        // instantiation sets the first weights, so a live contract always has some in force
        let weights = client
            .get_current_weights_at_height(height)
            .await?
            .weights
            .ok_or_else(|| {
                NyxdError::extension_query_failure("performance contract", "no weights in force")
            })?;

        let medians = match previous {
            // nothing was submitted since, so the entries and their medians are as they were
            Some(previous) if previous.last_submission == last_submission => previous.medians,
            previous => Self::read_medians(client, height, newest_epoch, previous).await?,
        };

        Ok(PerformanceAuxiliary {
            weights,
            block_time,
            medians,
            last_submission,
        })
    }

    pub(crate) fn weights(&self) -> EpochWeights {
        self.weights
    }

    pub(crate) fn block_time(&self) -> OffsetDateTime {
        self.block_time
    }

    /// The medians of `node_id`'s bundle at `epoch_id`; `None` when that epoch's are not held.
    pub(crate) fn node_medians(&self, epoch_id: EpochId, node_id: NodeId) -> Option<KindMedians> {
        self.medians.get(&epoch_id)?.get(&node_id).copied()
    }

    /// The medians of the newest entry epoch and the one before it, which holds the nodes whose
    /// chunk of the newest epoch has not landed yet. An epoch is frozen once the next one begins,
    /// so one older than the recorded submission's is kept as held rather than paged again.
    async fn read_medians<C>(
        client: &C,
        height: Height,
        newest_epoch: Option<EpochId>,
        previous: Option<PerformanceAuxiliary>,
    ) -> Result<BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>, NyxdError>
    where
        C: PinnedPerformanceQueryClient + Sync,
    {
        let Some(newest_epoch) = newest_epoch else {
            return Ok(BTreeMap::new());
        };
        let recorded_epoch = previous
            .as_ref()
            .and_then(|previous| previous.last_submission.as_ref())
            .map(|submission| submission.epoch_id);
        let mut held = previous
            .map(|previous| previous.medians)
            .unwrap_or_default();

        let mut medians = BTreeMap::new();
        for epoch in newest_epoch.saturating_sub(1)..=newest_epoch {
            let frozen = recorded_epoch.is_some_and(|recorded| epoch < recorded);

            let epoch_medians = match held.remove(&epoch) {
                Some(held) if frozen => held,
                _ => client
                    .get_all_epoch_performance_at_height(epoch, height)
                    .await?
                    .into_iter()
                    .map(|performance| (performance.node_id, performance.medians))
                    .collect(),
            };
            medians.insert(epoch, epoch_medians);
        }
        Ok(medians)
    }
}

#[cfg(test)]
impl PerformanceAuxiliary {
    /// What a read would hold beside a snapshot, without the recorded submission it skips on.
    pub(crate) fn new_for_tests(
        weights: EpochWeights,
        block_time: OffsetDateTime,
        medians: BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>,
    ) -> Self {
        PerformanceAuxiliary {
            weights,
            block_time,
            medians,
            last_submission: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_performance::attestation::source::PerformanceSnapshotSource;
    use crate::node_performance::attestation::test_helpers::{block_time, percent, weights};
    use crate::support::attested_snapshots::data::{
        AttestedSnapshot, AttestedSnapshotsCacheData, RetainedEntry,
    };
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use async_trait::async_trait;
    use cosmwasm_std::Timestamp;
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_performance_contract_common::{
        LastSubmittedData, LatestPerformance, Measurements, NodePerformance, NodeSubmission,
        WeightsResponse,
    };
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::sync::Mutex;

    const HEIGHT: u32 = 1000;

    #[derive(Clone, Copy, PartialEq)]
    enum FailingRead {
        LastSubmission,
        Weights,
        Page,
    }

    /// The performance contract as of one height, recording which epochs were paged and the
    /// height every read was pinned at.
    struct FakeContract {
        last_submission: LastSubmission,
        weights: Option<EpochWeights>,
        epochs: BTreeMap<EpochId, Vec<NodePerformance>>,
        failing: Option<FailingRead>,
        paged: Mutex<Vec<EpochId>>,
        read_at: Mutex<Vec<Height>>,
    }

    impl FakeContract {
        /// Nodes 1 and 2 hold a bundle in each of `epochs`, its medians all `offset + epoch % 50`.
        fn new(last_submission: LastSubmission, epochs: &[EpochId], offset: u64) -> Self {
            let epochs = epochs
                .iter()
                .map(|&epoch| {
                    let value = offset + epoch as u64 % 50;
                    (epoch, vec![bundle(1, value), bundle(2, value)])
                })
                .collect();
            FakeContract {
                last_submission,
                weights: Some(weights()),
                epochs,
                failing: None,
                paged: Mutex::new(Vec::new()),
                read_at: Mutex::new(Vec::new()),
            }
        }

        fn failing(mut self, read: FailingRead) -> Self {
            self.failing = Some(read);
            self
        }

        fn paged(&self) -> Vec<EpochId> {
            self.paged.lock().unwrap().clone()
        }

        fn read_at(&self) -> Vec<Height> {
            self.read_at.lock().unwrap().clone()
        }

        /// Records the read's height and fails it when scripted to.
        fn read(&self, read: FailingRead, height: Height) -> Result<(), NyxdError> {
            self.read_at.lock().unwrap().push(height);
            if self.failing == Some(read) {
                return Err(NyxdError::extension_query_failure("test", "failed read"));
            }
            Ok(())
        }
    }

    // fine in tests
    #[allow(clippy::unreachable)]
    #[async_trait]
    impl PinnedPerformanceQueryClient for FakeContract {
        async fn get_all_latest_performance_at_height(
            &self,
            _height: Height,
        ) -> Result<Vec<LatestPerformance>, NyxdError> {
            unreachable!("the auxiliary read never pages the committed set")
        }

        async fn get_all_epoch_performance_at_height(
            &self,
            epoch_id: EpochId,
            height: Height,
        ) -> Result<Vec<NodePerformance>, NyxdError> {
            self.paged.lock().unwrap().push(epoch_id);
            self.read(FailingRead::Page, height)?;
            Ok(self.epochs.get(&epoch_id).cloned().unwrap_or_default())
        }

        async fn get_current_weights_at_height(
            &self,
            height: Height,
        ) -> Result<WeightsResponse, NyxdError> {
            self.read(FailingRead::Weights, height)?;
            Ok(WeightsResponse {
                weights: self.weights,
            })
        }

        async fn get_last_submission_at_height(
            &self,
            height: Height,
        ) -> Result<LastSubmission, NyxdError> {
            self.read(FailingRead::LastSubmission, height)?;
            Ok(self.last_submission.clone())
        }
    }

    fn medians(value: u64) -> KindMedians {
        KindMedians {
            liveness: Some(percent(value)),
            stress: Some(percent(value)),
            config: Some(percent(value)),
        }
    }

    fn bundle(node_id: NodeId, value: u64) -> NodePerformance {
        NodePerformance {
            node_id,
            medians: medians(value),
            score: Some(percent(value)),
        }
    }

    fn submission(block_height: u64, epoch_id: EpochId, last_node_id: NodeId) -> LastSubmission {
        LastSubmission {
            block_height,
            block_time: Timestamp::from_seconds(block_height),
            data: Some(LastSubmittedData {
                sender: Addr::unchecked("n1monitor"),
                epoch_id,
                data: NodeSubmission {
                    node_id: last_node_id,
                    measurements: Measurements::default().with_liveness(percent(90)),
                },
            }),
        }
    }

    async fn read(
        contract: &FakeContract,
        newest_epoch: Option<EpochId>,
        previous: Option<PerformanceAuxiliary>,
    ) -> PerformanceAuxiliary {
        PerformanceAuxiliary::read(
            contract,
            Height::from(HEIGHT),
            block_time(HEIGHT),
            newest_epoch,
            previous,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn the_newest_entry_epoch_and_the_one_before_are_paged() {
        let contract = FakeContract::new(submission(950, 100, 2), &[98, 99, 100], 0);
        let aux = read(&contract, Some(100), None).await;

        assert_eq!(contract.paged(), vec![99, 100]);
        assert_eq!(
            aux.medians.keys().copied().collect::<Vec<_>>(),
            vec![99, 100]
        );
        assert_eq!(aux.medians[&99][&1], medians(49));
        assert_eq!(aux.medians[&100][&2], medians(0));
    }

    #[tokio::test]
    async fn every_read_is_pinned_at_the_snapshot_height() {
        let contract = FakeContract::new(submission(950, 100, 2), &[99, 100], 0);
        let aux = read(&contract, Some(100), None).await;

        // the last submission, the weights and both epochs' pages
        assert_eq!(contract.read_at(), vec![Height::from(HEIGHT); 4]);
        assert_eq!(aux.weights, weights());
        assert_eq!(aux.block_time, block_time(HEIGHT));
        assert_eq!(
            aux.last_submission,
            SubmissionKey::new(&submission(950, 100, 2))
        );
    }

    #[tokio::test]
    async fn an_unchanged_last_submission_pages_nothing() {
        let previous = read(
            &FakeContract::new(submission(950, 100, 2), &[99, 100], 0),
            Some(100),
            None,
        )
        .await;

        let contract = FakeContract::new(submission(950, 100, 2), &[99, 100], 0);
        let aux = read(&contract, Some(100), Some(previous.clone())).await;

        assert!(contract.paged().is_empty());
        assert_eq!(aux.medians, previous.medians);
    }

    #[tokio::test]
    async fn a_new_submission_in_the_same_epoch_repages_only_that_epoch() {
        let previous = read(
            &FakeContract::new(submission(950, 100, 2), &[99, 100], 0),
            Some(100),
            None,
        )
        .await;

        // a later chunk of epoch 100 landed; the contract's epoch 99 is offset too, so a re-read
        // of it would show
        let contract = FakeContract::new(submission(960, 100, 5), &[99, 100], 10);
        let aux = read(&contract, Some(100), Some(previous.clone())).await;

        assert_eq!(contract.paged(), vec![100]);
        assert_eq!(aux.medians[&99], previous.medians[&99]);
        assert_eq!(aux.medians[&100][&1], medians(10));
    }

    #[tokio::test]
    async fn a_new_epoch_repages_from_the_recorded_epoch_and_keeps_the_newest_two() {
        let previous = read(
            &FakeContract::new(submission(900, 99, 2), &[98, 99], 0),
            Some(99),
            None,
        )
        .await;

        let contract = FakeContract::new(submission(990, 100, 2), &[98, 99, 100], 0);
        let aux = read(&contract, Some(100), Some(previous)).await;

        assert_eq!(contract.paged(), vec![99, 100]);
        assert_eq!(
            aux.medians.keys().copied().collect::<Vec<_>>(),
            vec![99, 100]
        );
    }

    #[tokio::test]
    async fn a_restart_pages_nothing_when_nothing_landed() -> anyhow::Result<()> {
        let previous = read(
            &FakeContract::new(submission(950, 100, 2), &[99, 100], 0),
            Some(100),
            None,
        )
        .await;
        let restored = round_trip_through_disk_cache(previous)?;

        let contract = FakeContract::new(submission(950, 100, 2), &[99, 100], 0);
        read(&contract, Some(100), Some(restored)).await;

        assert!(contract.paged().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_contract_without_entries_pages_nothing() {
        let no_submission = LastSubmission {
            block_height: 0,
            block_time: Timestamp::from_seconds(0),
            data: None,
        };
        let contract = FakeContract::new(no_submission, &[], 0);
        let aux = read(&contract, None, None).await;

        assert!(contract.paged().is_empty());
        assert!(aux.medians.is_empty());
        assert!(aux.last_submission.is_none());
    }

    async fn fails(contract: FakeContract) -> bool {
        PerformanceAuxiliary::read(
            &contract,
            Height::from(HEIGHT),
            block_time(HEIGHT),
            Some(100),
            None,
        )
        .await
        .is_err()
    }

    fn contract() -> FakeContract {
        FakeContract::new(submission(950, 100, 2), &[99, 100], 0)
    }

    #[tokio::test]
    async fn a_failed_last_submission_read_fails_the_read() {
        assert!(fails(contract().failing(FailingRead::LastSubmission)).await);
    }

    #[tokio::test]
    async fn a_failed_weights_read_fails_the_read() {
        assert!(fails(contract().failing(FailingRead::Weights)).await);
    }

    #[tokio::test]
    async fn a_failed_page_read_fails_the_read() {
        assert!(fails(contract().failing(FailingRead::Page)).await);
    }

    #[tokio::test]
    async fn missing_weights_fail_the_read() {
        let mut contract = contract();
        contract.weights = None;
        assert!(fails(contract).await);
    }

    // The cache is persisted with bincode; a populated value, every `Option` set, must survive it.
    #[tokio::test]
    async fn a_populated_performance_cache_round_trips_through_the_on_disk_format(
    ) -> anyhow::Result<()> {
        let aux = read(&contract(), Some(100), None).await;
        let record = LatestPerformance {
            node_id: 1,
            epoch_id: 100,
            score: Some(percent(87)),
        };
        let snapshot = AttestedSnapshot::new(
            mock_digest_snapshot(Height::from(HEIGHT)).signed(&dummy_ed25519_keypair(1)),
            vec![record.clone()],
            BTreeMap::from([(1, *dummy_ed25519_keypair(2).public_key())]),
        );

        let mut cache = AttestedSnapshotsCacheData::<PerformanceSnapshotSource>::default();
        cache.insert_entry(RetainedEntry::new(snapshot, aux.clone()));

        let restored = round_trip_through_disk_cache(cache)?;
        let entry = restored.get_entry(Height::from(HEIGHT)).unwrap();
        assert_eq!(entry.snapshot().records(), [record]);
        assert_eq!(entry.snapshot().node_identities().len(), 1);
        assert_eq!(entry.auxiliary().medians, aux.medians);
        assert_eq!(entry.auxiliary().weights, aux.weights);
        assert_eq!(entry.auxiliary().block_time, aux.block_time);
        assert_eq!(entry.auxiliary().last_submission, aux.last_submission);
        Ok(())
    }
}
