// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract_window::WindowData;
use crate::network_view::{DirectoryAnchor, NetworkView};
use anyhow::{Context, bail};
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_directory_client::cadence::SnapshotCadence;
use nym_directory_client::key::digest_item_key;
use nym_mixnet_contract_common::{CurrentIntervalResponse, Interval};
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

    directory_anchor: DirectoryAnchor,

    /// How many blocks a cadence boundary must lie below the tip before the view is produced
    /// at it.
    settle_lag: u64,

    /// Where selection takes its candidates from, produced by the loop at each newly settled
    /// cadence boundary.
    network_view: NetworkView,
}

impl EpochAdvancer {
    /// Loads the window and produces the first network view, failing when the performance
    /// contract has never received a submission or when the view cannot be produced.
    pub(crate) async fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        settle_lag: u64,
    ) -> anyhow::Result<Self> {
        let directory_contract = nyxd_client
            .directory_contract_address()
            .cloned()
            .context("the network details carry no directory contract address")?;
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

        let cadence = SnapshotCadence::read(&nyxd_client)
            .await
            .context("failed to read the snapshot cadence")?;
        let height = cadence
            .newest_settled_height(settle_lag)
            .context("no cadence boundary has settled to produce the network view at")?;
        let network_view = NetworkView::produce(&directory_anchor, &nyxd_client, height)
            .await
            .context("failed to produce the network view")?;

        Ok(EpochAdvancer {
            nyxd_client,
            window,
            directory_anchor,
            settle_lag,
            network_view,
        })
    }

    /// Produces the view at the newest settled cadence boundary, when it is newer than the one
    /// held; on failure the held view stays.
    async fn refresh_network_view(&mut self) -> anyhow::Result<()> {
        let cadence = SnapshotCadence::read(&self.nyxd_client)
            .await
            .context("failed to read the snapshot cadence")?;
        let Some(height) = cadence.newest_settled_height(self.settle_lag) else {
            return Ok(());
        };
        if height > self.network_view.height {
            self.network_view =
                NetworkView::produce(&self.directory_anchor, &self.nyxd_client, height).await?;
        }
        Ok(())
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

        // selection reads the network view as held, without a directory read at epoch time
        self.update_rewarded_set_and_advance_epoch().await?;

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
                if let Err(err) = self.refresh_network_view().await {
                    warn!("failed to refresh the network view: {err:#}");
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
