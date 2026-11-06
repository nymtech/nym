// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config;
use crate::contract_window::WindowData;
use crate::network_view::DirectoryAnchor;
use crate::prometheus::PrometheusMetric;
use anyhow::{Context, bail};
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_directory_client::key::digest_item_key;
use nym_mixnet_contract_common::Interval;
use nym_task::ShutdownToken;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::error::NyxdError;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info};

mod event_reconciliation;
mod helpers;
mod inputs;
mod rewarded_set_assignment;
mod rewarding;
mod transition_beginning;

/// The epoch loop's periods and the network view's settle lag.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EpochAdvancerConfig {
    /// How many blocks a cadence boundary must lie below the tip before the view is produced
    /// at it.
    pub(crate) settle_lag: u64,

    /// How often the current interval is read while more than this is left of the epoch.
    pub(crate) epoch_check_interval: Duration,

    /// How long after a failed epoch operation it is retried.
    pub(crate) epoch_operations_retry: Duration,
}

impl From<config::Debug> for EpochAdvancerConfig {
    fn from(debug: config::Debug) -> Self {
        EpochAdvancerConfig {
            settle_lag: debug.settle_lag,
            epoch_check_interval: debug.epoch_check_interval,
            epoch_operations_retry: debug.epoch_operations_retry,
        }
    }
}

// naming things is difficult, ok?
// this is struct responsible for advancing an epoch
pub struct EpochAdvancer {
    /// The driver's only sender of transactions, so it needs no lock.
    nyxd_client: DirectSigningHttpRpcNyxdClient,

    /// Where rewarding and active-set selection take each node's performance from, brought up to
    /// date once each epoch's transition has begun; nothing reads it in between.
    window: WindowData,

    /// What the network view each epoch's selection reads is verified against.
    directory_anchor: DirectoryAnchor,

    config: EpochAdvancerConfig,

    shutdown_token: ShutdownToken,
}

impl EpochAdvancer {
    /// The startup checks, in order, failing at the first that does not hold: the account is the
    /// rewarding validator, the directory and performance contracts are configured, the window
    /// loads (so the contract has received a submission), and a network view can be produced.
    pub(crate) async fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        config: EpochAdvancerConfig,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        ensure_rewarding_permission(&nyxd_client).await?;

        // the contracts the network view and the window read
        let directory_contract = nyxd_client
            .directory_contract_address()
            .cloned()
            .context("the network details carry no directory contract address")?;
        if nyxd_client.performance_contract_address().is_none() {
            bail!("the network details carry no performance contract address")
        }

        let directory_anchor = ContractAnchor::new(
            ProvenChainAnchor::new(nyxd_client.clone_query_client()),
            nyxd_client.clone_query_client(),
            directory_contract,
            digest_item_key(),
        );

        let current_epoch = nyxd_client
            .get_current_interval_details()
            .await?
            .interval
            .current_epoch_absolute_id();
        let window = WindowData::load(&nyxd_client, current_epoch)
            .await
            .context("failed to load the performance contract window")?;

        let epoch_advancer = EpochAdvancer {
            nyxd_client,
            window,
            directory_anchor,
            config,
            shutdown_token,
        };

        // each epoch produces its own view, so this one only proves the directory can be read
        epoch_advancer
            .produce_network_view()
            .await
            .context("failed to produce the network view")?;

