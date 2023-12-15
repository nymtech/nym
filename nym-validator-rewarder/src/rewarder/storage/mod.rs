// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::storage::manager::StorageManager;
use crate::rewarder::EpochRewards;
use nym_validator_client::nyxd::Hash;
use sqlx::ConnectOptions;
use std::fmt::Debug;
use std::path::Path;
use tracing::{error, info, instrument};

mod manager;

pub struct RewarderStorage {
    pub(crate) manager: StorageManager,
}

impl RewarderStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(database_path: P) -> Result<Self, NymRewarderError> {
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

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

    pub(crate) async fn load_last_rewarding_epoch(
        &self,
    ) -> Result<Option<Epoch>, NymRewarderError> {
        Ok(self.manager.load_last_rewarding_epoch().await?)
    }

    pub(crate) async fn save_rewarding_information(
        &self,
        reward: EpochRewards,
        rewarding_tx: Result<Hash, NymRewarderError>,
    ) -> Result<(), NymRewarderError> {
        info!("persisting reward details");
        let (reward_tx, reward_err) = match rewarding_tx {
            Ok(hash) => (Some(hash.to_string()), None),
            Err(err) => (None, Some(err.to_string())),
        };

        let epoch_id = reward.epoch.id;

        self.manager
            .insert_rewarding_epoch(
                reward.epoch,
                reward.total_budget.to_string(),
                reward_tx,
                reward_err,
            )
            .await?;

        self.manager
            .insert_rewarding_epoch_block_signing(
                epoch_id,
                reward.signing.total_voting_power_at_epoch_start,
                reward.signing.blocks,
                reward.signing_budget.to_string(),
            )
            .await?;

        for validator in reward.signing.validators {
            let reward_amount = validator.reward_amount(&reward.signing_budget).to_string();
            self.manager
                .insert_rewarding_epoch_block_signing_reward(
                    epoch_id,
                    validator.validator.consensus_address,
                    validator.operator_account.to_string(),
                    reward_amount,
                    validator.voting_power_at_epoch_start,
                    validator.voting_power_ratio.to_string(),
                    validator.signed_blocks,
                    validator.ratio_signed.to_string(),
                )
                .await?;
        }
        //
        // self.manager
        //     .insert_rewarding_epoch_credential_issuance(epoch_id)
        //     .await?;
        //
        // for api_runner in reward.credentials.api_runners {
        //     self.manager
        //         .insert_rewarding_epoch_credential_issuance_reward(epoch_id)
        //         .await?;
        // }

        Ok(())
    }
}
