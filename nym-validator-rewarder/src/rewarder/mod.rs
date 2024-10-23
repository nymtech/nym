// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::{InsufficientBalance, NymRewarderError};
use crate::rewarder::block_signing::types::EpochSigningResults;
use crate::rewarder::block_signing::EpochSigning;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use crate::rewarder::ticketbook_issuance::helpers::end_of_day_ticker;
use crate::rewarder::ticketbook_issuance::types::TicketbookIssuanceResults;
use crate::rewarder::ticketbook_issuance::TicketbookIssuance;
use futures::future::{Fuse, FusedFuture, OptionFuture};
use futures::FutureExt;
use nym_ecash_time::{ecash_today, ecash_today_date, EcashTime};
use nym_task::TaskManager;
use nym_validator_client::nyxd::{AccountId, Coin, Hash};
use nyxd_scraper::NyxdScraper;
use std::future::Future;
use std::ops::Add;
use std::pin::Pin;
use time::Date;
use tokio::pin;
use tokio::time::{interval_at, Instant};
use tracing::{error, info, instrument, warn};

mod block_signing;
mod epoch;
mod helpers;
mod nyxd_client;
mod storage;
mod tasks;
mod ticketbook_issuance;

pub(crate) use crate::rewarder::epoch::Epoch;

pub struct RewardingResult {
    pub total_spent: Coin,
    pub rewarding_tx: Option<Hash>,
}

pub struct ExtractedRewardingResults {
    pub rewarding_tx: Option<String>,
    pub total_spent: Coin,
    pub rewarding_err: Option<String>,
    pub monitor_only: bool,
}

pub fn extract_rewarding_results(
    results: Result<RewardingResult, NymRewarderError>,
    rewarding_denom: &str,
) -> ExtractedRewardingResults {
    match results {
        Ok(res) => match res.rewarding_tx {
            None => ExtractedRewardingResults {
                rewarding_tx: None,
                total_spent: Coin::new(0, rewarding_denom),
                rewarding_err: None,
                monitor_only: true,
            },
            Some(hash) => ExtractedRewardingResults {
                rewarding_tx: Some(hash.to_string()),
                total_spent: res.total_spent,
                rewarding_err: None,
                monitor_only: false,
            },
        },
        Err(err) => ExtractedRewardingResults {
            rewarding_tx: Some(err.to_string()),
            total_spent: Coin::new(0, rewarding_denom),
            rewarding_err: None,
            monitor_only: false,
        },
    }
}

#[deprecated]
pub struct EpochRewards {
    pub epoch: Epoch,
    pub signing: Result<Option<EpochSigningResults>, NymRewarderError>,
    pub credentials: Result<Option<TicketbookIssuanceResults>, NymRewarderError>,

    #[deprecated]
    pub total_budget: Coin,
    pub signing_budget: Coin,
    pub credentials_budget: Coin,
}

pub struct BlockSigningDetails {
    pub epoch: Epoch,
    pub results: Option<Result<EpochSigningResults, NymRewarderError>>,
    pub budget: Coin,
}

impl BlockSigningDetails {
    pub fn rewarding_amounts(&self) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        let mut amounts = Vec::new();

        match &self.results {
            Some(Ok(signing)) => {
                for (account, signing_amount) in signing.rewarding_amounts(&self.budget) {
                    if signing_amount[0].amount != 0 {
                        amounts.push((account, signing_amount))
                    }
                }
            }
            Some(Err(err)) => error!("failed to determine rewards for block signing: {err}"),
            _ => (),
        }

        Ok(amounts)
    }
}

pub struct TicketbookIssuanceDetails {
    pub date: Date,
    pub results: Option<Result<TicketbookIssuanceResults, NymRewarderError>>,
    pub budget: Coin,
}

