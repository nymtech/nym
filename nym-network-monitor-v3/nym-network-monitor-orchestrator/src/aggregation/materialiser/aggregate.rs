// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::aggregate_window;
use crate::orchestrator::config::AggregationWindows;
use crate::orchestrator::mixnet_epoch::MixnetEpochSource;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{MixnetEpochAggregate, SampleWindow, TestKind};
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use std::time::Duration;
use strum::IntoEnumIterator;
use time::OffsetDateTime;
use tracing::{info, warn};

/// Computes each epoch's windowed probe aggregates once, as that epoch begins.
///
/// The value for an epoch covers the window PRECEDING it, so it is fully determined the instant the
/// epoch opens and is available for the whole of it. That is what the whole arrangement is for: a
/// consumer reads the value the moment the epoch closes and cannot wait for it to be produced.
///
/// Depends only on epoch timing and the stored samples, never on a chain client: it is handed the
/// epoch source to ask when each epoch began, and everything else it needs is in storage.
pub(crate) struct AggregateMaterialiser {
    /// How far back each kind's aggregate reaches.
    windows: AggregationWindows,

    /// How long assignment records are kept, which bounds how far back an epoch can be recovered.
    sample_retention: Duration,

    storage: NetworkMonitorStorage,
}

impl AggregateMaterialiser {
    pub(crate) fn new(
        windows: AggregationWindows,
        sample_retention: Duration,
        storage: NetworkMonitorStorage,
    ) -> Self {
        AggregateMaterialiser {
            windows,
            sample_retention,
            storage,
        }
    }

    /// Computes and stores every node's aggregates for `mixnet_epoch`, one kind at a time.
    ///
    /// Driven by the samples rather than by the registry: an aggregate is only ever written where
    /// runs came back, so the nodes worth considering are exactly the ones a window turned up, and a
    /// node the sweep never reached needs no row to say so.
    ///
    /// What each pass saw is logged per kind, because the difference between a node that was never
    /// assigned work and one whose assignments never came back is real - one is the monitor's
    /// coverage, the other its reliability - and it is deliberately not persisted onto every
    /// aggregate. The log is where that distinction stays visible.
    async fn materialise<C: MixnetQueryClient + Sync>(
        &self,
        epochs: &mut MixnetEpochSource<C>,
        mixnet_epoch: EpochId,
    ) -> anyhow::Result<()> {
        let epoch_start = epochs.mixnet_epoch_start(mixnet_epoch).await?;
        let mut aggregates = Vec::new();

        let retained_from = OffsetDateTime::now_utc() - self.sample_retention;

        for test_kind in TestKind::iter() {
            let window = SampleWindow::ending_at(epoch_start, self.windows.for_kind(test_kind));

            // a window reaching back further than retention would be computed over whatever happens
            // to still be there, producing a plausible figure derived from part of the evidence.
            // that is worse than no figure, so this kind is left without one. checked per kind
            // because the windows differ: an epoch can still be recoverable for liveness at a point
            // where it no longer is for stress
            if window.start < retained_from {
                warn!(
                    mixnet_epoch,
                    %test_kind,
                    window_start = %window.start,
                    %retained_from,
                    "skipping a kind whose window reaches beyond sample retention"
                );
                continue;
            }

            let samples = self
                .storage
                .get_samples_in_window(test_kind, window)
                .await?;

            let mut measured = 0;
            let mut assigned_and_silent = 0;
            for (node_id, node_samples) in samples {
                let Some(aggregated) = aggregate_window(&node_samples) else {
                    assigned_and_silent += 1;
                    continue;
                };

                measured += 1;
                aggregates.push(MixnetEpochAggregate {
                    mixnet_epoch: mixnet_epoch as i64,
                    node_id,
                    test_kind,
                    score: aggregated.score,
                    samples: aggregated.samples as i64,
                });
            }

            info!(
                mixnet_epoch,
                %test_kind,
                window_start = %window.start,
                measured,
                assigned_and_silent,
                "materialised a kind's aggregates"
            );
        }

        self.storage
            .batch_insert_mixnet_epoch_aggregates(&aggregates)
            .await?;
        Ok(())
    }

