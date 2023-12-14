// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
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

        Ok(())
    }
}
