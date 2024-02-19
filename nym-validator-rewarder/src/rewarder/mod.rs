// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::{InsufficientBalance, NymRewarderError};
use crate::rewarder::block_signing::types::EpochSigningResults;
use crate::rewarder::block_signing::EpochSigning;
use crate::rewarder::credential_issuance::types::CredentialIssuanceResults;
use crate::rewarder::credential_issuance::CredentialIssuance;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use futures::future::{FusedFuture, OptionFuture};
use futures::FutureExt;
use nym_task::TaskManager;
use nym_validator_client::nyxd::{AccountId, Coin, Hash};
use nyxd_scraper::NyxdScraper;
use std::ops::Add;
use tokio::pin;
use tokio::time::{interval_at, Instant};
use tracing::{error, info, instrument, warn};

mod block_signing;
mod credential_issuance;
mod epoch;
mod helpers;
mod nyxd_client;
mod storage;
mod tasks;

pub struct RewardingResult {
    pub total_spent: Coin,
    pub rewarding_tx: Hash,
}

pub struct EpochRewards {
    pub epoch: Epoch,
    pub signing: Result<Option<EpochSigningResults>, NymRewarderError>,
    pub credentials: Result<Option<CredentialIssuanceResults>, NymRewarderError>,

    pub total_budget: Coin,
    pub signing_budget: Coin,
    pub credentials_budget: Coin,
}

impl EpochRewards {
    pub fn amounts(&self) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        let mut amounts = Vec::new();

        if let Ok(Some(signing)) = &self.signing {
            for (account, signing_amount) in signing.rewarding_amounts(&self.signing_budget) {
                if signing_amount[0].amount != 0 {
                    amounts.push((account, signing_amount))
                }
            }
        }

        if let Ok(Some(credentials)) = &self.credentials {
            for (account, credential_amount) in
                credentials.rewarding_amounts(&self.credentials_budget)
            {
                if credential_amount[0].amount != 0 {
                    amounts.push((account, credential_amount))
                }
            }
        }

        Ok(amounts)
    }
}

pub fn total_spent(amounts: &[(AccountId, Vec<Coin>)], denom: &str) -> Coin {
    let amount = amounts.iter().map(|(_, amount)| amount[0].amount).sum();
    Coin::new(amount, denom)
}

pub struct Rewarder {
    config: Config,
    current_epoch: Epoch,

    storage: RewarderStorage,
    nyxd_client: NyxdClient,
    epoch_signing: Option<EpochSigning>,
    credential_issuance: Option<CredentialIssuance>,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        let nyxd_client = NyxdClient::new(&config)?;
        let storage = RewarderStorage::init(&config.storage_paths.reward_history).await?;
        let current_epoch = if let Some(last_epoch) = storage.load_last_rewarding_epoch().await? {
            last_epoch.next()
        } else {
            Epoch::first(config.rewarding.epoch_duration)?
        };

        let epoch_signing = if config.block_signing.enabled {
            let whitelist = config.block_signing.whitelist.clone();
            if whitelist.is_empty() {
                return Err(NymRewarderError::EmptyBlockSigningWhitelist);
            }

            if config.block_signing.monitor_only {
                info!("the block signing rewarding is running in monitor only mode");
            }

            let nyxd_scraper = NyxdScraper::new(config.scraper_config()).await?;

            Some(EpochSigning {
                nyxd_scraper,
                nyxd_client: nyxd_client.clone(),
                whitelist,
            })
        } else {
            None
        };

        let credential_issuance = if config.issuance_monitor.enabled {
            let whitelist = &config.issuance_monitor.whitelist;
            if whitelist.is_empty() {
                return Err(NymRewarderError::EmptyCredentialIssuanceWhitelist);
            }

            Some(CredentialIssuance::new(current_epoch, &nyxd_client, whitelist).await?)
        } else {
            None
        };

        if config.issuance_monitor.enabled
            || (config.block_signing.enabled && !config.block_signing.monitor_only)
        {
            let balance = nyxd_client
                .balance(&config.rewarding.epoch_budget.denom)
                .await?;
            let minimum = Coin::new(
                config.rewarding.epoch_budget.amount * 100,
                &config.rewarding.epoch_budget.denom,
            );

            if balance.amount < minimum.amount {
                return Err(NymRewarderError::InsufficientRewarderBalance(Box::new(
                    InsufficientBalance {
                        epoch_budget: config.rewarding.epoch_budget.clone(),
                        balance,
                        minimum,
                    },
                )));
            }
        }

