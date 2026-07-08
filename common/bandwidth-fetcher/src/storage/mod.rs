// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials::{IssuanceTicketBook, ecash::bandwidth::serialiser::VersionedSerialise};
use nym_sqlx_pool_guard::SqlitePoolGuard;

use sqlite::SqliteZkNymRequestsStorageManager;
use sqlx::{
    ConnectOptions,
    sqlite::{SqliteAutoVacuum, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use zeroize::Zeroizing;

use crate::storage::{error::StorageError, models::RetrievedPendingTicketbook};

pub(crate) mod error;
pub(crate) mod models;
mod sqlite;

#[derive(Clone)]
pub(crate) struct PendingCredentialRequestsStorage {
    database_path: PathBuf,
    storage_manager: SqliteZkNymRequestsStorageManager,
}

impl PendingCredentialRequestsStorage {
    pub(crate) async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        let database_path = database_path.as_ref();

        tracing::debug!(
            "Setting up pending credential requests storage: {}",
            database_path.display()
        );

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        tracing::debug!("Connecting to the database");
        let connection_pool = SqlitePoolGuard::new(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect_with(opts)
                .await?,
        );

        tracing::debug!("Running migrations");
        if let Err(e) = sqlx::migrate!("./migrations").run(&*connection_pool).await {
            connection_pool.close().await;
            return Err(e.into());
        }

        Ok(Self {
            database_path: database_path.to_path_buf(),
            storage_manager: SqliteZkNymRequestsStorageManager::new(connection_pool),
        })
    }

    pub(crate) async fn close(&self) {
        self.storage_manager.close().await
    }

    pub(crate) async fn reset(self) -> Result<(), StorageError> {
        // First we close the storage to ensure that all files are closed
        tracing::debug!("Closing pending credential requests storage");
        self.storage_manager.close().await;

        // Calling close on the storage should be enough to ensure that all files
        // are closed but just to be sure we wait a bit
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Then we remove the database file
        tracing::debug!("Removing pending credential requests storage file");
        tokio::fs::remove_file(&self.database_path).await?;
        tracing::info!("Removed file: {}", self.database_path.display());

        Ok(())
    }

    pub(crate) async fn insert_pending_ticketbook(
        &self,
        ticketbook: &IssuanceTicketBook,
    ) -> Result<(), StorageError> {
        let ser = ticketbook.pack();
        let data = Zeroizing::new(ser.data);
        let serialisation_revision = ser.revision;

        self.storage_manager
            .insert_pending_ticketbook(
                serialisation_revision,
                ticketbook.deposit_id(),
                &data,
                ticketbook.expiration_date(),
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn get_pending_ticketbooks(
        &self,
    ) -> Result<Vec<RetrievedPendingTicketbook>, StorageError> {
        let pending = self
            .storage_manager
            .get_pending_ticketbooks()
            .await?
            .into_iter()
            .map(|p| {
                IssuanceTicketBook::try_unpack(&p.pending_ticketbook_data, p.serialization_revision)
                    .map_err(|err| {
                        StorageError::database_inconsistency(format!(
                            "failed to deserialise stored pending ticketbook: {err}"
                        ))
                    })
                    .map(|pending_ticketbook| RetrievedPendingTicketbook {
                        pending_id: p.deposit_id,
                        pending_ticketbook,
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(pending)
    }

    pub(crate) async fn remove_pending_ticketbook(
        &self,
        pending_id: i64,
    ) -> Result<(), StorageError> {
        self.storage_manager
            .remove_pending_ticketbook(pending_id)
            .await?;
        Ok(())
    }
}
