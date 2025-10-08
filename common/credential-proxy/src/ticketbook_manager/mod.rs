// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::deposits_buffer::DepositsBuffer;
use crate::error::CredentialProxyError;
use crate::quorum_checker::QuorumStateChecker;
use crate::shared_state::CredentialProxyState;
use crate::shared_state::ecash_state::EcashState;
use crate::shared_state::nyxd_client::ChainClient;
use crate::shared_state::required_deposit_cache::RequiredDepositCache;
use crate::storage::CredentialProxyStorage;
use crate::storage::pruner::StoragePruner;
use crate::webhook::ZkNymWebhook;
use nym_credentials::ecash::utils::ecash_default_expiration_date;
use nym_validator_client::nym_api::EpochId;
use std::future::Future;
use std::time::Duration;
use time::Date;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

mod shares_handlers;
pub mod ticketbook_handlers;
pub mod wallet_shares;

#[derive(Clone, Default)]
pub struct ShutdownTracker {
    pub shutdown_token: CancellationToken,
    pub tracker: TaskTracker,
}

#[derive(Clone)]
pub struct TicketbookManager {
    pub(crate) state: CredentialProxyState,
    pub(crate) webhook: ZkNymWebhook,
    pub(crate) shutdown_tracker: ShutdownTracker,
}

impl TicketbookManager {
    pub async fn new(
        build_sha: &'static str,
        quorum_check_interval: Duration,
        deposits_buffer_size: usize,
        max_concurrent_deposits: usize,
        storage: CredentialProxyStorage,
        mnemonic: bip39::Mnemonic,
        webhook: ZkNymWebhook,
    ) -> Result<Self, CredentialProxyError> {
        let chain_client = ChainClient::new(mnemonic)?;
        let shutdown_tracker = ShutdownTracker::default();

        let quorum_state_checker = QuorumStateChecker::new(
            chain_client.clone(),
            quorum_check_interval,
            shutdown_tracker.shutdown_token.clone(),
        )
        .await?;

        let required_deposit_cache = RequiredDepositCache::default();

        let deposits_buffer = DepositsBuffer::new(
            storage.clone(),
            chain_client.clone(),
            required_deposit_cache.clone(),
            build_sha,
            deposits_buffer_size,
            max_concurrent_deposits,
            shutdown_tracker.shutdown_token.clone(),
        )
        .await?;

        let storage_pruner =
            StoragePruner::new(shutdown_tracker.shutdown_token.clone(), storage.clone());

        let this = TicketbookManager {
            state: CredentialProxyState::new(
                storage.clone(),
                chain_client,
                deposits_buffer,
                EcashState::new(
                    required_deposit_cache,
                    quorum_state_checker.quorum_state_ref(),
                ),
            ),
            webhook,
            shutdown_tracker,
        };

        // since this is startup,
        // might as well do all the needed network queries to establish needed global signatures
        // if we don't already have them
        this.build_initial_cache().await?;

        // spawn the background tasks
        this.try_spawn_in_background(quorum_state_checker.run_forever());
        this.try_spawn_in_background(storage_pruner.run_forever());

        Ok(this)
    }

    async fn build_initial_cache(&self) -> Result<(), CredentialProxyError> {
        let default_expiration = ecash_default_expiration_date();

        let epoch_id = self.state.current_epoch_id().await?;
        let _ = self.state.deposit_amount().await?;
        let _ = self.state.master_verification_key(Some(epoch_id)).await?;
        let _ = self.state.ecash_threshold(epoch_id).await?;
        let _ = self.state.ecash_clients(epoch_id).await?;
        let _ = self
            .state
            .master_coin_index_signatures(Some(epoch_id))
            .await?;
        let _ = self
            .state
            .master_expiration_date_signatures(epoch_id, default_expiration)
            .await?;

        Ok(())
    }

    pub async fn cancel_and_wait(&self) {
        self.shutdown_tracker.shutdown_token.cancel();
        self.state.deposits_buffer().wait_for_shutdown().await;
        self.shutdown_tracker.tracker.wait().await
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_tracker.shutdown_token.clone()
    }

    /// Ensure the required global data for the specified epoch and expiration date exists in our cache (and storage)
    async fn ensure_global_data_cached(
        &self,
        epoch: EpochId,
        expiration_date: Date,
    ) -> Result<(), CredentialProxyError> {
        let _ = self.state.master_verification_key(Some(epoch)).await?;
        let _ = self.state.master_coin_index_signatures(Some(epoch)).await?;
        let _ = self
            .state
            .master_expiration_date_signatures(epoch, expiration_date)
            .await?;
        Ok(())
    }

    pub fn try_spawn_in_background<F>(&self, task: F) -> Option<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // don't spawn new task if we've received cancellation token
        if self.shutdown_tracker.shutdown_token.is_cancelled() {
            None
        } else {
            self.shutdown_tracker.tracker.reopen();
            // TODO: later use a task queue since most requests will be blocked waiting on chain permit anyway
            let join_handle = self.shutdown_tracker.tracker.spawn(task);
            self.shutdown_tracker.tracker.close();
            Some(join_handle)
        }
    }
}
