// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! A local copy of the performance contract's per-epoch medians, enough to resolve every node's
//! figure at the current epoch with the contract's own rule.

use anyhow::bail;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{EpochId, NodeId};
use nym_validator_client::nyxd::contract_traits::performance_query_client::{
    EpochWeights, KindMedians, LastSubmission, NodePerformance,
};
use nym_validator_client::nyxd::contract_traits::{
    PagedPerformanceQueryClient, PerformanceQueryClient,
};
use nym_validator_client::nyxd::nym_performance_contract_common::constants::MAX_FALLBACK_LOOKBACK_EPOCHS;
use nym_validator_client::nyxd::nym_performance_contract_common::resolution::resolve_source;
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::time::Instant;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, info};

pub(crate) mod refresher;

/// How far below the top of the startup walk R may lie before the load gives up. A submission
/// leaves its epoch empty only when every node of its batch was skipped as not bonded, so a gap
/// this long means the contract is not being fed, and walking on could reach epoch 0.
const MAX_EMPTY_EPOCHS_ABOVE_R: EpochId = 24;

/// Shared handle to the window, holding nothing until its first successful load.
#[derive(Clone, Default)]
pub(crate) struct PerformanceContractWindow {
    inner: Arc<RwLock<Option<WindowData>>>,
}

impl PerformanceContractWindow {
    /// The window's data, unless it has never loaded.
    pub(crate) async fn read(&self) -> Option<RwLockReadGuard<'_, WindowData>> {
        RwLockReadGuard::try_map(self.inner.read().await, Option::as_ref).ok()
    }

    /// One refresh with `current_epoch` as X: a full load until the window has seen a submission,
    /// else a re-page of whatever can have changed since the last one it recorded.
    pub(crate) async fn poll<C>(&self, client: &C, current_epoch: EpochId) -> anyhow::Result<()>
    where
        C: PerformanceQueryClient + Sync,
    {
        // held throughout, so that the refresher's and the epoch operations' steps never interleave
        let mut window = self.inner.write().await;
        match window.as_mut() {
            Some(data) if data.has_submission() => data.update(client, current_epoch).await,
            _ => {
                *window = Some(WindowData::load(client, current_epoch).await?);
                Ok(())
            }
        }
    }
}

/// The contract's per-node medians for `[R - 24, R]` and the current epoch X, with the weights at X.
pub(crate) struct WindowData {
    /// An epoch read with no nodes is not held, so it never counts as R.
    epochs: BTreeMap<EpochId, HashMap<NodeId, KindMedians>>,

    current_epoch: EpochId,

    /// The weights in force at the current epoch.
    weights: Option<EpochWeights>,

    /// The newest submission seen, read before the pages it triggered; a change means the epochs
    /// from its epoch to the new one's have to be re-paged.
    last_submission: LastSubmission,

    /// When the last successful step started.
    refreshed_at: OffsetDateTime,
}

impl WindowData {
    /// Pages the newest submission's epoch and each earlier one down to R, the newest holding nodes
    /// at or below X, then on to `R - 24`. Nothing above the newest submission can hold a bundle,
    /// so nothing above it is paged. It is paged even above a lagging X, since X can reach it
    /// without another submission to trigger a re-page.
    async fn load<C>(client: &C, current_epoch: EpochId) -> anyhow::Result<Self>
    where
        C: PerformanceQueryClient + Sync,
    {
        let refreshed_at = OffsetDateTime::now_utc();
        let mut data = WindowData {
            epochs: BTreeMap::new(),
            current_epoch,
            weights: client.get_weights_at(current_epoch).await?.weights,
            last_submission: client.get_last_submission().await?,
            refreshed_at,
        };
        let Some(newest_submitted) = data
            .last_submission
            .data
            .as_ref()
            .map(|submitted| submitted.epoch_id)
        else {
            debug!("the performance contract has received no submission yet, so the window holds nothing");
            return Ok(data);
        };

        let top = newest_submitted;
        info!("building the performance contract window down from epoch {top}");
        let started = Instant::now();
        for epoch_id in (0..=top).rev() {
            // until R is found, it can only lie within the cap below the top
            let lowest = data
                .lookback_floor()
                .unwrap_or(top.saturating_sub(MAX_EMPTY_EPOCHS_ABOVE_R));
            if epoch_id < lowest {
                break;
            }
            data.insert_page(epoch_id, client.get_all_epoch_performance(epoch_id).await?);
        }

        let (Some(lowest), Some(newest)) = (
            data.epochs.keys().next().copied(),
            data.epochs.keys().next_back().copied(),
        ) else {
            bail!(
                "the performance contract has received a submission, but no epoch from {top} down \
                 to {MAX_EMPTY_EPOCHS_ABOVE_R} below it holds any node"
            )
        };
        info!(
            "built the performance contract window in {:?}, holding epochs {lowest} to {newest}",
            started.elapsed()
        );
        Ok(data)
    }