    /// Materialises every epoch from the one after the last stored through to `current`, the epoch in
    /// progress.
    ///
    /// Deliberately ONE path for what would otherwise be three. In the steady state the range holds
    /// a single epoch, the one that has just begun. After a restart that spanned transitions it
    /// holds the epochs missed, which is the backfill: a hole is permanent once its samples age out,
    /// while recomputing is cheap and the data is usually still there. And on a first deployment
    /// there is no last epoch, so the range starts at the one already in progress - the same code
    /// path as recovery, which is worth having exercised on every deploy rather than only during an
    /// incident.
    ///
    /// Where the range starts is read back from storage rather than remembered by the task, because
    /// a process that has just started has no memory of what the one before it did. When nothing has
    /// advanced the range comes out empty, which is what makes a repeated pass a no-op in its own
    /// right rather than by leaning on the write refusing to overwrite.
    ///
    /// A backfilled value can differ from the one that would have been written at the time, because
    /// results have arrived since. That is accepted: evidence that is more complete is not worse.
    pub(crate) async fn materialise_pending<C: MixnetQueryClient + Sync>(
        &self,
        epochs: &mut MixnetEpochSource<C>,
        current: EpochId,
    ) -> anyhow::Result<()> {
        let last_materialised = self.storage.get_last_materialised_mixnet_epoch().await?;

        // `last + 1` in the steady state; the epoch in progress when there is no last one. a range
        // that starts past `current` is empty, which covers the chain not having advanced yet
        let first = last_materialised.map_or(current, |last| last as EpochId + 1);

        for mixnet_epoch in first..=current {
            self.materialise(epochs, mixnet_epoch).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{NewNymNode, node_with_ips, whole_seconds};
    use async_trait::async_trait;
    use cosmwasm_std::Timestamp;
    use cosmwasm_std::testing::mock_env;
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        CurrentIntervalResponse, Interval, QueryMsg as MixnetQueryMsg,
    };
    use serde::Deserialize;

    const EPOCH_LENGTH: Duration = Duration::from_secs(60 * 60);
    const WINDOW: Duration = Duration::from_secs(2 * 60 * 60);
    const RETENTION: Duration = Duration::from_secs(6 * 60 * 60);
    const NODE: i64 = 1;

    /// A chain sitting at a fixed interval, so a test can say which epoch is in progress and when it
    /// began.
    struct ChainAt(Interval);

    #[async_trait]
    impl MixnetQueryClient for ChainAt {
        async fn query_mixnet_contract<T>(&self, _query: MixnetQueryMsg) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            let response = CurrentIntervalResponse {
                interval: self.0,
                current_blocktime: self.0.current_epoch_start().unix_timestamp() as u64,
                is_current_interval_over: false,
                is_current_epoch_over: false,
            };
            Ok(cosmwasm_std::from_json(cosmwasm_std::to_json_vec(
                &response,
            )?)?)
        }
    }

    /// An interval whose current epoch has absolute id `epochs_elapsed` and began that many epochs
    /// after `first_epoch_start`.
    fn chain_at(first_epoch_start: OffsetDateTime, epochs_elapsed: u32) -> Interval {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(first_epoch_start.unix_timestamp() as u64);

        let mut interval = Interval::init_interval(100, EPOCH_LENGTH, &env);
        for _ in 0..epochs_elapsed {
            interval = interval.advance_epoch();
        }
        interval
    }

    fn aggregate_materialiser(storage: NetworkMonitorStorage) -> AggregateMaterialiser {
        AggregateMaterialiser::new(
            AggregationWindows {
                stress: WINDOW,
                liveness: WINDOW,
            },
            RETENTION,
            storage,
        )
    }

    async fn epochs_at(interval: Interval) -> MixnetEpochSource<ChainAt> {
        MixnetEpochSource::new(ChainAt(interval))
            .await
            .expect("the fixed chain should have answered")
    }

    /// Materialises everything pending for a chain sitting at `interval`, the way the task's loop
    /// does: read the epoch in progress, then backfill up to it.
    async fn run_pending(storage: &NetworkMonitorStorage, interval: Interval) {
        let mut epochs = epochs_at(interval).await;
        let current = epochs
            .current_mixnet_epoch()
            .await
            .expect("the fixed chain should have answered");
        aggregate_materialiser(storage.clone())
            .materialise_pending(&mut epochs, current)
            .await
            .unwrap();
    }

