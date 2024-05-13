// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backends::sqlite::CoconutCredentialManager;
use crate::error::StorageError;
use crate::storage::Storage;

use crate::models::{StorableIssuedCredential, StoredIssuedCredential};
use async_trait::async_trait;
use log::{debug, error};
use sqlx::ConnectOptions;
use std::path::Path;

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
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
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
    type StorageError = StorageError;

    async fn insert_issued_credential<'a>(
        &self,
        bandwidth_credential: StorableIssuedCredential<'a>,
    ) -> Result<(), Self::StorageError> {
        self.coconut_credential_manager
            .insert_issued_credential(
                bandwidth_credential.credential_type,
                bandwidth_credential.serialization_revision,
                bandwidth_credential.credential_data,
                bandwidth_credential.epoch_id,
            )
            .await
            .map_err(|err| {
                // There is one error we want to handle specifically.
                // Check if database_error is `SqliteError` with code 2067 which
                // means UNIQUE constraint violation
                if let Some(db_error) = err.as_database_error() {
                    if db_error.code().map_or(false, |code| code == "2067") {
                        StorageError::ConstraintUnique
                    } else {
                        err.into()
                    }
                } else {
                    err.into()
                }
            })
    }

    async fn get_next_unspent_credential(
        &self,
        gateway_id: &str,
    ) -> Result<Option<StoredIssuedCredential>, Self::StorageError> {
        // first try to get a free pass if available, otherwise fallback to bandwidth voucher
        let maybe_freepass = self
            .coconut_credential_manager
            .get_next_unspect_freepass(gateway_id)
            .await?;
        if maybe_freepass.is_some() {
            return Ok(maybe_freepass);
        }

        Ok(self
            .coconut_credential_manager
            .get_next_unspect_bandwidth_voucher()
            .await?)
    }

    async fn consume_coconut_credential(
        &self,
        id: i64,
        gateway_id: &str,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .consume_coconut_credential(id, gateway_id)
            .await?;

        Ok(())
    }

    async fn mark_expired(&self, id: i64) -> Result<(), Self::StorageError> {
        self.coconut_credential_manager.mark_expired(id).await?;

        Ok(())
    }
}
