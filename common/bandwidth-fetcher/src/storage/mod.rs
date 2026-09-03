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
use nym_validator_client::nym_api::EpochId;

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

    /// In-memory storage for tests, migrated the same way as the real thing. Has no file backing,
    /// so [`Self::reset`] does not apply to it.
    #[cfg(test)]
    pub(crate) async fn init_in_memory() -> Result<Self, StorageError> {
        let connection_pool = SqlitePoolGuard::new(
            sqlx::sqlite::SqlitePoolOptions::new()
                // every connection to `:memory:` is its own database, so the pool has to be held
                // to a single one or the migrations land somewhere the queries cannot see
                .min_connections(1)
                .max_connections(1)
                .connect_with(
                    sqlx::sqlite::SqliteConnectOptions::new()
                        .filename(":memory:")
                        .create_if_missing(true)
                        .disable_statement_logging(),
                )
                .await?,
        );

        sqlx::migrate!("./migrations")
            .run(&*connection_pool)
            .await?;

        Ok(Self {
            database_path: PathBuf::new(),
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
        dkg_epoch_id: EpochId,
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
                dkg_epoch_id as i64,
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
                        issuance_epoch: p.dkg_epoch_id.map(|e| e as EpochId),
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

#[cfg(test)]
mod tests {
    use super::*;
    use nym_bandwidth_controller::TicketType;
    use nym_crypto::asymmetric::ed25519;
    use rand::rngs::OsRng;

    fn issuance_fixture(deposit_id: u32) -> IssuanceTicketBook {
        let mut rng = OsRng;
        IssuanceTicketBook::new(
            deposit_id,
            b"client-id",
            ed25519::PrivateKey::new(&mut rng),
            TicketType::V1MixnetEntry,
        )
    }

    /// A collection resumed after a restart has to continue under the epoch its existing shares
    /// were signed under. Resolving the epoch afresh would risk finishing under a later one, whose
    /// shares cannot be aggregated with what has already been gathered - and the deposit cannot be
    /// spent a second time to start over.
    #[tokio::test]
    async fn a_pending_issuance_remembers_the_epoch_it_is_being_collected_under() {
        let storage = PendingCredentialRequestsStorage::init_in_memory()
            .await
            .unwrap();

        storage
            .insert_pending_ticketbook(&issuance_fixture(42), 7)
            .await
            .unwrap();

        let pending = storage.get_pending_ticketbooks().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].issuance_epoch, Some(7));
    }

    /// Rows written before the epoch was recorded have to keep loading, and to say they do not know
    /// rather than claim an epoch. Those resolve one afresh on resume, which is what they did all
    /// along.
    #[tokio::test]
    async fn an_issuance_stored_before_the_epoch_was_recorded_reports_none() {
        let storage = PendingCredentialRequestsStorage::init_in_memory()
            .await
            .unwrap();

        let book = issuance_fixture(42);
        let ser = book.pack();
        storage
            .storage_manager
            .insert_legacy_pending_ticketbook(
                ser.revision,
                book.deposit_id(),
                &ser.data,
                book.expiration_date(),
            )
            .await
            .unwrap();

        let pending = storage.get_pending_ticketbooks().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].issuance_epoch, None);
    }
}
