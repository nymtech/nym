// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract_window::WindowData;
use anyhow::{Context, bail};
use nym_mixnet_contract_common::{CurrentIntervalResponse, Interval, NymNodeDetails};
use nym_task::{ShutdownManager, ShutdownToken};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::error::NyxdError;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

mod event_reconciliation;
mod helpers;
mod rewarded_set_assignment;
mod rewarding;
mod transition_beginning;

// naming things is difficult, ok?
// this is struct responsible for advancing an epoch
pub struct EpochAdvancer {
    /// The driver's only sender of transactions, so it needs no lock.
    nyxd_client: DirectSigningHttpRpcNyxdClient,

    /// Where rewarding and active-set selection take each node's performance from, kept current
    /// by the loop while it waits for the epoch to end.
    window: WindowData,
}

impl EpochAdvancer {
    /// Loads the window, failing when the performance contract has never received a submission.
    pub(crate) async fn new(nyxd_client: DirectSigningHttpRpcNyxdClient) -> anyhow::Result<Self> {
        let current_epoch = nyxd_client
            .get_current_interval_details()
            .await?
            .interval
            .current_epoch_absolute_id();
        let window = WindowData::load(&nyxd_client, current_epoch)
            .await
            .context("failed to load the performance contract window")?;

        Ok(EpochAdvancer {
            nyxd_client,
            window,
        })
    }

    pub(crate) async fn current_interval_details(&self) -> anyhow::Result<CurrentIntervalResponse> {
        Ok(self.nyxd_client.get_current_interval_details().await?)
    }

    /// The contract every epoch transaction is sent to.
    fn mixnet_contract(&self) -> Result<AccountId, NyxdError> {
        self.nyxd_client
            .mixnet_contract_address()
            .cloned()
            .ok_or_else(|| NyxdError::unavailable_contract_address("mixnet contract"))
    }

    /// Runs an ended epoch's operations in the contract's order: begin the transition, bring the
    /// window up to date, reward, reconcile the pending events, then assign the next rewarded set.
    /// Each stage re-reads the epoch state and skips what is done, so a retry resumes where it
    /// failed.
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

        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        if !epoch_status.is_in_progress() {
            let address = self.nyxd_client.address();

            if epoch_status.being_advanced_by.as_str() != address.as_ref() {
                // another account is already handling
                error!(
                    "another account ({}) is already advancing the epoch... but only the rewarding validator should!",
                    epoch_status.being_advanced_by
                );
                return Ok(());
            } else {
                warn!("we seem to have crashed mid-epoch advancement...");
            }
        } else {
            let should_continue = self.begin_epoch_transition().await?;
            if !should_continue {
                return Ok(());
            }
        }

        // the transition has begun, so the performance contract accepts nothing more for this epoch
        self.window
            .update(&self.nyxd_client, interval.current_epoch_absolute_id())
            .await?;

        let mut rewards = self.nodes_to_reward().await?;
        rewards.sort_by_key(|a| a.node_id);

        // Reward all the nodes in the still current, soon to be previous rewarded set
        info!("Rewarding the current rewarded set...");
        self.reward_current_rewarded_set(rewards, interval).await?;

        // note: those operations don't really have to be atomic, so it's fine to send them
        // as separate transactions
        self.reconcile_epoch_events().await?;

        let nym_nodes: Vec<NymNodeDetails> =
            todo!("the selection candidates come from the network view");
        self.update_rewarded_set_and_advance_epoch(&nym_nodes)
            .await?;

        Ok(())
    }

    async fn wait_until_epoch_end(&mut self, shutdown_token: &ShutdownToken) -> Option<Interval> {
        const POLL_INTERVAL: Duration = Duration::from_secs(120);

        loop {
            let current_interval = match self.current_interval_details().await {
                Err(err) => {
                    error!(
                        "failed to obtain information about the current interval - {err}. Going to retry in {}s",
                        POLL_INTERVAL.as_secs()
                    );
                    tokio::select! {
                        _ = sleep(POLL_INTERVAL) => {
                            continue
                        },
                        _ = shutdown_token.cancelled() => {
                            trace!("wait_until_epoch_end: Received shutdown");
                            break None
                        }
                    }
                }
                Ok(interval) => interval,
            };

            if current_interval.is_current_epoch_over {
                return Some(current_interval.interval);
            } else {
                if let Err(err) = self
                    .window
                    .update(
                        &self.nyxd_client,
                        current_interval.interval.current_epoch_absolute_id(),
                    )
                    .await
                {
                    warn!("failed to refresh the performance contract window: {err:#}");
                }

                let time_left = current_interval.time_until_current_epoch_end();
                info!(
                    "Waiting for epoch change, it should take approximately {}s",
                    time_left.as_secs()
                );
                let wait_time = if time_left < POLL_INTERVAL {
                    // add few seconds to adjust for possible block time drift
                    time_left + Duration::from_secs(10)
                } else {
                    POLL_INTERVAL
                };

                tokio::select! {
                    _ = sleep(wait_time) => {

                    },
                    _ = shutdown_token.cancelled() => {
                        trace!("wait_until_epoch_end: Received shutdown");
                        break None
                    }
                }
            }
        }
    }

    pub(crate) async fn run(&mut self, shutdown_token: ShutdownToken) -> anyhow::Result<()> {
        while !shutdown_token.is_cancelled() {
            let interval_details = match self.wait_until_epoch_end(&shutdown_token).await {
                // received a shutdown
                None => return Ok(()),
                Some(interval) => interval,
            };

            if let Err(err) = self.perform_epoch_operations(interval_details).await {
                error!("failed to perform epoch operations - {err:#}");
                sleep(Duration::from_secs(30)).await;
            }
        }

        Ok(())
    }

    pub(crate) fn start(mut self, shutdown_manager: &ShutdownManager) {
        let shutdown_listener = shutdown_manager.clone_shutdown_token();
        tokio::spawn(async move { self.run(shutdown_listener).await });
    }
}

// before going any further, let's check whether we're allowed to perform rewarding
// (if not, let's blow up sooner rather than later)
pub(crate) async fn ensure_rewarding_permission(
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