    /// Re-pages every epoch that can have received a submission since the recorded one, reads the
    /// weights at a new X, then trims. Everything is read before anything changes, so a failed
    /// step changes nothing.
    async fn update<C>(&mut self, client: &C, current_epoch: EpochId) -> anyhow::Result<()>
    where
        C: PerformanceQueryClient + Sync,
    {
        let refreshed_at = OffsetDateTime::now_utc();
        let last_submission = client.get_last_submission().await?;

        // epochs only move forward, so an older X comes from a caller with a stale view; taking it
        // could compute the epoch operations' rewards at the previous epoch
        let current_epoch = current_epoch.max(self.current_epoch);
        let weights = if current_epoch != self.current_epoch {
            client.get_weights_at(current_epoch).await?.weights
        } else {
            self.weights
        };
        let mut pages = Vec::new();
        if let Some(epochs) = self.epochs_to_repage(&last_submission) {
            for epoch_id in epochs {
                pages.push((epoch_id, client.get_all_epoch_performance(epoch_id).await?));
            }
        }

        for (epoch_id, nodes) in pages {
            self.insert_page(epoch_id, nodes);
        }
        self.current_epoch = current_epoch;
        self.weights = weights;
        self.last_submission = last_submission;
        self.refreshed_at = refreshed_at;
        self.trim();
        Ok(())
    }

    pub(crate) fn current_epoch(&self) -> EpochId {
        self.current_epoch
    }

    pub(crate) fn refreshed_at(&self) -> OffsetDateTime {
        self.refreshed_at
    }

    /// Whether the contract has ever received a submission.
    pub(crate) fn has_submission(&self) -> bool {
        self.last_submission.data.is_some()
    }

    /// Whether anything was submitted for the current epoch, i.e. R is X.
    pub(crate) fn current_epoch_has_data(&self) -> bool {
        self.epochs.contains_key(&self.current_epoch)
    }

    /// The node's figure at the current epoch, by the contract's own resolution rule.
    pub(crate) fn figure(&self, node_id: NodeId) -> Option<ContractFigure> {
        let source = resolve_source(
            self.current_epoch,
            |epoch_id| Ok(self.medians_at(epoch_id, node_id)),
            |at_or_before| Ok(self.newest_measured(at_or_before)),
            // no pointer is kept: it only ever saves the contract loads
            || Ok(None),
        )
        // the lookups above never fail
        .ok()
        .flatten()?;

        Some(ContractFigure {
            source_epoch: source.epoch_id,
            medians: source.medians,
            score: self
                .weights
                .as_ref()
                .and_then(|weights| weights.weights.score(source.medians)),
        })
    }

    /// The node's rewarding performance at the current epoch: its score, or zero without one.
    /// The score already includes the config median, so nothing is multiplied into it.
    pub(crate) fn rewarding_performance(&self, node_id: NodeId) -> Performance {
        self.figure(node_id)
            .and_then(|figure| figure.score)
            .unwrap_or_default()
    }

