// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use crate::rewarder::block_signing::EpochSigning;
use crate::rewarder::epoch::Epoch;
use nym_task::TaskManager;
use nym_validator_client::nyxd::{AccountId, Coin};
use nym_validator_client::QueryHttpRpcNyxdClient;
use nyxd_scraper::NyxdScraper;
use std::ops::Add;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::pin;
use tokio::time::{interval_at, Instant};
use tracing::{debug, error, info, instrument};

mod block_signing;
mod epoch;
mod helpers;
mod nyxd_client;
mod tasks;

pub struct Rewarder {
    config: Config,
    current_epoch: Epoch,

    epoch_signing: EpochSigning,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        let nyxd_scraper = NyxdScraper::new(config.scraper_config()).await?;
        let rpc_client = QueryHttpRpcNyxdClient::connect(
            config.rpc_client_config(),
            config.base.upstream_nyxd.as_str(),
        )?;

        Ok(Rewarder {
            current_epoch: Epoch::first()?,
            config,
            epoch_signing: EpochSigning {
                nyxd_scraper,
                rpc_client,
            },
        })
    }

    #[instrument(skip(self,budget), fields(budget = %budget))]
    async fn calculate_block_signing_rewards(
        &mut self,
        budget: Coin,
    ) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        info!("calculating reward shares");
        let signed = self
            .epoch_signing
            .get_signed_blocks_results(self.current_epoch)
            .await?;

        debug!("details: {signed:?}");

        Ok(signed.rewarding_amounts(&budget))
    }

    #[instrument(skip(self,budget), fields(budget = %budget))]
    async fn calculate_credential_rewards(
        &mut self,
        budget: Coin,
    ) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        info!("calculating reward shares");
        Ok(Vec::new())
    }

    async fn determine_epoch_rewards(
        &mut self,
    ) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        let epoch_budget = &self.config.rewarding.epoch_budget;
        let denom = &epoch_budget.denom;
        let signing_budget = Coin::new(
            (self.config.rewarding.ratios.block_signing * epoch_budget.amount as f64) as u128,
            denom,
        );
        let credential_budget = Coin::new(
            (self.config.rewarding.ratios.credential_issuance * epoch_budget.amount as f64) as u128,
            denom,
        );

        let signing_rewards = self.calculate_block_signing_rewards(signing_budget).await?;
        let mut credential_rewards = self.calculate_credential_rewards(credential_budget).await?;

        let mut rewards = signing_rewards;
        rewards.append(&mut credential_rewards);
        Ok(rewards)
    }

    async fn send_rewards(
        &self,
        amounts: Vec<(AccountId, Vec<Coin>)>,
    ) -> Result<(), NymRewarderError> {
        Ok(())
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

        if let Err(err) = self.send_rewards(rewards).await {
            error!("failed to send epoch rewards: {err}");
            return;
        };

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

        /*
           task 1:
           on timer:
               - go to DKG contract
               - get all coconut signers
               - for each of them get the info, verify, etc

           task 2:
           on timer (or maybe per block?):
               - query abci endpoint for VP
               - also maybe missed blocks, etc

        */

        todo!()
    }
}
