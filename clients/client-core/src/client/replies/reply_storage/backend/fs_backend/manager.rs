// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::backend::fs_backend::error::StorageError;
use crate::client::replies::reply_storage::backend::fs_backend::models::{
    ReplySurbStorageMetadata, StoredReplyKey, StoredReplySurb, StoredSenderTag, StoredSurbSender,
};
use log::{error, info};
use sqlx::ConnectOptions;
use std::path::Path;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    pub(crate) async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(pool) => pool,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(StorageError::DatabaseConnectionError { source: err });
            }
        };

        if let Err(err) = sqlx::migrate!("./fs_surbs_migrations")
            .run(&connection_pool)
            .await
        {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");
        Ok(StorageManager { connection_pool })
    }

    pub(crate) async fn create_status_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO status(flush_in_progress) VALUES (0)")
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_flush_status(&self) -> Result<bool, sqlx::Error> {
        sqlx::query!("SELECT flush_in_progress FROM status;")
            .fetch_one(&self.connection_pool)
            .await
            .map(|r| r.flush_in_progress > 0)
    }

    pub(crate) async fn set_flush_status(&self, in_progress: bool) -> Result<(), sqlx::Error> {
        let in_progress_int = i64::from(in_progress);
        sqlx::query!("UPDATE status SET flush_in_progress = ?", in_progress_int)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_tags(&self) -> Result<Vec<StoredSenderTag>, sqlx::Error> {
        sqlx::query_as!(StoredSenderTag, "SELECT * FROM sender_tag;",)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_tag(&self, stored_tag: StoredSenderTag) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO sender_tag(recipient, tag) VALUES (?, ?);
            "#,
            stored_tag.recipient,
            stored_tag.tag
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_reply_keys(&self) -> Result<Vec<StoredReplyKey>, sqlx::Error> {
        sqlx::query_as!(StoredReplyKey, "SELECT * FROM reply_key;",)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_reply_key(
        &self,
        stored_reply_key: StoredReplyKey,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO reply_key(key_digest, reply_key) VALUES (?, ?);
            "#,
            stored_reply_key.key_digest,
            stored_reply_key.reply_key
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_surb_senders(&self) -> Result<Vec<StoredSurbSender>, sqlx::Error> {
        sqlx::query_as!(StoredSurbSender, "SELECT * FROM reply_surb_sender;",)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_surb_sender(
        &self,
        stored_surb_sender: StoredSurbSender,
    ) -> Result<i64, sqlx::Error> {
        let id = sqlx::query!(
            r#"
                INSERT INTO reply_surb_sender(tag, last_sent_timestamp) VALUES (?, ?);
            "#,
            stored_surb_sender.tag,
            stored_surb_sender.last_sent_timestamp
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    pub(crate) async fn get_reply_surbs(
        &self,
        sender_id: i64,
    ) -> Result<Vec<StoredReplySurb>, sqlx::Error> {
        sqlx::query_as!(
            StoredReplySurb,
            "SELECT * FROM reply_surb WHERE reply_surb_sender_id = ?",
            sender_id
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(crate) async fn insert_reply_surb(
        &self,
        stored_reply_surb: StoredReplySurb,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO reply_surb(reply_surb_sender_id, reply_surb) VALUES (?, ?);
            "#,
            stored_reply_surb.reply_surb_sender_id,
            stored_reply_surb.reply_surb
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_reply_surb_storage_metadata(
        &self,
    ) -> Result<ReplySurbStorageMetadata, sqlx::Error> {
        sqlx::query_as!(
            ReplySurbStorageMetadata,
             r#"
                SELECT min_reply_surb_threshold as "min_reply_surb_threshold: u32", max_reply_surb_threshold as "max_reply_surb_threshold: u32" FROM reply_surb_storage_metadata;
             "#,
        )
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn insert_reply_surb_storage_metadata(
        &self,
        metadata: ReplySurbStorageMetadata,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"
            INSERT INTO reply_surb_storage_metadata(min_reply_surb_threshold, max_reply_surb_threshold)
            VALUES (?, ?);
        "#,
            metadata.min_reply_surb_threshold,
            metadata.max_reply_surb_threshold,
        ).execute(&self.connection_pool).await?;
        Ok(())
    }
}
