// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::AggregationWindows;
use crate::storage::NetworkMonitorStorage;
use crate::submission::PerformanceSubmitter;
use anyhow::Context;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, PerformanceQueryClient, PerformanceSigningClient,
};
use nym_validator_client::nyxd::nym_mixnet_contract_common::Interval;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{error, info, warn};

mod aggregate;
mod config_score;

use aggregate::AggregateMaterialiser;
use config_score::ConfigScoreMaterialiser;

/// Shortest wait between two looks at the chain, and the wait before retrying a failed pass.
///
/// Matters when an epoch is overdue: an epoch is advanced by a transaction, so the moment it was due
/// to end can pass with the chain still reporting it, and without a floor the loop would spin
/// against the predicted deadline until it finally moved. A failed pass waits only this long rather
/// than until the epoch's end, because the contract accepts a submission only while its epoch is in
/// progress.
const MIN_CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// The value configuration the materialiser needs, grouped so construction takes one bundle rather
/// than a long argument list.
pub(crate) struct MaterialiserConfig {
    /// How far back each kind's aggregate reaches.
    pub(crate) windows: AggregationWindows,

    /// How long completed runs are kept, which bounds how far back an epoch can be recovered.
    pub(crate) testrun_retention: Duration,

    /// Minimum on-chain balance a node must hold to count as able to transact, for config scoring.
    pub(crate) minimum_balance: Coin,

    /// Config-score penalty applied to a node that cannot transact on chain.
    pub(crate) chain_interactions_penalty: f64,
}

/// Produces every per-`(node, epoch)` figure the orchestrator serves, one epoch at a time as that
/// epoch begins, and submits the epoch in progress's figures to the performance contract.
///
/// Owns the single task and the client it reads the contract's interval with, and drives two
/// members that each produce a different family of figures: [`AggregateMaterialiser`] for the windowed probe aggregates
/// (backfilled across epochs missed while down) and [`ConfigScoreMaterialiser`] for the
/// current-state config-score snapshot (filed only under the epoch in progress, never backfilled).
/// When submission is on, a [`PerformanceSubmitter`] sends what both produced for the epoch in
/// progress as the pass's last step.
///
/// Each tick reads the contract's interval once and hands that reading to every step, so they always
/// agree on which epoch is in progress. Epoch identity and timing therefore always come from a
/// reading the contract has just given, never from local arithmetic over an older one: a boundary
/// that disagrees with the chain would file figures under the wrong epoch, which is unrecoverable
/// once published.
pub(crate) struct Materialiser<C, S> {
    client: C,

    aggregates: AggregateMaterialiser,

    config_scores: ConfigScoreMaterialiser<C>,

    /// `None` while submission is off.
    submitter: Option<PerformanceSubmitter<S>>,

    shutdown_token: ShutdownToken,
}