    /// The node's selection performance at the current epoch: its score, or 1 for any score above
    /// zero when nothing was submitted for the current epoch; nothing without a score.
    pub(crate) fn selection_performance(&self, node_id: NodeId) -> Option<Performance> {
        let score = self.figure(node_id)?.score?;
        // any constant reduces `saturation x performance^20` to stake alone, while a frozen zero (a
        // node last measured dead) stays out of the stake-weighted draw
        if !self.current_epoch_has_data() && !score.is_zero() {
            Some(Performance::hundred())
        } else {
            Some(score)
        }
    }

    /// The epochs that can have received a submission since the recorded one: a submission lands
    /// in the epoch current at the time, so those from its epoch to the newest one's. `None` when
    /// nothing was submitted since.
    fn epochs_to_repage(&self, newest: &LastSubmission) -> Option<RangeInclusive<EpochId>> {
        if *newest == self.last_submission {
            return None;
        }
        let recorded = self.last_submission.data.as_ref()?.epoch_id;
        Some(recorded..=newest.data.as_ref()?.epoch_id)
    }

    fn medians_at(&self, epoch_id: EpochId, node_id: NodeId) -> Option<KindMedians> {
        self.epochs.get(&epoch_id)?.get(&node_id).copied()
    }

    /// R: the newest held epoch not after `at_or_before`.
    fn newest_measured(&self, at_or_before: EpochId) -> Option<EpochId> {
        self.epochs
            .range(..=at_or_before)
            .next_back()
            .map(|(epoch_id, _)| *epoch_id)
    }

    /// `R - 24` for the current epoch: no epoch below it can be a source.
    fn lookback_floor(&self) -> Option<EpochId> {
        self.newest_measured(self.current_epoch)
            .map(|newest| newest.saturating_sub(MAX_FALLBACK_LOOKBACK_EPOCHS))
    }

    /// Holds the epoch's medians unless it has no nodes, so that it never counts as R. The scores
    /// are dropped: they are under that epoch's own weights, not X's.
    fn insert_page(&mut self, epoch_id: EpochId, nodes: Vec<NodePerformance>) {
        if !nodes.is_empty() {
            let medians = nodes
                .into_iter()
                .map(|node| (node.node_id, node.medians))
                .collect();
            self.epochs.insert(epoch_id, medians);
        }
    }

    fn trim(&mut self) {
        if let Some(floor) = self.lookback_floor() {
            self.epochs.retain(|epoch_id, _| *epoch_id >= floor);
        }
    }
}