    async fn storage_with_node() -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        let node: NewNymNode = node_with_ips(NODE, "identity", "1.2.3.4");
        storage
            .batch_insert_or_update_nym_nodes(&[node])
            .await
            .unwrap();
        storage
    }

    async fn stored_scores(storage: &NetworkMonitorStorage, mixnet_epoch: i64) -> Vec<f64> {
        storage
            .storage_manager
            .get_mixnet_epoch_aggregates(mixnet_epoch)
            .await
            .unwrap()
            .into_iter()
            .map(|aggregate| aggregate.score)
            .collect()
    }

    // Decision 4 in one test: what is published stops moving. results keep arriving for runs whose
    // assignment already falls inside an anchored window, and a figure a consumer may have read must
    // not change underneath it - but the late result is not lost either, it counts towards the
    // epochs whose windows still contain it
    #[tokio::test]
    async fn a_late_result_leaves_its_epoch_alone_and_counts_towards_the_next() {
        let storage = storage_with_node().await;
        let now = whole_seconds(OffsetDateTime::now_utc());

        // epoch 1 began an hour ago, so its window is [now - 3h, now - 1h); epoch 2 begins now, so
        // its window is [now - 2h, now). a sample 90 minutes back falls in both
        let assigned_at = now - EPOCH_LENGTH - EPOCH_LENGTH / 2;
        let epochs = chain_at(now - EPOCH_LENGTH * 2, 1);

        storage
            .storage_manager
            .insert_scored_sample(NODE, TestKind::Stress, assigned_at, 1.0)
            .await
            .unwrap();
        run_pending(&storage, epochs).await;
        assert_eq!(stored_scores(&storage, 1).await, vec![1.0]);

        // a second run of the same window is submitted late, and would have made epoch 1's mean 0.5
        storage
            .storage_manager
            .insert_scored_sample(NODE, TestKind::Stress, assigned_at, 0.0)
            .await
            .unwrap();
        run_pending(&storage, chain_at(now - EPOCH_LENGTH * 2, 2)).await;

        assert_eq!(
            stored_scores(&storage, 1).await,
            vec![1.0],
            "a published value moved when a late result arrived"
        );
        assert_eq!(
            stored_scores(&storage, 2).await,
            vec![0.5],
            "the late result was lost rather than counting towards a later epoch"
        );
    }

    // a restart spanning transitions would otherwise leave a permanent hole, since a hole cannot be
    // filled once its samples age out while recomputing is cheap and the data is usually still there
    #[tokio::test]
    async fn epochs_missed_while_down_are_backfilled() {
        let storage = storage_with_node().await;
        let now = whole_seconds(OffsetDateTime::now_utc());
        let first_epoch_start = now - EPOCH_LENGTH * 3;

        // epoch 1 starts at now - 2h and covers [now - 4h, now - 2h); epoch 2 starts at now - 1h and
        // covers [now - 3h, now - 1h); epoch 3 starts now and covers [now - 2h, now). so the older
        // sample belongs to epochs 1 and 2, and the newer one to epoch 3 alone
        for (assigned_at, score) in [(now - EPOCH_LENGTH * 3, 0.2), (now - EPOCH_LENGTH, 0.8)] {
            storage
                .storage_manager
                .insert_scored_sample(NODE, TestKind::Stress, assigned_at, score)
                .await
                .unwrap();
        }

        // the orchestrator was up for epoch 1 and then went away
        run_pending(&storage, chain_at(first_epoch_start, 1)).await;
        assert_eq!(stored_scores(&storage, 1).await, vec![0.2]);

        // it comes back two transitions later, and recovers both rather than skipping to the newest
        run_pending(&storage, chain_at(first_epoch_start, 3)).await;

        assert_eq!(
            stored_scores(&storage, 2).await,
            vec![0.2],
            "the epoch missed while down was left as a permanent hole"
        );
        assert_eq!(stored_scores(&storage, 3).await, vec![0.8]);
    }

    // computing over whatever survived eviction would publish a plausible figure derived from part
    // of the evidence, which is worse than publishing none
    #[tokio::test]
    async fn an_epoch_whose_window_predates_retention_is_left_empty() {
        let storage = storage_with_node().await;
        let now = whole_seconds(OffsetDateTime::now_utc());

        // ten hourly epochs, the last of them starting now, against six hours of retention. epoch 1
        // covers [now - 11h, now - 9h), which is long gone; epoch 10 covers [now - 2h, now)
        let epochs_elapsed = 10;
        let first_epoch_start = now - EPOCH_LENGTH * epochs_elapsed;

        for (assigned_at, score) in [(now - EPOCH_LENGTH * 10, 0.1), (now - EPOCH_LENGTH, 0.9)] {
            storage
                .storage_manager
                .insert_scored_sample(NODE, TestKind::Stress, assigned_at, score)
                .await
                .unwrap();
        }

        // epoch 0 was materialised long ago, so the backfill range opens at epoch 1
        storage
            .batch_insert_mixnet_epoch_aggregates(&[MixnetEpochAggregate {
                mixnet_epoch: 0,
                node_id: NODE,
                test_kind: TestKind::Stress,
                score: 1.0,
                samples: 1,
            }])
            .await
            .unwrap();

        run_pending(&storage, chain_at(first_epoch_start, epochs_elapsed)).await;

        assert!(
            stored_scores(&storage, 1).await.is_empty(),
            "an epoch reaching past retention was computed from whatever happened to survive"
        );
        assert_eq!(
            stored_scores(&storage, 10).await,
            vec![0.9],
            "an epoch still covered by retention was skipped along with the aged-out ones"
        );
    }
}
