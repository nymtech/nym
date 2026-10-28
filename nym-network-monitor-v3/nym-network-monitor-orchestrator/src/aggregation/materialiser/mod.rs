// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::AggregationWindows;
use crate::storage::NetworkMonitorStorage;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::Interval;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{error, info, warn};

mod aggregate;
mod config_score;

use aggregate::AggregateMaterialiser;
use config_score::ConfigScoreMaterialiser;

/// Shortest wait between two looks at the chain.
///
/// Matters when an epoch is overdue: an epoch is advanced by a transaction, so the moment it was due
/// to end can pass with the chain still reporting it, and without a floor the loop would spin
/// against the predicted deadline until it finally moved.
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
/// epoch begins.
///
/// Owns the single task and the client it reads the contract's interval with, and drives two
/// members that each produce a different family of figures: [`AggregateMaterialiser`] for the windowed probe aggregates
/// (backfilled across epochs missed while down) and [`ConfigScoreMaterialiser`] for the
/// current-state config-score snapshot (filed only under the epoch in progress, never backfilled).
///
/// Each tick reads the contract's interval once and hands that reading to both, so the two always
/// agree on which epoch is in progress. Epoch identity and timing therefore always come from a
/// reading the contract has just given, never from local arithmetic over an older one: a boundary
/// that disagrees with the chain would file figures under the wrong epoch, which is unrecoverable
/// once published.
pub(crate) struct Materialiser<C> {
    client: C,

    aggregates: AggregateMaterialiser,

    config_scores: ConfigScoreMaterialiser<C>,

    shutdown_token: ShutdownToken,
}

impl<C: MixnetQueryClient + Sync> Materialiser<C> {
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
            shutdown_token,
        }
    }

    /// Runs until the shutdown token is cancelled, materialising each epoch as it begins.
    ///
    /// A tick that cannot read the interval does nothing and waits the floor: neither member can
    /// act without it, and an epoch missed this way is a backfill candidate rather than a loss. Each
    /// member's pass is otherwise isolated, so one failing is logged and left for the next tick
    /// rather than skipping the other.
    pub(crate) async fn run(self) {
        loop {
            // read before waiting rather than after, so that whatever a restart missed is recovered
            // now instead of an epoch from now
            let delay = match self.client.get_current_interval_details().await {
                Ok(details) => {
                    let interval = details.interval;

                    if let Err(err) = self.aggregates.materialise_pending(&interval).await {
                        error!("failed to materialise pending aggregates: {err}");
                    }

                    // config score is filed only under the epoch in progress and never backfilled
                    if let Err(err) = self.config_scores.materialise(&interval).await {
                        error!("failed to materialise config scores: {err}");
                    }

                    until_epoch_end(&interval)
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
