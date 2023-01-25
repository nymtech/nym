/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::coconut::CoconutCredentialManager;
use crate::error::StorageError;
use crate::storage::Storage;

use crate::models::CoconutCredential;
use async_trait::async_trait;
use log::{debug, error};
use sqlx::ConnectOptions;
use std::path::{Path, PathBuf};

mod coconut;
pub mod error;
mod models;
pub mod storage;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct PersistentStorage {
    coconut_credential_manager: CoconutCredentialManager,
}

impl PersistentStorage {
    /// Initialises `PersistentStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    pub async fn init<P: AsRef<Path> + Send>(database_path: P) -> Result<Self, StorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to perform migration on the SQLx database: {err}");
            return Err(err.into());
        }

        Ok(PersistentStorage {
            coconut_credential_manager: CoconutCredentialManager::new(connection_pool.clone()),
        })
    }
}

#[async_trait]
impl Storage for PersistentStorage {
    async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
        epoch_id: String,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .insert_coconut_credential(
                voucher_value,
                voucher_info,
                serial_number,
                binding_number,
                signature,
                epoch_id,
            )
            .await?;

        Ok(())
    }

    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError> {
        let credential = self
            .coconut_credential_manager
            .get_next_coconut_credential()
            .await?;

        Ok(credential)
    }

    async fn consume_coconut_credential(&self, id: i64) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .consume_coconut_credential(id)
            .await?;

        Ok(())
    }
}

pub async fn initialise_storage(path: PathBuf) -> PersistentStorage {
    match PersistentStorage::init(path).await {
        Err(err) => panic!("failed to initialise credential storage - {err}"),
        Ok(storage) => storage,
    }
}