impl EpochRewards {
    pub fn amounts(&self) -> Result<Vec<(AccountId, Vec<Coin>)>, NymRewarderError> {
        let mut amounts = Vec::new();

        match &self.signing {
            Ok(Some(signing)) => {
                for (account, signing_amount) in signing.rewarding_amounts(&self.signing_budget) {
                    if signing_amount[0].amount != 0 {
                        amounts.push((account, signing_amount))
                    }
                }
            }
            Err(err) => error!("failed to determine rewards for block signing: {err}"),
            _ => (),
        }

        match &self.credentials {
            Ok(Some(credentials)) => {
                for (account, credential_amount) in
                    credentials.rewarding_amounts(&self.credentials_budget)
                {
                    if credential_amount[0].amount != 0 {
                        amounts.push((account, credential_amount))
                    }
                }
            }
            Err(err) => error!("failed to determine rewards for credential issuance: {err}"),
            _ => (),
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
    current_block_signing_epoch: Epoch,

    storage: RewarderStorage,
    nyxd_client: NyxdClient,
    epoch_signing: Option<EpochSigning>,
    ticketbook_issuance: Option<TicketbookIssuance>,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        // no point in starting up if both modules are disabled
        if !config.block_signing.enabled && !config.ticketbook_issuance.enabled {
            return Err(NymRewarderError::RewardingModulesDisabled);
        }

        let nyxd_client = NyxdClient::new(&config)?;
        let storage = RewarderStorage::init(&config.storage_paths.reward_history).await?;
        let current_block_signing_epoch =
            if let Some(last_epoch) = storage.load_last_block_signing_rewarding_epoch().await? {
                last_epoch.next()
            } else {
                Epoch::first(config.block_signing.epoch_duration)?
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

        let credential_issuance = if config.ticketbook_issuance.enabled {
            let whitelist = &config.ticketbook_issuance.whitelist;
            if whitelist.is_empty() {
                return Err(NymRewarderError::EmptyTicketbookIssuanceWhitelist);
            }

            Some(
                TicketbookIssuance::new(
                    current_block_signing_epoch,
                    storage.clone(),
                    &nyxd_client,
                    whitelist,
                )
                .await?,
            )
        } else {
            None
        };

        if config.will_attempt_to_send_rewards() {
            let balance = nyxd_client
                .balance(&config.rewarding.daily_budget.denom)
                .await?;
            let minimum = Coin::new(
                config.rewarding.daily_budget.amount * 7,
                &config.rewarding.daily_budget.denom,
            );

            if balance.amount < minimum.amount {
                return Err(NymRewarderError::InsufficientRewarderBalance(Box::new(
                    InsufficientBalance {
                        daily_budget: config.rewarding.daily_budget.clone(),
                        balance,
                        minimum,
                    },
                )));
            }
        }

        Ok(Rewarder {
            ticketbook_issuance: credential_issuance,
            epoch_signing,
            nyxd_client,
            storage,
            config,
            current_block_signing_epoch,
        })
    }

    #[instrument(skip(self))]
    async fn block_signing_details(&mut self) -> BlockSigningDetails {
        info!("calculating reward shares");
        let results = if let Some(epoch_signing) = &mut self.epoch_signing {
            Some(
                epoch_signing
                    .get_signed_blocks_results(self.current_block_signing_epoch)
                    .await,
            )
        } else {
            None
        };
        BlockSigningDetails {
            epoch: self.current_block_signing_epoch,
            results,
            budget: self.config.block_signing_epoch_budget(),
        }
    }

    #[instrument(skip(self))]
    async fn calculate_credential_rewards(
        &mut self,
    ) -> Result<Option<TicketbookIssuanceResults>, NymRewarderError> {
        info!("calculating reward shares");
        if let Some(credential_issuance) = &mut self.ticketbook_issuance {
            Some(
                credential_issuance
                    .get_issued_credentials_results(self.current_block_signing_epoch)
                    .await,
            )
        } else {
            None
        }
        .transpose()
    }

    #[instrument(skip(self))]
    async fn send_block_signing_rewards(
        &self,
        amounts: Vec<(AccountId, Vec<Coin>)>,
    ) -> Result<Option<Hash>, NymRewarderError> {
        if self.config.block_signing.monitor_only {
            info!("skipping sending rewards, monitoring mode only");
            return Ok(None);
        }

        if amounts.is_empty() {
            warn!("no rewards to send");
            return Err(NymRewarderError::NoValidatorsToReward);
        }

        info!("sending rewards");
        warn!("here be tx sending");
        Ok(Some(Hash::Sha256([0u8; 32])))
        // self.nyxd_client
        //     .send_rewards(self.current_block_signing_epoch, amounts)
        //     .await
        //     .map(Some)
    }

    async fn calculate_and_send_block_signing_epoch_rewards(
        &mut self,
        signed_blocks: &BlockSigningDetails,
    ) -> Result<RewardingResult, NymRewarderError> {
        let rewarding_amounts = signed_blocks.rewarding_amounts()?;
        let total_spent = total_spent(
            &rewarding_amounts,
            &self.config.rewarding.daily_budget.denom,
        );

        let rewarding_tx = self.send_block_signing_rewards(rewarding_amounts).await?;

        Ok(RewardingResult {
            total_spent,
            rewarding_tx,
        })
    }

    async fn calculate_and_send_ticketbook_issuance_rewards(
        &mut self,
        issued_ticketbooks: &TicketbookIssuanceDetails,
    ) -> Result<RewardingResult, NymRewarderError> {
        todo!()
    }

    #[deprecated]
    async fn determine_epoch_rewards(&mut self) -> EpochRewards {
        todo!()
        // let epoch_budget = self.config.rewarding.daily_budget.clone();
        // let signing_budget = self.config.block_signing_epoch_budget();
        // let credentials_budget = self.config.ticketbook_issuance_daily_budget();
        //
        // let signing_rewards = self.calculate_block_signing_rewards().await;
        // let credential_rewards = self.calculate_credential_rewards().await;
        //
        // EpochRewards {
        //     epoch: self.current_block_signing_epoch,
        //     signing: signing_rewards,
        //     credentials: credential_rewards,
        //     total_budget: epoch_budget.clone(),
        //     signing_budget,
        //     credentials_budget,
        // }
    }

    async fn handle_block_signing_epoch_end(&mut self) {
        info!("handling the block signing epoch end");

        let details = self.block_signing_details().await;

        let rewarding_result = self
            .calculate_and_send_block_signing_epoch_rewards(&details)
            .await
            .inspect_err(|err| error!("failed to determine and send block signing rewards: {err}"));

        if let Err(err) = self
            .storage
            .save_block_signing_rewarding_information(details, rewarding_result)
            .await
        {
            error!("failed to persist rewarding information: {err}")
        }

        self.current_block_signing_epoch = self.current_block_signing_epoch.next();
    }

    #[instrument(skip(self))]
    async fn ticketbook_issuance_details(&mut self, yesterday: Date) -> TicketbookIssuanceDetails {
        info!("calculating reward shares");
        let results = if let Some(ticketbook_issuance) = &mut self.ticketbook_issuance {
            Some(
                ticketbook_issuance
                    .get_issued_ticketbooks_results(yesterday)
                    .await,
            )
        } else {
            None
        };
        TicketbookIssuanceDetails {
            date: yesterday,
            results,
            budget: self.config.ticketbook_issuance_daily_budget(),
        }
    }

    async fn handle_next_ticketbook_issuance_day(&mut self) {
        // sanity check to make sure it's actually after midnight
        let today = ecash_today();
        assert_eq!(today.hour(), 0);

        // safety: this software is not run in 1 AD...
        #[allow(clippy::unwrap_used)]
        let yesterday = today.ecash_date().previous_day().unwrap();

        let details = self.ticketbook_issuance_details(yesterday).await;

        let rewarding_result = self
            .calculate_and_send_ticketbook_issuance_rewards(&details)
            .await
            .inspect_err(|err| {
                error!("failed to determine and send ticketbook issuance rewards: {err}")
            });

        todo!()
    }

    async fn ensure_has_epoch_blocks(&self) -> Result<(), NymRewarderError> {
        // make sure we at least have a single block processed within the epoch
        let epoch_start = self.current_block_signing_epoch.start_time;
        let epoch_end = self.current_block_signing_epoch.end_time;

        if let Some(epoch_signing) = &self.epoch_signing {
            if epoch_signing
                .nyxd_scraper
                .storage
                .get_first_block_height_after(epoch_start)
                .await?
                .is_none()
            {
                return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                    epoch: self.current_block_signing_epoch,
                });
            }

            if epoch_signing
                .nyxd_scraper
                .storage
                .get_last_block_height_before(epoch_end)
                .await?
                .is_none()
            {
                return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                    epoch: self.current_block_signing_epoch,
                });
            }
        }

        Ok(())
    }

    async fn startup_resync(&mut self) -> Result<(), NymRewarderError> {
        // no sync required
        if !self.current_block_signing_epoch.has_finished() {
            return Ok(());
        }

        info!("attempting to distribute missed rewards");
        while self.current_block_signing_epoch.has_finished() {
            info!("processing epoch {}", self.current_block_signing_epoch);
            self.ensure_has_epoch_blocks().await?;

            // we need to perform rewarding from the 'current' epoch until the actual current epoch
            self.handle_block_signing_epoch_end().await
        }

        Ok(())
    }

    async fn setup_tasks(
        &self,
        task_manager: &mut TaskManager,
    ) -> Result<impl FusedFuture, NymRewarderError> {
        if let Some(ref credential_issuance) = self.ticketbook_issuance {
            credential_issuance.start_monitor(
                self.config.ticketbook_issuance.clone(),
                self.nyxd_client.clone(),
                task_manager.subscribe_named("credential-monitor"),
            );
        }

        let scraper_cancellation: OptionFuture<_> =
            if let Some(epoch_signing) = &self.epoch_signing {
                let cancellation_token = epoch_signing.nyxd_scraper.cancel_token();
                epoch_signing.nyxd_scraper.start().await?;
                epoch_signing.nyxd_scraper.wait_for_startup_sync().await;
                Some(Box::pin(async move { cancellation_token.cancelled_owned().await }).fuse())
            } else {
                None
            }
            .into();
        Ok(scraper_cancellation)
    }

    async fn main_loop(
        mut self,
        mut task_manager: TaskManager,
        mut scraper_cancellation: impl FusedFuture + Unpin,
    ) {
        let until_end = self.current_block_signing_epoch.until_end();
        info!(
            "the initial block signing epoch (id: {}) will finish on {} ({} secs remaining)",
            self.current_block_signing_epoch.id,
            self.current_block_signing_epoch.end_rfc3339(),
            until_end.as_secs()
        );
        // runs as often as specified in the config. by default every 1h
        let mut block_signing_epoch_ticker = interval_at(
            Instant::now().add(until_end),
            self.config.block_signing.epoch_duration,
        );

        // runs daily
        let mut ticketbook_issuance_ticker = end_of_day_ticker();

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
                _ = block_signing_epoch_ticker.tick() => self.handle_block_signing_epoch_end().await,
                _ = ticketbook_issuance_ticker.tick() => self.handle_next_ticketbook_issuance_day().await,
            }
        }

        if let Some(epoch_signing) = self.epoch_signing {
            epoch_signing.nyxd_scraper.stop().await;
        }
    }

    pub async fn run(mut self) -> Result<(), NymRewarderError> {
        info!("Starting nym validators rewarder");

        // setup shutdowns
        let mut task_manager = TaskManager::new(5);
        let scraper_cancellation = self.setup_tasks(&mut task_manager).await?;

        if let Err(err) = self.startup_resync().await {
            error!("failed to perform startup sync: {err}");
            error!("if the failure was due to insufficient number of blocks, your course of action is as follows:");
            error!("(ideally it would have been automatically resolved in this very method, but that'd require some serious refactoring)");
            error!(
                "1. determine height of the first block of the epoch (doesn't have to be exact)"
            );
            error!("2. run the following subcommand of the rewarder: `nym-validator-rewarder process-until --start-height=$STARTING_BLOCK");
            error!("3. !!IMPORTANT!! go to config.toml and temporarily disable block pruning, i.e. `pruning.strategy=nothing`");
            error!("4. restart nym-validator-rewarder as normal until it sends missing rewards");
            error!("5. re-enable pruning and restart the nym-validator rewarder");
            return Err(err);
        }

        self.main_loop(task_manager, scraper_cancellation).await;

        Ok(())
    }
}
