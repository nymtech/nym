// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::SqliteScraperError;
use crate::models::{CommitSignature, Validator};
use crate::storage::manager::{
    StorageManager, prune_blocks, prune_messages, prune_pre_commits, prune_transactions,
    update_last_pruned,
};
use crate::storage::transaction::SqliteStorageTransaction;
use async_trait::async_trait;
use nyxd_scraper_shared::storage::helpers::log_db_operation_time;
use nyxd_scraper_shared::storage::{NyxdScraperStorage, NyxdScraperStorageError};
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use sqlx::types::time::OffsetDateTime;
use std::fmt::Debug;
use std::path::Path;
use tokio::time::Instant;
use tracing::{debug, error, info, instrument};

#[derive(Clone)]
pub struct SqliteScraperStorage {
    pub(crate) manager: StorageManager,
}

impl SqliteScraperStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(
        database_path: P,
    ) -> Result<Self, SqliteScraperError> {
        let database_path = database_path.as_ref();
        debug!(
            "initialising scraper database path to '{}'",
            database_path.display()
        );

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
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

        if let Err(err) = sqlx::migrate!("./sql_migrations")
            .run(&connection_pool)
            .await
        {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let manager = StorageManager { connection_pool };
        manager.set_initial_metadata().await?;

        let storage = SqliteScraperStorage { manager };

        Ok(storage)
    }

    #[instrument(skip(self))]
    pub async fn prune_storage(
        &self,
        oldest_to_keep: u32,
        current_height: u32,
    ) -> Result<(), SqliteScraperError> {
        let start = Instant::now();

        let mut tx = self.begin_processing_tx().await?;

        prune_messages(oldest_to_keep.into(), &mut **tx).await?;
        prune_transactions(oldest_to_keep.into(), &mut **tx).await?;
        prune_pre_commits(oldest_to_keep.into(), &mut **tx).await?;
        prune_blocks(oldest_to_keep.into(), &mut **tx).await?;
        update_last_pruned(current_height.into(), &mut **tx).await?;

        let commit_start = Instant::now();
        tx.0.commit()
            .await
            .map_err(|source| SqliteScraperError::StorageTxCommitFailure { source })?;
        log_db_operation_time("committing pruning tx", commit_start);

        log_db_operation_time("pruning storage", start);
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn begin_processing_tx(
        &self,
    ) -> Result<SqliteStorageTransaction, SqliteScraperError> {
        debug!("starting storage tx");
        self.manager
            .connection_pool
            .begin()
            .await
            .map(SqliteStorageTransaction)
            .map_err(|source| SqliteScraperError::StorageTxBeginFailure { source })
    }

    pub async fn lowest_block_height(&self) -> Result<Option<i64>, SqliteScraperError> {
        Ok(self.manager.get_lowest_block().await?)
    }

    pub async fn get_first_block_height_after(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, SqliteScraperError> {
        Ok(self.manager.get_first_block_height_after(time).await?)
    }

    pub async fn get_last_block_height_before(
        &self,
        time: OffsetDateTime,
    ) -> Result<Option<i64>, SqliteScraperError> {
        Ok(self.manager.get_last_block_height_before(time).await?)
    }

    pub async fn get_blocks_between(
        &self,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> Result<i64, SqliteScraperError> {
        let Some(block_start) = self.get_first_block_height_after(start_time).await? else {
            return Ok(0);
        };
        let Some(block_end) = self.get_last_block_height_before(end_time).await? else {
            return Ok(0);
        };

        Ok(block_end - block_start)
    }

    pub async fn get_signed_between(
        &self,
        consensus_address: &str,
        start_height: i64,
        end_height: i64,
    ) -> Result<i64, SqliteScraperError> {
        Ok(self
            .manager
            .get_signed_between(consensus_address, start_height, end_height)
            .await?)
    }

    pub async fn get_signed_between_times(
        &self,
        consensus_address: &str,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> Result<i64, SqliteScraperError> {
        let Some(block_start) = self.get_first_block_height_after(start_time).await? else {
            return Ok(0);
        };
        let Some(block_end) = self.get_last_block_height_before(end_time).await? else {
            return Ok(0);
        };

        self.get_signed_between(consensus_address, block_start, block_end)
            .await
    }

    pub async fn get_precommit(
        &self,
        consensus_address: &str,
        height: i64,
    ) -> Result<Option<CommitSignature>, SqliteScraperError> {
        Ok(self
            .manager
            .get_precommit(consensus_address, height)
            .await?)
    }

    pub async fn get_block_signers(
        &self,
        height: i64,
    ) -> Result<Vec<Validator>, SqliteScraperError> {
        Ok(self.manager.get_block_validators(height).await?)
    }

    pub async fn get_all_known_validators(&self) -> Result<Vec<Validator>, SqliteScraperError> {
        Ok(self.manager.get_validators().await?)
    }

    pub async fn get_last_processed_height(&self) -> Result<i64, SqliteScraperError> {
        Ok(self.manager.get_last_processed_height().await?)
    }

    pub async fn get_pruned_height(&self) -> Result<i64, SqliteScraperError> {
        Ok(self.manager.get_pruned_height().await?)
    }
}

#[async_trait]
impl NyxdScraperStorage for SqliteScraperStorage {
    type StorageTransaction = SqliteStorageTransaction;

    async fn initialise(storage: &str) -> Result<Self, NyxdScraperStorageError> {
        SqliteScraperStorage::init(storage)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn begin_processing_tx(
        &self,
    ) -> Result<Self::StorageTransaction, NyxdScraperStorageError> {
        self.begin_processing_tx()
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn get_last_processed_height(&self) -> Result<i64, NyxdScraperStorageError> {
        self.get_last_processed_height()
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn get_pruned_height(&self) -> Result<i64, NyxdScraperStorageError> {
        self.get_pruned_height()
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn lowest_block_height(&self) -> Result<Option<i64>, NyxdScraperStorageError> {
        self.lowest_block_height()
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn prune_storage(
        &self,
        oldest_to_keep: u32,
        current_height: u32,
    ) -> Result<(), NyxdScraperStorageError> {
        self.prune_storage(oldest_to_keep, current_height)
            .await
            .map_err(NyxdScraperStorageError::from)
    }
}