/// A node's resolved contract figure at the window's current epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContractFigure {
    pub(crate) source_epoch: EpochId,

    pub(crate) medians: KindMedians,

    /// `None` when the source cannot be scored (no weighted kind measured, or no config).
    pub(crate) score: Option<Percent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cosmwasm_std::{Addr, Timestamp};
    use nym_validator_client::nyxd::contract_traits::performance_query_client::{
        EpochPerformancePagedResponse, PerformanceQueryMsg, Weights, WeightsResponse,
    };
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_performance_contract_common::{
        LastSubmittedData, Measurements, NodeSubmission,
    };
    use serde::Deserialize;
    use std::collections::BTreeSet;
    use std::sync::{Mutex, MutexGuard};

    fn p(value: &str) -> Percent {
        value.parse().unwrap()
    }

    fn medians(liveness: &str, config: &str) -> KindMedians {
        KindMedians {
            liveness: Some(p(liveness)),
            stress: None,
            config: Some(p(config)),
        }
    }

    fn liveness_only() -> Weights {
        Weights {
            liveness: p("1"),
            stress: p("0"),
        }
    }

    fn half_and_half() -> Weights {
        Weights {
            liveness: p("0.5"),
            stress: p("0.5"),
        }
    }

    struct FakeContractState {
        /// Each epoch's bundles, as the medians the contract pages them with.
        bundles: BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>,

        /// Each weights update, by the epoch it takes effect from.
        weights: BTreeMap<EpochId, Weights>,

        last_submission: LastSubmission,

        /// Every query made, in order.
        queries: Vec<PerformanceQueryMsg>,

        /// A submission landing once an epoch's last page has been served, so that the pages just
        /// read miss it.
        landing: Option<(EpochId, NodeId)>,

        /// Fails the next weights read, once.
        fail_weights_read: bool,
    }

    impl FakeContractState {
        fn submit(&mut self, epoch_id: EpochId, node_id: NodeId, medians: KindMedians) {
            self.bundles
                .entry(epoch_id)
                .or_default()
                .insert(node_id, medians);
            self.record_submission(epoch_id, node_id);
        }

        /// Moves the last submission on, as every accepted batch does.
        fn record_submission(&mut self, epoch_id: EpochId, node_id: NodeId) {
            self.last_submission = LastSubmission {
                block_height: self.last_submission.block_height + 1,
                block_time: self.last_submission.block_time,
                data: Some(LastSubmittedData {
                    sender: Addr::unchecked("network-monitor"),
                    epoch_id,
                    data: NodeSubmission {
                        node_id,
                        measurements: Measurements::default(),
                    },
                }),
            };
        }

        /// Every node after the cursor in one page; as with the contract, the cursor is the page's
        /// last node and an empty page ends the paging.
        fn page(
            &self,
            epoch_id: EpochId,
            start_after: Option<NodeId>,
        ) -> EpochPerformancePagedResponse {
            let start = start_after.map_or(0, |node_id| node_id + 1);
            let performance: Vec<_> = self
                .bundles
                .get(&epoch_id)
                .into_iter()
                .flat_map(|nodes| nodes.range(start..))
                .map(|(node_id, medians)| NodePerformance {
                    node_id: *node_id,
                    medians: *medians,
                    score: None,
                })
                .collect();

            EpochPerformancePagedResponse {
                epoch_id,
                start_next_after: performance.last().map(|node| node.node_id),
                performance,
            }
        }
    }

    /// The performance contract as the window reads it: its bundles, weights and last submission,
    /// with every query recorded.
    #[derive(Clone)]
    struct FakeContract(Arc<Mutex<FakeContractState>>);

    impl FakeContract {
        /// No submission yet, with `weights` in force from epoch 0.
        fn new(weights: Weights) -> Self {
            FakeContract(Arc::new(Mutex::new(FakeContractState {
                bundles: BTreeMap::new(),
                weights: BTreeMap::from([(0, weights)]),
                last_submission: LastSubmission {
                    block_height: 1,
                    block_time: Timestamp::from_seconds(0),
                    data: None,
                },
                queries: Vec::new(),
                landing: None,
                fail_weights_read: false,
            })))
        }

        fn state(&self) -> MutexGuard<'_, FakeContractState> {
            self.0.lock().unwrap()
        }

        fn submit(&self, epoch_id: EpochId, node_id: NodeId, medians: KindMedians) {
            self.state().submit(epoch_id, node_id, medians)
        }

        /// A batch whose every node was skipped as not bonded: the last submission moves on, but
        /// no bundle is written.
        fn skipped_batch(&self, epoch_id: EpochId) {
            self.state().record_submission(epoch_id, 1)
        }

        fn update_weights(&self, effective_from: EpochId, weights: Weights) {
            self.state().weights.insert(effective_from, weights);
        }

        fn land_after_next_epoch_read(&self, epoch_id: EpochId, node_id: NodeId) {
            self.state().landing = Some((epoch_id, node_id))
        }

        fn fail_next_weights_read(&self) {
            self.state().fail_weights_read = true
        }

        fn take_queries(&self) -> Vec<PerformanceQueryMsg> {
            std::mem::take(&mut self.state().queries)
        }

        /// The epochs paged since the queries were last taken.
        fn take_paged_epochs(&self) -> BTreeSet<EpochId> {
            self.take_queries()
                .into_iter()
                .filter_map(|query| match query {
                    PerformanceQueryMsg::EpochPerformancePaged { epoch_id, .. } => Some(epoch_id),
                    _ => None,
                })
                .collect()
        }
    }

    #[async_trait]
    impl PerformanceQueryClient for FakeContract {
        async fn query_performance_contract<T>(
            &self,
            query: PerformanceQueryMsg,
        ) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            let mut state = self.state();
            state.queries.push(query.clone());

            let response = match query {
                PerformanceQueryMsg::LastSubmittedMeasurement {} => {
                    cosmwasm_std::to_json_vec(&state.last_submission)
                }
                PerformanceQueryMsg::WeightsAt { epoch_id } => {
                    if std::mem::take(&mut state.fail_weights_read) {
                        // any failed read would do
                        return Err(NyxdError::InvalidHeight);
                    }
                    let weights = state.weights.range(..=epoch_id).next_back().map(
                        |(effective_from, weights)| EpochWeights {
                            effective_from: *effective_from,
                            weights: *weights,
                        },
                    );
                    cosmwasm_std::to_json_vec(&WeightsResponse { weights })
                }
                PerformanceQueryMsg::EpochPerformancePaged {
                    epoch_id,
                    start_after,
                    ..
                } => {
                    let page = state.page(epoch_id, start_after);
                    if page.start_next_after.is_none() {
                        if let Some((epoch_id, node_id)) = state.landing.take() {
                            state.submit(epoch_id, node_id, medians("1", "1"));
                        }
                    }
                    cosmwasm_std::to_json_vec(&page)
                }
                other => panic!("the window makes no {other:?} query"),
            };

            // the trait is generic over the response, so it goes through its wire form
            Ok(cosmwasm_std::from_json(response.unwrap()).unwrap())
        }
    }

    fn held(data: &WindowData) -> Vec<EpochId> {
        data.epochs.keys().copied().collect()
    }

    /// A window after its first poll at `current_epoch`.
    async fn loaded(contract: &FakeContract, current_epoch: EpochId) -> PerformanceContractWindow {
        let window = PerformanceContractWindow::default();
        window.poll(contract, current_epoch).await.unwrap();
        window
    }

    /// The window's data loaded at `current_epoch`, for resolving figures.
    async fn data_at(contract: &FakeContract, current_epoch: EpochId) -> WindowData {
        WindowData::load(contract, current_epoch).await.unwrap()
    }

    #[tokio::test]
    async fn a_start_after_an_outage_loads_only_what_the_rule_can_reach() {
        let contract = FakeContract::new(liveness_only());
        contract.update_weights(120, half_and_half());
        for epoch_id in 60..=100 {
            contract.submit(epoch_id, 1, medians("1", "1"));
        }

        let window = loaded(&contract, 130).await;

        let data = window.read().await.unwrap();
        assert_eq!(held(&data), (76..=100).collect::<Vec<_>>());
        assert!(!data.current_epoch_has_data());
        assert_eq!(
            data.weights,
            Some(EpochWeights {
                effective_from: 120,
                weights: half_and_half(),
            })
        );
        assert_eq!(contract.take_paged_epochs(), (76..=100).collect());
    }

    #[tokio::test]
    async fn a_newest_submission_whose_epoch_holds_nothing_walks_down_to_r() {
        let contract = FakeContract::new(liveness_only());
        for epoch_id in 60..=98 {
            contract.submit(epoch_id, 1, medians("1", "1"));
        }
        contract.skipped_batch(100);

        let window = loaded(&contract, 100).await;

        let data = window.read().await.unwrap();
        assert_eq!(held(&data), (74..=98).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn a_start_behind_the_newest_submission_holds_it_once_x_reaches_it() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(99, 1, medians("1", "1"));
        contract.submit(100, 1, medians("0.5", "1"));

        // the mixnet contract cache is still an epoch behind the chain
        let window = loaded(&contract, 99).await;
        assert_eq!(contract.take_paged_epochs(), (75..=100).collect());

        // the cache catches up, with nothing new submitted
        window.poll(&contract, 100).await.unwrap();

        let data = window.read().await.unwrap();
        assert!(data.current_epoch_has_data());
        assert_eq!(data.figure(1).map(|figure| figure.source_epoch), Some(100));
    }

    #[tokio::test]
    async fn a_load_finding_nothing_near_the_newest_submission_fails() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(10, 1, medians("1", "1"));
        contract.skipped_batch(100);

        let window = PerformanceContractWindow::default();
        assert!(window.poll(&contract, 100).await.is_err());

        assert!(window.read().await.is_none());
        assert_eq!(contract.take_paged_epochs(), (76..=100).collect());
    }

    #[tokio::test]
    async fn a_contract_without_a_submission_yields_an_empty_window() {
        let contract = FakeContract::new(liveness_only());

        let window = loaded(&contract, 10).await;

        let data = window.read().await.unwrap();
        assert!(!data.has_submission());
        assert!(held(&data).is_empty());
        assert_eq!(data.figure(1), None);
        assert!(contract.take_paged_epochs().is_empty());
    }

    #[tokio::test]
    async fn a_first_submission_loads_the_window_afresh() {
        let contract = FakeContract::new(liveness_only());
        let window = loaded(&contract, 10).await;

        contract.submit(10, 1, medians("1", "1"));
        window.poll(&contract, 10).await.unwrap();

        assert!(window.read().await.unwrap().current_epoch_has_data());
    }

    #[tokio::test]
    async fn a_new_submission_at_x_is_paged_and_becomes_r() {
        let contract = FakeContract::new(liveness_only());
        for epoch_id in 60..=100 {
            contract.submit(epoch_id, 1, medians("1", "1"));
        }
        let window = loaded(&contract, 101).await;
        contract.take_queries();

        contract.submit(101, 1, medians("1", "1"));
        window.poll(&contract, 101).await.unwrap();

        let data = window.read().await.unwrap();
        assert!(data.current_epoch_has_data());
        assert_eq!(held(&data), (77..=101).collect::<Vec<_>>());
        // from the recorded submission's epoch to the new one's
        assert_eq!(contract.take_paged_epochs(), BTreeSet::from([100, 101]));
    }

    #[tokio::test]
    async fn an_unchanged_last_submission_reads_nothing_else() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(100, 1, medians("1", "1"));
        let window = loaded(&contract, 100).await;
        contract.take_queries();

        window.poll(&contract, 100).await.unwrap();

        assert_eq!(
            contract.take_queries(),
            vec![PerformanceQueryMsg::LastSubmittedMeasurement {}]
        );
    }

    #[tokio::test]
    async fn a_submission_landing_after_the_pages_shows_up_at_the_next_poll() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(100, 1, medians("1", "1"));
        let window = loaded(&contract, 100).await;

        // node 2's chunk triggers a re-page of epoch 100, and node 3's lands right after it
        contract.submit(100, 2, medians("1", "1"));
        contract.land_after_next_epoch_read(100, 3);
        window.poll(&contract, 100).await.unwrap();
        assert_eq!(window.read().await.unwrap().figure(3), None);

        window.poll(&contract, 100).await.unwrap();
        let figure = window.read().await.unwrap().figure(3);
        assert_eq!(figure.map(|figure| figure.source_epoch), Some(100));
    }

    #[tokio::test]
    async fn a_new_epoch_reads_its_weights_and_drops_nothing_while_r_stays() {
        let contract = FakeContract::new(liveness_only());
        for epoch_id in 76..=100 {
            contract.submit(epoch_id, 1, medians("1", "1"));
        }
        let window = loaded(&contract, 100).await;
        contract.take_queries();

        contract.update_weights(101, half_and_half());
        window.poll(&contract, 101).await.unwrap();

        let data = window.read().await.unwrap();
        assert_eq!(data.current_epoch(), 101);
        assert!(!data.current_epoch_has_data());
        assert_eq!(
            data.weights,
            Some(EpochWeights {
                effective_from: 101,
                weights: half_and_half(),
            })
        );
        assert_eq!(held(&data), (76..=100).collect::<Vec<_>>());
        assert_eq!(
            contract.take_queries(),
            vec![
                PerformanceQueryMsg::LastSubmittedMeasurement {},
                PerformanceQueryMsg::WeightsAt { epoch_id: 101 },
            ]
        );
    }

    #[tokio::test]
    async fn a_poll_with_an_older_epoch_keeps_the_current_one() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(100, 1, medians("1", "1"));
        let window = loaded(&contract, 101).await;
        contract.take_queries();

        window.poll(&contract, 100).await.unwrap();

        assert_eq!(window.read().await.unwrap().current_epoch(), 101);
        assert_eq!(
            contract.take_queries(),
            vec![PerformanceQueryMsg::LastSubmittedMeasurement {}]
        );
    }

    #[tokio::test]
    async fn a_late_chunk_is_not_lost_to_the_next_epochs_first() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(100, 1, medians("1", "1"));
        let window = loaded(&contract, 100).await;

        // between two polls: a late chunk for 100, the epoch moving on, and 101's first chunk
        contract.submit(100, 2, medians("0.5", "1"));
        contract.submit(101, 1, medians("1", "1"));
        window.poll(&contract, 101).await.unwrap();

        let data = window.read().await.unwrap();
        assert!(data.current_epoch_has_data());
        assert_eq!(data.figure(2).map(|figure| figure.source_epoch), Some(100));
    }

    #[tokio::test]
    async fn a_failed_step_leaves_the_window_as_it_was() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(100, 1, medians("1", "1"));
        let window = loaded(&contract, 100).await;

        contract.update_weights(101, half_and_half());
        contract.submit(101, 1, medians("1", "1"));
        contract.fail_next_weights_read();
        assert!(window.poll(&contract, 101).await.is_err());
        {
            let data = window.read().await.unwrap();
            assert_eq!(data.current_epoch(), 100);
            assert_eq!(
                data.weights.as_ref().map(|weights| weights.effective_from),
                Some(0)
            );
            assert_eq!(held(&data), vec![100]);
        }

        // the submission was not recorded either, so the next step still pages epoch 101
        window.poll(&contract, 101).await.unwrap();
        let data = window.read().await.unwrap();
        assert_eq!(data.current_epoch(), 101);
        assert_eq!(
            data.weights.as_ref().map(|weights| weights.effective_from),
            Some(101)
        );
        assert!(data.current_epoch_has_data());
    }

    // the figures below mirror the contract's own `RewardingInputs` tests, over the same bundles

    #[tokio::test]
    async fn resolves_the_bundle_of_the_current_epoch() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(10, 1, medians("0.8", "0.5"));

        assert_eq!(
            data_at(&contract, 10).await.figure(1),
            Some(ContractFigure {
                source_epoch: 10,
                medians: medians("0.8", "0.5"),
                score: Some(p("0.4")),
            })
        );
    }

    #[tokio::test]
    async fn falls_back_to_the_newest_earlier_bundle_within_the_lookback() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(8, 1, medians("0.8", "1"));
        contract.submit(12, 1, medians("0.9", "1"));

        for current_epoch in [10, 11] {
            assert_eq!(
                data_at(&contract, current_epoch).await.figure(1),
                Some(ContractFigure {
                    source_epoch: 8,
                    medians: medians("0.8", "1"),
                    score: Some(p("0.8")),
                })
            );
        }

        let figure = data_at(&contract, 13).await.figure(1).unwrap();
        assert_eq!((figure.source_epoch, figure.score), (12, Some(p("0.9"))));
    }

    #[tokio::test]
    async fn does_not_reach_past_the_lookback() {
        let contract = FakeContract::new(liveness_only());
        contract.submit(10, 1, medians("0.8", "1"));
        // node 2 is measured every epoch, so the lookback anchor keeps moving
        let edge = 10 + MAX_FALLBACK_LOOKBACK_EPOCHS;
        for epoch_id in 10..=edge + 1 {
            contract.submit(epoch_id, 2, medians("1", "1"));
        }

        let figure = data_at(&contract, edge).await.figure(1).unwrap();
        assert_eq!((figure.source_epoch, figure.score), (10, Some(p("0.8"))));

        assert_eq!(data_at(&contract, edge + 1).await.figure(1), None);
    }

    #[tokio::test]
    async fn scores_with_the_weights_of_the_current_epoch_not_the_source() {
        let contract = FakeContract::new(liveness_only());
        let with_stress = KindMedians {
            stress: Some(p("0.5")),
            ..medians("1", "1")
        };
        contract.submit(10, 1, with_stress);
        contract.update_weights(11, half_and_half());

        // at 10 the stress median does not count
        let figure = data_at(&contract, 10).await.figure(1).unwrap();
        assert_eq!(figure.score, Some(p("1")));

        // at 12 the same bundle is scored under the weights in force at 12
        let figure = data_at(&contract, 12).await.figure(1).unwrap();
        assert_eq!((figure.source_epoch, figure.score), (10, Some(p("0.75"))));
    }

    #[tokio::test]
    async fn a_source_without_a_config_median_cannot_be_scored() {
        let contract = FakeContract::new(liveness_only());
        let unconfigured = KindMedians {
            config: None,
            ..medians("0.8", "1")
        };
        contract.submit(10, 1, unconfigured);

        assert_eq!(
            data_at(&contract, 10).await.figure(1),
            Some(ContractFigure {
                source_epoch: 10,
                medians: unconfigured,
                score: None,
            })
        );
    }

    /// Node 1 scored 0.4, node 2 scored 0 and node 3 unscoreable at epoch 10; node 4 never
    /// measured.
    fn scored_at_10() -> FakeContract {
        let contract = FakeContract::new(liveness_only());
        contract.submit(10, 1, medians("0.4", "1"));
        contract.submit(10, 2, medians("0", "1"));
        contract.submit(
            10,
            3,
            KindMedians {
                config: None,
                ..medians("0.8", "1")
            },
        );
        contract
    }

    #[tokio::test]
    async fn rewarding_pays_the_contract_score_as_it_is() {
        let data = data_at(&scored_at_10(), 10).await;

        assert_eq!(data.rewarding_performance(1), p("0.4"));
    }

    #[tokio::test]
    async fn a_node_without_a_contract_score_is_paid_zero() {
        let data = data_at(&scored_at_10(), 10).await;

        // a source the contract cannot score, or no source at all
        assert!(data.rewarding_performance(3).is_zero());
        assert!(data.rewarding_performance(4).is_zero());
    }

    #[tokio::test]
    async fn selection_scores_pass_through_while_the_epoch_holds_data() {
        let data = data_at(&scored_at_10(), 10).await;

        assert_eq!(data.selection_performance(1), Some(p("0.4")));
        assert_eq!(data.selection_performance(2), Some(p("0")));
    }

    #[tokio::test]
    async fn a_candidate_without_a_contract_score_is_not_selected() {
        // at 11 nothing was submitted for the current epoch
        for current_epoch in [10, 11] {
            let data = data_at(&scored_at_10(), current_epoch).await;

            assert_eq!(data.selection_performance(3), None);
            assert_eq!(data.selection_performance(4), None);
        }
    }

    #[tokio::test]
    async fn without_a_submission_for_the_epoch_selection_is_by_stake_alone_but_keeps_a_zero_out() {
        let data = data_at(&scored_at_10(), 11).await;

        assert_eq!(data.selection_performance(1), Some(Performance::hundred()));
        assert_eq!(data.selection_performance(2), Some(Performance::zero()));
    }
}