        Ok(Rewarder {
            current_epoch,
            credential_issuance,
            epoch_signing,
            nyxd_client,
            storage,
            config,
        })
    }

    #[instrument(skip(self))]
    async fn calculate_block_signing_rewards(
        &mut self,
    ) -> Result<Option<EpochSigningResults>, NymRewarderError> {
        info!("calculating reward shares");
        if let Some(epoch_signing) = &mut self.epoch_signing {
            Some(
                epoch_signing
                    .get_signed_blocks_results(self.current_epoch)
                    .await,
            )
        } else {
            None
        }
        .transpose()
    }

    #[instrument(skip(self))]
    async fn calculate_credential_rewards(
        &mut self,
    ) -> Result<Option<CredentialIssuanceResults>, NymRewarderError> {
        info!("calculating reward shares");
        if let Some(credential_issuance) = &mut self.credential_issuance {
            Some(
                credential_issuance
                    .get_issued_credentials_results(self.current_epoch)
                    .await,
            )
        } else {
            None
        }
        .transpose()
    }

    async fn determine_epoch_rewards(&mut self) -> EpochRewards {
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

        let signing_rewards = self.calculate_block_signing_rewards().await;
        let credential_rewards = self.calculate_credential_rewards().await;

        EpochRewards {
            epoch: self.current_epoch,
            signing: signing_rewards,
            credentials: credential_rewards,
            total_budget: epoch_budget.clone(),
            signing_budget,
            credentials_budget,
        }
    }

    #[instrument(skip(self))]
    async fn send_rewards(
        &self,
        amounts: Vec<(AccountId, Vec<Coin>)>,
    ) -> Result<Hash, NymRewarderError> {
        if self.config.block_signing.monitor_only {
            info!("skipping sending rewards, monitoring mode only");
            return Ok(Hash::Sha256([0u8; 32]));
        }

        if amounts.is_empty() {
            warn!("no rewards to send");
            return Err(NymRewarderError::NoValidatorsToReward);
        }

        info!("sending rewards");
        self.nyxd_client
            .send_rewards(self.current_epoch, amounts)
            .await
    }

    async fn calculate_and_send_epoch_rewards(
        &mut self,
        rewards: &EpochRewards,
    ) -> Result<RewardingResult, NymRewarderError> {
        let rewarding_amounts = rewards.amounts()?;
        let total_spent = total_spent(
            &rewarding_amounts,
            &self.config.rewarding.epoch_budget.denom,
        );

        let rewarding_tx = self.send_rewards(rewarding_amounts).await?;

        Ok(RewardingResult {
            total_spent,
            rewarding_tx,
        })
    }

    async fn handle_epoch_end(&mut self) {
        info!("handling the epoch end");
        let base_rewards = self.determine_epoch_rewards().await;

        let rewarding_result = self
            .calculate_and_send_epoch_rewards(&base_rewards)
            .await
            .inspect_err(|err| error!("failed to determine and send epoch_rewards: {err}"));

        if let Err(err) = self
            .storage
            .save_rewarding_information(base_rewards, rewarding_result)
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

        if let Some(ref credential_issuance) = self.credential_issuance {
            credential_issuance.start_monitor(
                self.config.issuance_monitor.clone(),
                self.nyxd_client.clone(),
                task_manager.subscribe(),
            );
        }

        let mut scraper_cancellation: OptionFuture<_> =
            if let Some(epoch_signing) = &self.epoch_signing {
                let cancellation_token = epoch_signing.nyxd_scraper.cancel_token();
                epoch_signing.nyxd_scraper.start().await?;
                epoch_signing.nyxd_scraper.wait_for_startup_sync().await;
                Some(Box::pin(async move { cancellation_token.cancelled().await }).fuse())
            } else {
                None
            }
            .into();

        let until_end = self.current_epoch.until_end();

        info!(
            "the initial epoch (id: {}) will finish in {} secs",
            self.current_epoch.id,
            until_end.as_secs()
        );
        let mut epoch_ticker = interval_at(
            Instant::now().add(until_end),
            self.config.rewarding.epoch_duration,
        );

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
                _ = &mut scraper_cancellation, if !scraper_cancellation.is_terminated() => {
                    warn!("the nyxd scraper has been cancelled");
                    break
                }
                _ = epoch_ticker.tick() => self.handle_epoch_end().await
            }
        }

        if let Some(epoch_signing) = self.epoch_signing {
            epoch_signing.nyxd_scraper.stop().await;
        }

        Ok(())
    }
}
