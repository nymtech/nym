// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::AggregationWindows;
use crate::orchestrator::mixnet_epoch::MixnetEpochSource;
use crate::storage::NetworkMonitorStorage;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
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

    /// How long assignment records are kept, which bounds how far back an epoch can be recovered.
    pub(crate) sample_retention: Duration,

    /// Minimum on-chain balance a node must hold to count as able to transact, for config scoring.
    pub(crate) minimum_balance: Coin,

    /// Config-score penalty applied to a node that cannot transact on chain.
    pub(crate) chain_interactions_penalty: f64,
}

/// Produces every per-`(node, epoch)` figure the orchestrator serves, one epoch at a time as that
/// epoch begins.
///
/// Owns the single task and the single [`MixnetEpochSource`], and drives two members that each
/// produce a different family of figures: [`AggregateMaterialiser`] for the windowed probe
/// aggregates (backfilled across epochs missed while down) and [`ConfigScoreMaterialiser`] for the
/// current-state config-score snapshot (filed only under the epoch in progress, never backfilled).
/// The two share nothing but this cadence and the storage handle, so each is a self-contained,
/// separately testable unit; the epoch in progress is read once per tick here and handed to both.
pub(crate) struct Materialiser<C> {
    epochs: MixnetEpochSource<C>,

    aggregates: AggregateMaterialiser,

    config_scores: ConfigScoreMaterialiser<C>,

    shutdown_token: ShutdownToken,
}

impl<C: MixnetQueryClient + Sync> Materialiser<C> {
    /// Builds the materialiser and its two members from one config bundle, handing each half of the
    /// bundle to the member that needs it.
    ///
    /// `config_score_client` is a separate handle from the one inside `epochs`: the epoch source
    /// borrows its client for timing and the config-score member borrows its own for the contract's
    /// scoring params and version history, so neither reaches into the other.
    pub(crate) fn new(
        config: MaterialiserConfig,
        storage: NetworkMonitorStorage,
        epochs: MixnetEpochSource<C>,
        config_score_client: C,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let aggregates =
            AggregateMaterialiser::new(config.windows, config.sample_retention, storage.clone());
        let config_scores = ConfigScoreMaterialiser::new(
            config.minimum_balance,
            config.chain_interactions_penalty,
            config_score_client,
            storage,
        );

        Materialiser {
            epochs,
            aggregates,
            config_scores,
            shutdown_token,
        }
    }

    /// Runs until the shutdown token is cancelled, materialising each epoch as it begins.
    ///
    /// The epoch in progress is read once per tick and handed to both members. A tick that cannot
    /// resolve it does nothing and waits the floor: neither member can act without it, and an epoch
    /// missed this way is a backfill candidate rather than a loss. Each member's pass is otherwise
    /// isolated, so one failing is logged and left for the next tick rather than skipping the other.
    pub(crate) async fn run(mut self) {
        loop {
            // read the epoch before waiting rather than after, so that whatever a restart missed is
            // recovered now instead of an epoch from now
            match self.epochs.current_mixnet_epoch().await {
                Ok(current) => {
                    if let Err(err) = self
                        .aggregates
                        .materialise_pending(&mut self.epochs, current)
                        .await
                    {
                        error!("failed to materialise pending aggregates: {err}");
                    }

                    // config score is filed only under the current epoch and never backfilled, so it
                    // takes the current id directly rather than walking a range
                    if let Err(err) = self.config_scores.materialise(current).await {
                        error!("failed to materialise config scores: {err}");
                    }
                }
                Err(err) => {
                    warn!("could not resolve the current mixnet epoch, skipping this pass: {err}");
                }
            }

            let delay = until_next_check(&mut self.epochs).await;
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = sleep(delay) => {}
            }
        }

        info!("aggregate materialisation stopped");
    }
}

/// How long to wait before looking again: until the epoch in progress is due to end, or the floor
/// when that is already past or cannot be worked out because the chain is unreachable.
async fn until_next_check<C: MixnetQueryClient + Sync>(
    epochs: &mut MixnetEpochSource<C>,
) -> Duration {
    let ends_at = match epochs.current_mixnet_epoch_end().await {
        Ok(ends_at) => ends_at,
        Err(err) => {
            warn!("could not work out when the current mixnet epoch ends: {err}");
            return MIN_CHECK_INTERVAL;
        }
    };

    let remaining = ends_at - OffsetDateTime::now_utc();
    Duration::try_from(remaining)
        .unwrap_or(Duration::ZERO)
        .max(MIN_CHECK_INTERVAL)
}
