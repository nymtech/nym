// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::rewarder::{extract_rewarding_results, BlockSigningDetails, TicketbookIssuanceDetails};
use crate::{
    error::NymRewarderError,
    rewarder::{epoch::Epoch, storage::manager::StorageManager, RewardingResult},
};
use nym_contracts_common::types::NaiveFloat;
use sqlx::ConnectOptions;
use std::{fmt::Debug, path::Path};
use time::{Date, OffsetDateTime};
use tracing::{error, info, instrument};

mod manager;

#[derive(Clone)]
pub struct RewarderStorage {
    pub(crate) manager: StorageManager,
}

impl RewarderStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(database_path: P) -> Result<Self, NymRewarderError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        // TODO: do we want auto_vacuum ?

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let manager = StorageManager { connection_pool };
        let storage = RewarderStorage { manager };

        Ok(storage)
    }

    pub(crate) async fn load_last_block_signing_rewarding_epoch(
        &self,
    ) -> Result<Option<Epoch>, NymRewarderError> {
        Ok(self
            .manager
            .load_last_block_signing_rewarding_epoch()
            .await?)
    }

    pub(crate) async fn load_last_ticketbook_issuance_expiration_date(
        &self,
    ) -> Result<Option<Date>, NymRewarderError> {
        Ok(self
            .manager
            .load_last_ticketbook_issuance_expiration_date()
            .await?)
    }

    pub(crate) async fn load_banned_ticketbook_issuers(
        &self,
    ) -> Result<Vec<String>, NymRewarderError> {
        Ok(self.manager.load_banned_ticketbook_issuers().await?)
    }

    pub(crate) async fn save_block_signing_rewarding_information(
        &self,
        details: BlockSigningDetails,
        rewarding_result: Result<RewardingResult, NymRewarderError>,
    ) -> Result<(), NymRewarderError> {
        info!("persisting block signing reward details");
        let denom = &details.budget.denom;

        let extracted_results = extract_rewarding_results(rewarding_result, denom);
        let epoch_id = details.epoch.id;

        // general epoch info
        self.manager
            .insert_block_signing_rewarding_epoch(
                details.epoch,
                details.budget.to_string(),
                details.results.is_none(),
            )
            .await?;

        let Some(results) = details.results else {
            // no information to save as it's disabled
            return Ok(());
        };

        let results = match results {
            Ok(results) => results,
            Err(err) => {
                // we didn't manage to calculate or send rewards for anyone.
                // save the failure information and continue
                if extracted_results.total_spent.amount != 0 {
                    error!(
                        "BROKEN INVARIANT: failed to send rewards yet we spent a non-zero amount!"
                    );
                    error!("the rewards weren't sent because of: {err}");
                    return Ok(());
                }

                self.manager
                    .insert_block_signing_rewarding_details(
                        epoch_id,
                        -1,
                        -1,
                        extracted_results.total_spent.to_string(),
                        None,
                        Some(err.to_string()),
                        extracted_results.monitor_only,
                    )
                    .await?;
                return Ok(());
            }
        };

        self.manager
            .insert_block_signing_rewarding_details(
                epoch_id,
                results.total_voting_power_at_epoch_start,
                results.blocks,
                extracted_results.total_spent.to_string(),
                extracted_results.rewarding_tx,
                extracted_results.rewarding_err,
                extracted_results.monitor_only,
            )
            .await?;

        for validator in results.validators {
            let reward_amount = validator.reward_amount(&details.budget).to_string();
            self.manager
                .insert_block_signing_reward(
                    epoch_id,
                    validator.validator.consensus_address,
                    validator.operator_account.to_string(),
                    validator.whitelisted,
                    reward_amount,
                    validator.voting_power_at_epoch_start,
                    validator.voting_power_ratio.to_string(),
                    validator.signed_blocks,
                    validator.ratio_signed.to_string(),
                )
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn save_ticketbook_issuance_rewarding_information(
        &self,
        details: TicketbookIssuanceDetails,
        rewarding_result: Result<RewardingResult, NymRewarderError>,
    ) -> Result<(), NymRewarderError> {
        info!("persisting ticketbook issuance reward details");
        let denom = &details.total_budget.denom;

        let extracted_results = extract_rewarding_results(rewarding_result, denom);
        let expiration_date = details.expiration_date;

        // general info for the epoch as marked by the ticketbook expiration date
        self.manager
            .insert_ticketbook_issuance_epoch(
                details.expiration_date,
                details.total_budget.to_string(),
                details.whitelist_size as u32,
                details.per_operator_budget.to_string(),
                details.results.is_none(),
            )
            .await?;

        let Some(results) = details.results else {
            // no information to save as it's disabled
            return Ok(());
        };

        let results = match results {
            Ok(results) => results,
            Err(err) => {
                // we didn't manage to calculate or send rewards for anyone.
                // save the failure information and continue
                if extracted_results.total_spent.amount != 0 {
                    error!(
                        "BROKEN INVARIANT: failed to send rewards yet we spent a non-zero amount!"
                    );
                    error!("the rewards weren't sent because of: {err}");
                    return Ok(());
                }

                self.manager
                    .insert_ticketbook_issuance_rewarding_details(
                        expiration_date,
                        -1,
                        extracted_results.total_spent.to_string(),
                        None,
                        Some(err.to_string()),
                        extracted_results.monitor_only,
                    )
                    .await?;
                return Ok(());
            }
        };

        self.manager
            .insert_ticketbook_issuance_rewarding_details(
                expiration_date,
                results.approximate_deposits as i64,
                extracted_results.total_spent.to_string(),
                extracted_results.rewarding_tx,
                extracted_results.rewarding_err,
                extracted_results.monitor_only,
            )
            .await?;

        for issuer in results.api_runners {
            let reward_amount = issuer
                .reward_amount(&details.per_operator_budget)
                .to_string();
            self.manager
                .insert_ticketbook_issuance_reward(
                    expiration_date,
                    issuer.api_runner.clone(),
                    issuer.runner_account.to_string(),
                    issuer.whitelisted,
                    issuer.pre_banned || issuer.issuer_ban.is_some(),
                    reward_amount,
                    issuer.issued_ticketbooks,
                    issuer.issued_ratio.naive_to_f64() as f32,
                    issuer.skipped_verification,
                    issuer.subsample_size,
                )
                .await?;

            if let Some(cheating) = issuer.issuer_ban {
                self.manager
                    .insert_banned_ticketbook_issuer(
                        issuer.api_runner,
                        issuer.runner_account.to_string(),
                        OffsetDateTime::now_utc(),
                        expiration_date,
                        cheating.reason,
                        cheating.serialised_evidence,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