impl<C, S> Materialiser<C, S>
where
    C: MixnetQueryClient + Sync,
    S: PerformanceQueryClient + PerformanceSigningClient + Sync,
{
    /// Builds the materialiser and its two members from one config bundle, handing each part of the
    /// bundle to the member that needs it.
    ///
    /// `config_score_client` is a separate handle from `client`: the materialiser reads the interval
    /// through its own and the config-score member reads the contract's scoring params and version
    /// history through its own.
    pub(crate) fn new(
        config: MaterialiserConfig,
        storage: NetworkMonitorStorage,
        client: C,
        config_score_client: C,
        submitter: Option<PerformanceSubmitter<S>>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let aggregates =
            AggregateMaterialiser::new(config.windows, config.testrun_retention, storage.clone());
        let config_scores = ConfigScoreMaterialiser::new(
            config.minimum_balance,
            config.chain_interactions_penalty,
            config_score_client,
            storage,
        );

        Materialiser {
            client,
            aggregates,
            config_scores,
            submitter,
            shutdown_token,
        }
    }

    /// Runs until the shutdown token is cancelled, materialising each epoch as it begins.
    ///
    /// A tick that cannot read the interval does nothing and waits the floor: no step can act
    /// without it, and an epoch missed this way is a backfill candidate rather than a loss. A pass
    /// that fails is retried after the floor as well, until one completes: every step is safe to
    /// repeat.
    pub(crate) async fn run(self) {
        loop {
            // read before waiting rather than after, so that whatever a restart missed is recovered
            // now instead of an epoch from now
            let delay = match self.client.get_current_interval_details().await {
                Ok(details) => {
                    let interval = details.interval;
                    let result = self.materialise_and_submit(&interval).await;
                    if let Err(err) = &result {
                        error!("{err:#}");
                    }
                    delay_after(result.is_ok(), &interval)
                }
                Err(err) => {
                    warn!("could not read the mixnet interval, skipping this pass: {err}");
                    MIN_CHECK_INTERVAL
                }
            };

            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = sleep(delay) => {}
            }
        }

        info!("materialisation stopped");
    }

    /// Runs every step against one interval reading, stopping at the first that fails.
    ///
    /// The submission runs only while the epoch is in progress: an epoch in transition is skipped
    /// without failing, since the contract would reject it anyway.
    async fn materialise_and_submit(&self, interval: &Interval) -> anyhow::Result<()> {
        self.aggregates
            .materialise_pending(interval)
            .await
            .context("failed to materialise pending aggregates")?;

        // config score is filed only under the epoch in progress and never backfilled
        self.config_scores
            .materialise(interval)
            .await
            .context("failed to materialise config scores")?;

        let Some(submitter) = &self.submitter else {
            return Ok(());
        };

        let mixnet_epoch = interval.current_epoch_absolute_id();
        let status = self
            .client
            .get_current_epoch_status()
            .await
            .context("failed to read the mixnet epoch status")?;
        if !status.is_in_progress() {
            info!(
                mixnet_epoch,
                "the epoch's transition has begun, skipping its submission"
            );
            return Ok(());
        }

        submitter.submit(mixnet_epoch).await.with_context(|| {
            format!("failed to submit epoch {mixnet_epoch} to the performance contract")
        })
    }
}

/// How long to wait after a pass: the floor after a failure, so the epoch in progress is retried
/// while the contract still accepts it, else until that epoch is due to end.
fn delay_after(succeeded: bool, interval: &Interval) -> Duration {
    if succeeded {
        until_epoch_end(interval)
    } else {
        MIN_CHECK_INTERVAL
    }
}

