// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use crate::rewarder::block_signing::types::EpochSigningResults;
use crate::rewarder::block_signing::EpochSigning;
use crate::rewarder::credential_issuance::types::CredentialIssuanceResults;
use crate::rewarder::credential_issuance::CredentialIssuance;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use nym_task::TaskManager;
use nym_validator_client::nyxd::{AccountId, Coin, Hash};
use nyxd_scraper::NyxdScraper;
use std::ops::Add;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::pin;
use tokio::time::{interval_at, Instant};
use tracing::{error, info, instrument};

mod block_signing;
mod credential_issuance;
mod epoch;
mod helpers;
mod nyxd_client;
mod storage;
mod tasks;

pub struct EpochRewards {
    pub epoch: Epoch,
    pub signing: EpochSigningResults,
    pub credentials: CredentialIssuanceResults,

    pub total_budget: Coin,
    pub signing_budget: Coin,
    pub credentials_budget: Coin,
}

impl EpochRewards {
    pub fn amounts(&self) -> Vec<(AccountId, Vec<Coin>)> {
        let signing = self.signing.rewarding_amounts(&self.signing_budget);
        let mut credentials = Vec::new();

        let mut amounts = signing;
        amounts.append(&mut credentials);

        amounts
    }
}

pub struct Rewarder {
    config: Config,
    current_epoch: Epoch,

    storage: RewarderStorage,
    nyxd_client: NyxdClient,
    epoch_signing: EpochSigning,
    credential_issuance: CredentialIssuance,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        let nyxd_scraper = NyxdScraper::new(config.scraper_config()).await?;
        let nyxd_client = NyxdClient::new(&config);
        let storage = RewarderStorage::init(&config.storage_paths.reward_history).await?;

        Ok(Rewarder {
            current_epoch: Epoch::first()?,
            config,
            epoch_signing: EpochSigning {
                nyxd_scraper,
                nyxd_client: nyxd_client.clone(),
            },
            nyxd_client,
            storage,
            credential_issuance: CredentialIssuance {},
        })
    }

    #[instrument(skip(self))]
    async fn calculate_block_signing_rewards(
        &mut self,
    ) -> Result<EpochSigningResults, NymRewarderError> {
        info!("calculating reward shares");
        self.epoch_signing
            .get_signed_blocks_results(self.current_epoch)
            .await
    }

    #[instrument(skip(self))]
    async fn calculate_credential_rewards(
        &mut self,
    ) -> Result<CredentialIssuanceResults, NymRewarderError> {
        info!("calculating reward shares");
        self.credential_issuance
            .get_signed_blocks_results(self.current_epoch)
            .await
    }

    async fn determine_epoch_rewards(&mut self) -> Result<EpochRewards, NymRewarderError> {
        let epoch_budget = self.config.rewarding.epoch_budget.clone();
        let denom = &epoch_budget.denom;
        let signing_budget = Coin::new(
            (self.config.rewarding.ratios.block_signing * epoch_budget.amount as f64) as u128,
            denom,
        );
        let credentials_budget = Coin::new(
            (self.config.rewarding.ratios.credential_issuance * epoch_budget.amount as f64) as u128,
            denom,
        );

        let signing_rewards = self.calculate_block_signing_rewards().await?;
        let credential_rewards = self.calculate_credential_rewards().await?;

        Ok(EpochRewards {
            epoch: self.current_epoch,
            signing: signing_rewards,
            credentials: credential_rewards,
            total_budget: epoch_budget.clone(),
            signing_budget,
            credentials_budget,
        })
    }

    async fn send_rewards(
        &self,
        amounts: Vec<(AccountId, Vec<Coin>)>,
    ) -> Result<Hash, NymRewarderError> {
        let address = self.nyxd_client.address().await;
        info!("here we ({address}) will be sending the following rewards:");
        for (target, amount) in amounts {
            info!("{amount:?} to {target}")
        }

        Ok(Hash::Sha256([0u8; 32]))
    }

    async fn handle_epoch_end(&mut self) {
        info!("handling the epoch end");

        let rewards = match self.determine_epoch_rewards().await {
            Ok(rewards) => rewards,
            Err(err) => {
                error!("failed to determine epoch rewards: {err}");
                return;
            }
        };

        let rewarding_result = self.send_rewards(rewards.amounts()).await;
        if let Err(err) = self
            .storage
            .save_rewarding_information(rewards, rewarding_result)
            .await
        {
            error!("failed to persist rewarding information: {err}")
        }

        self.current_epoch = self.current_epoch.next();
    }

    pub async fn run(mut self) -> Result<(), NymRewarderError> {
        info!("Starting nym validators rewarder");

        // setup shutdowns
        let mut task_manager = TaskManager::new(5);

        self.epoch_signing.nyxd_scraper.start().await?;

        // rewarding epochs last from :00 to :00
        self.current_epoch.end = OffsetDateTime::now_utc();
        self.current_epoch.start = self.current_epoch.end - Duration::from_secs(60 * 60);

        let until_end = self.current_epoch.until_end();

        info!(
            "the first epoch will finish in {} secs",
            until_end.as_secs()
        );
        let mut epoch_ticker = interval_at(Instant::now().add(until_end), Epoch::LENGTH);

        let shutdown_future = task_manager.catch_interrupt();
        pin!(shutdown_future);

        loop {
            tokio::select! {
                biased;
                interrupt_res = &mut shutdown_future => {
                    info!("received interrupt");
                    if let Err(err) = interrupt_res {
                        error!("runtime interrupt failure: {err}")
                    }
                    break;
                }
                _ = epoch_ticker.tick() => self.handle_epoch_end().await
            }
        }

        self.epoch_signing.nyxd_scraper.stop().await;

        Ok(())
    }
}
