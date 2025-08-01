// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backend::fs_backend::{
    error::StorageError,
    models::{ReplySurbStorageMetadata, StoredReplyKey, StoredReplySurb, StoredSurbSender},
};
use sqlx::{
    sqlite::{SqliteAutoVacuum, SqliteSynchronous},
    ConnectOptions,
};
use std::path::Path;
use time::OffsetDateTime;
use tracing::{error, info};

use sqlx_pool_guard::SqlitePoolGuard;

#[derive(Debug, Clone)]
pub struct StorageManager {
    connection_pool: SqlitePoolGuard,
}

// all SQL goes here
impl StorageManager {
    pub async fn init<P: AsRef<Path>>(database_path: P, fresh: bool) -> Result<Self, StorageError> {
        // ensure the whole directory structure exists
        if let Some(parent_dir) = database_path.as_ref().parent() {
            std::fs::create_dir_all(parent_dir).map_err(|source| {
                StorageError::DatabasePathUnableToCreateParentDirectory {
                    provided_path: database_path.as_ref().to_path_buf(),
                    source,
                }
            })?;
        }

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(fresh)
            .disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(pool) => pool,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(StorageError::DatabaseConnectionError { source: err });
            }
        };

        let connection_pool = SqlitePoolGuard::new(connection_pool);

        if let Err(err) = sqlx::migrate!("./fs_surbs_migrations")
            .run(&*connection_pool)
            .await
        {
            error!("Failed to initialize SQLx database: {err}");
            connection_pool.close().await;
            return Err(err.into());
        }

        info!("Database migration finished!");
        Ok(StorageManager { connection_pool })
    }

    /// Close connection pool waiting for all connections to be closed.
    pub async fn close_pool(&self) {
        self.connection_pool.close().await;
    }

    #[allow(dead_code)]
    pub async fn status_table_exists(&self) -> Result<bool, sqlx::Error> {
        sqlx::query!("SELECT name FROM sqlite_master WHERE type='table' AND name='status'")
            .fetch_optional(&*self.connection_pool)
            .await
            .map(|r| r.is_some())
    }

    pub async fn create_status_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO status(flush_in_progress, previous_flush, client_in_use) VALUES (0, 0, 1)"
        )
        .execute(&*self.connection_pool)
        .await?;
        Ok(())
    }

    pub async fn get_flush_status(&self) -> Result<bool, sqlx::Error> {
        sqlx::query!("SELECT flush_in_progress FROM status;")
            .fetch_one(&*self.connection_pool)
            .await
            .map(|r| r.flush_in_progress > 0)
    }

    pub async fn set_previous_flush(&self, timestamp: OffsetDateTime) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE status SET previous_flush = ?", timestamp)
            .execute(&*self.connection_pool)
            .await?;
        Ok(())
    }

    pub async fn get_previous_flush_time(&self) -> Result<OffsetDateTime, sqlx::Error> {
        sqlx::query!(r#"SELECT previous_flush AS "previous_flush: OffsetDateTime" FROM status"#)
            .fetch_one(&*self.connection_pool)
            .await
            .map(|r| r.previous_flush)
    }

    pub async fn set_flush_status(&self, in_progress: bool) -> Result<(), sqlx::Error> {
        let in_progress_int = i64::from(in_progress);
        sqlx::query!("UPDATE status SET flush_in_progress = ?", in_progress_int)
            .execute(&*self.connection_pool)
            .await?;
        Ok(())
    }

    pub async fn get_client_in_use_status(&self) -> Result<bool, sqlx::Error> {
        sqlx::query!("SELECT client_in_use FROM status;")
            .fetch_one(&*self.connection_pool)
            .await
            .map(|r| r.client_in_use > 0)
    }

    pub async fn set_client_in_use_status(&self, in_use: bool) -> Result<(), sqlx::Error> {
        let in_use_int = i64::from(in_use);
        sqlx::query!("UPDATE status SET client_in_use = ?", in_use_int)
            .execute(&*self.connection_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_all_reply_keys(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM reply_key;")
            .execute(&*self.connection_pool)
            .await?;
        Ok(())
    }

    pub async fn get_reply_keys(&self) -> Result<Vec<StoredReplyKey>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM reply_key;")
            .fetch_all(&*self.connection_pool)
            .await
    }

    pub async fn insert_reply_key(
        &self,
        stored_reply_key: StoredReplyKey,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO reply_key(key_digest, reply_key, sent_at) VALUES (?, ?, ?);
            "#,
            stored_reply_key.key_digest,
            stored_reply_key.reply_key,
            stored_reply_key.sent_at
        )
        .execute(&*self.connection_pool)
        .await?;
        Ok(())
    }

    pub async fn get_surb_senders(&self) -> Result<Vec<StoredSurbSender>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM reply_surb_sender;")
            .fetch_all(&*self.connection_pool)
            .await
    }

    pub async fn insert_surb_sender(
        &self,
        stored_surb_sender: StoredSurbSender,
    ) -> Result<i64, sqlx::Error> {
        let id = sqlx::query!(
            r#"
                INSERT INTO reply_surb_sender(tag, last_sent) VALUES (?, ?);
            "#,
            stored_surb_sender.tag,
            stored_surb_sender.last_sent
        )
        .execute(&*self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    pub async fn get_reply_surbs(
        &self,
        sender_id: i64,
    ) -> Result<Vec<StoredReplySurb>, sqlx::Error> {
        sqlx::query_as!(
            StoredReplySurb,
            r#"
                SELECT reply_surb_sender_id, reply_surb, encoded_key_rotation as "encoded_key_rotation: u8" FROM reply_surb
                WHERE reply_surb_sender_id = ?
            "#,
            sender_id
        )
        .fetch_all(&*self.connection_pool)
        .await
    }

    pub async fn delete_all_reply_surb_data(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM reply_surb;")
            .execute(&*self.connection_pool)
            .await?;

        sqlx::query!("DELETE FROM reply_surb_sender;")
            .execute(&*self.connection_pool)
            .await?;

        Ok(())
    }

    pub async fn insert_reply_surb(
        &self,
        stored_reply_surb: StoredReplySurb,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO reply_surb(reply_surb_sender_id, reply_surb, encoded_key_rotation) VALUES (?, ?, ?);
            "#,
            stored_reply_surb.reply_surb_sender_id,
            stored_reply_surb.reply_surb,
            stored_reply_surb.encoded_key_rotation
        )
        .execute(&*self.connection_pool)
        .await?;
        Ok(())
    }

    pub async fn get_reply_surb_storage_metadata(
        &self,
    ) -> Result<ReplySurbStorageMetadata, sqlx::Error> {
        sqlx::query_as!(
            ReplySurbStorageMetadata,
             r#"
                SELECT min_reply_surb_threshold as "min_reply_surb_threshold: u32", max_reply_surb_threshold as "max_reply_surb_threshold: u32" FROM reply_surb_storage_metadata;
             "#,
        )
            .fetch_one(&*self.connection_pool)
            .await
    }

    pub async fn insert_reply_surb_storage_metadata(
        &self,
        metadata: ReplySurbStorageMetadata,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"
            INSERT INTO reply_surb_storage_metadata(min_reply_surb_threshold, max_reply_surb_threshold)
            VALUES (?, ?);
        "#,
            metadata.min_reply_surb_threshold,
            metadata.max_reply_surb_threshold,
        ).execute(&*self.connection_pool).await?;
        Ok(())
    }
}