/// How long to wait before looking again: until the epoch in progress is due to end, or the floor
/// when that is already past.
///
/// Due rather than guaranteed: an epoch is advanced by a transaction, so the chain can be late, and
/// the next tick has to cope with the epoch still being current.
fn until_epoch_end(interval: &Interval) -> Duration {
    let remaining = interval.current_epoch_end() - OffsetDateTime::now_utc();
    Duration::try_from(remaining)
        .unwrap_or(Duration::ZERO)
        .max(MIN_CHECK_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::aggregate::tests::{
        EPOCH_LENGTH, NODE, RETENTION, WINDOW, chain_at, now, storage_with_node, store_run,
        stored_scores,
    };
    use super::*;
    use crate::storage::models::{FIXTURE_EPOCH, config_score};
    use crate::submission::tests::{FakeContract, submitter_over};
    use async_trait::async_trait;
    use cosmwasm_std::Addr;
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        EpochStatus, QueryMsg as MixnetQueryMsg,
    };
    use serde::Deserialize;

    /// A mixnet contract whose epoch is in progress but whose every other query fails: the config
    /// step cannot read its params, while nothing about the epoch would stop a submission.
    struct ConfigUnreadableMixnet;

    #[async_trait]
    impl MixnetQueryClient for ConfigUnreadableMixnet {
        async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            let MixnetQueryMsg::GetEpochStatus {} = query else {
                return Err(NyxdError::unavailable_contract_address("mixnet contract"));
            };

            // the trait is generic over the response, so it goes through its wire form
            let status = EpochStatus::new(Addr::unchecked("validator"));
            Ok(cosmwasm_std::from_json(cosmwasm_std::to_json_vec(&status).unwrap()).unwrap())
        }
    }

    /// A materialiser over `storage` whose config step always fails, submitting to `contract`.
    fn config_failing_materialiser(
        storage: NetworkMonitorStorage,
        contract: &FakeContract,
    ) -> Materialiser<ConfigUnreadableMixnet, FakeContract> {
        let submitter = submitter_over(contract, storage.clone());

        Materialiser::new(
            MaterialiserConfig {
                windows: AggregationWindows {
                    stress: WINDOW,
                    liveness: WINDOW,
                },
                testrun_retention: RETENTION,
                minimum_balance: Coin::new(1_000_000u128, "unym"),
                chain_interactions_penalty: 0.2,
            },
            storage,
            ConfigUnreadableMixnet,
            ConfigUnreadableMixnet,
            Some(submitter),
            ShutdownToken::new(),
        )
    }

    // waiting for the epoch's end would lose its submission, since the contract only accepts the
    // epoch in progress
    #[test]
    fn a_failed_pass_is_retried_after_the_floor_however_far_the_epochs_end() {
        let epoch_ending_in_an_hour = chain_at(now(), 0);

        assert_eq!(
            delay_after(false, &epoch_ending_in_an_hour),
            MIN_CHECK_INTERVAL
        );
    }

    #[test]
    fn a_completed_pass_waits_until_the_epoch_is_due_to_end() {
        let epoch_ending_in_an_hour = chain_at(now(), 0);

        // an hour, less however long the test took to get here
        let delay = delay_after(true, &epoch_ending_in_an_hour);
        assert!(
            delay > EPOCH_LENGTH - MIN_CHECK_INTERVAL && delay <= EPOCH_LENGTH,
            "expected the time to the epoch's end, got {delay:?}"
        );
    }

    // a retry repeats every step, so the aggregates step must leave what the failed pass stored as
    // it was rather than recompute it
    #[tokio::test]
    async fn a_pass_whose_config_step_fails_reports_failure_and_its_retry_leaves_the_aggregates_alone()
     {
        let storage = storage_with_node().await;
        let now = now();
        let materialiser = config_failing_materialiser(storage.clone(), &FakeContract::default());

        // epoch 1 began an hour ago, so its window [now - 3h, now - 1h) holds a run 90 minutes back
        let tested_at = now - EPOCH_LENGTH - EPOCH_LENGTH / 2;
        let interval = chain_at(now - EPOCH_LENGTH * 2, 1);
        store_run(&storage, tested_at, 10).await;

        assert!(
            materialiser
                .materialise_and_submit(&interval)
                .await
                .is_err(),
            "a pass whose config step failed reported success"
        );
        assert_eq!(stored_scores(&storage, 1).await, vec![1.0]);

        // a late run in the same window would make a recomputed figure 0.5
        store_run(&storage, tested_at, 0).await;

        assert!(
            materialiser
                .materialise_and_submit(&interval)
                .await
                .is_err()
        );
        assert_eq!(
            stored_scores(&storage, 1).await,
            vec![1.0],
            "the retry recomputed an epoch the failed pass had already stored"
        );
    }

    // a submission built from part of an epoch's figures would be filed as the whole of it. the
    // epoch is in progress, a config score is stored and the cursor would take a batch, so only the
    // failed step stands between the pass and a submission
    #[tokio::test]
    async fn a_pass_whose_config_step_fails_submits_nothing() {
        let storage = storage_with_node().await;
        let now = now();
        let contract = FakeContract::with_cursor(FIXTURE_EPOCH - 1, 0);
        let materialiser = config_failing_materialiser(storage.clone(), &contract);

        // the epoch began an hour ago, its window holds a run, and an earlier pass in it had already
        // scored the node's config
        let interval = chain_at(now - EPOCH_LENGTH * (FIXTURE_EPOCH + 1), FIXTURE_EPOCH);
        store_run(&storage, now - EPOCH_LENGTH - EPOCH_LENGTH / 2, 10).await;
        storage
            .batch_insert_mixnet_epoch_config_scores(&[config_score(NODE, 1.0)])
            .await
            .unwrap();

        assert!(
            materialiser
                .materialise_and_submit(&interval)
                .await
                .is_err()
        );
        assert!(
            contract.batches().is_empty(),
            "a pass whose config step failed still submitted"
        );
    }
}