        Ok(epoch_advancer)
    }

    /// Reads the interval and, once the epoch is over, runs its operations; returns how long
    /// until the interval is read again.
    async fn advance_epoch_if_over(&mut self) -> Duration {
        let current_interval = match self.nyxd_client.get_current_interval_details().await {
            Ok(current_interval) => current_interval,
            Err(err) => {
                error!("failed to obtain information about the current interval - {err}");
                return self.config.epoch_check_interval;
            }
        };
        PrometheusMetric::CurrentEpochEndTimestampSeconds.report(
            current_interval
                .interval
                .current_epoch_end()
                .unix_timestamp(),
        );

        if !current_interval.is_current_epoch_over {
            let time_left = current_interval.time_until_current_epoch_end();
            info!(
                "Waiting for epoch change, it should take approximately {}s",
                time_left.as_secs()
            );
            // a few seconds past the end, to adjust for possible block time drift
            return self
                .config
                .epoch_check_interval
                .min(time_left + Duration::from_secs(10));
        }

        match self
            .perform_epoch_operations(current_interval.interval)
            .await
        {
            // the next epoch's interval is read straight away
            Ok(()) => Duration::ZERO,
            Err(err) => {
                error!("failed to perform epoch operations - {err:#}");
                self.config.epoch_operations_retry
            }
        }
    }

    /// The contract every epoch transaction is sent to.
    fn mixnet_contract(&self) -> Result<AccountId, NyxdError> {
        self.nyxd_client
            .mixnet_contract_address()
            .cloned()
            .ok_or_else(|| NyxdError::unavailable_contract_address("mixnet contract"))
    }

    /// Runs an ended epoch's operations in the contract's order: produce the network view, begin
    /// the transition, bring the window up to date, reward, reconcile the pending events, then
    /// assign the next rewarded set. Each stage re-reads the epoch state and skips what is done,
    /// so a retry resumes where it failed.
    async fn perform_epoch_operations(&mut self, interval: Interval) -> anyhow::Result<()> {
        info!("The current epoch has finished.");
        info!(
            "Interval id: {}, epoch id: {} (absolute epoch id: {})",
            interval.current_interval_id(),
            interval.current_epoch_id(),
            interval.current_epoch_absolute_id()
        );
        info!(
            "The current epoch has lasted from {} until {}",
            interval.current_epoch_start(),
            interval.current_epoch_end()
        );

        info!("Performing all epoch operations...");
        let started = Instant::now();

        // 1. get the current network view first,
        // so that a failed production leaves the contract as it was, nothing begun
        let network_view = self.produce_network_view().await?;

        // 2. begin the epoch transition
        self.begin_epoch_transition().await?;

        // 3. get the most up to date performance figures
        // the transition has begun, so the performance contract accepts nothing more for this epoch
        self.update_performance_window(interval.current_epoch_absolute_id())
            .await?;

        // 4. reward all the nodes in the still current, soon to be previous rewarded set
        self.reward_current_rewarded_set().await?;

        // 5. perform pending events reconciliation (e.g. unbond, undelegate, etc.)
        // note: those operations don't really have to be atomic, so it's fine to send them
        // as separate transactions
        self.reconcile_epoch_events().await?;

        // 6. assign new rewarded set and advance the epoch
        self.update_rewarded_set_and_advance_epoch(&network_view)
            .await?;

        PrometheusMetric::EpochOperationsDurationSeconds.report(started.elapsed().as_secs() as i64);
        Ok(())
    }

    pub(crate) async fn run(mut self) {
        let shutdown_token = self.shutdown_token.clone();
        loop {
            let next_check = self.advance_epoch_if_over().await;
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("EpochAdvancer: Received shutdown");
                    break;
                }
                _ = sleep(next_check) => {}
            }
        }
    }
}

// before going any further, let's check whether we're allowed to perform rewarding
// (if not, let's blow up sooner rather than later)
async fn ensure_rewarding_permission(
    nyxd_client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    let our_address = nyxd_client.address();

    let cosmwasm_addr = nyxd_client
        .get_mixnet_contract_state()
        .await?
        .rewarding_validator_address
        .into_string();

    // this should never fail otherwise it implies either
    // 1) our mixnet contract state is invalid
    // 2) cosmwasm accepts invalid addresses
    // 3) cosmrs fails to parse valid addresses
    // all of those options are BAD
    let allowed_address: AccountId = cosmwasm_addr
        .clone()
        .parse()
        .map_err(|_| NyxdError::MalformedAccountAddress(cosmwasm_addr))?;

    if allowed_address != our_address {
        bail!(
            "Our account ({our_address}) is not permitted to update rewarded set and perform rewarding. The allowed address is {allowed_address}"
        )
    }
    Ok(())
}
