// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use error::StatsStorageError;
use models::{ActiveSession, FinishedSession, SessionType, StoredFinishedSession};
use nym_sphinx::DestinationAddressBytes;
use sessions::SessionManager;
use sqlx::ConnectOptions;
use std::path::Path;
use time::Date;
use tracing::{debug, error};

pub mod error;
pub mod models;
mod sessions;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct PersistentStatsStorage {
    session_manager: SessionManager,
}

impl PersistentStatsStorage {
    /// Initialises `PersistentStatsStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    pub async fn init<P: AsRef<Path> + Send>(database_path: P) -> Result<Self, StatsStorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        // TODO: we can inject here more stuff based on our gateway global config
        // struct. Maybe different pool size or timeout intervals?
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
            error!("Failed to perform migration on the SQLx database: {err}");
            return Err(err.into());
        }

        // the cloning here are cheap as connection pool is stored behind an Arc
        Ok(PersistentStatsStorage {
            session_manager: sessions::SessionManager::new(connection_pool),
        })
    }

    //Sessions fn
    pub async fn insert_finished_session(
        &self,
        date: Date,
        session: FinishedSession,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .insert_finished_session(
                date,
                session.duration.whole_milliseconds() as i64,
                session.typ.to_string().into(),
            )
            .await?)
    }

    pub async fn get_finished_sessions(
        &self,
        date: Date,
    ) -> Result<Vec<StoredFinishedSession>, StatsStorageError> {
        Ok(self.session_manager.get_finished_sessions(date).await?)
    }

    pub async fn delete_finished_sessions(
        &self,
        before_date: Date,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .delete_finished_sessions(before_date)
            .await?)
    }

    pub async fn insert_unique_user(
        &self,
        date: Date,
        client_address_bs58: String,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .insert_unique_user(date, client_address_bs58)
            .await?)
    }

    pub async fn get_unique_users_count(&self, date: Date) -> Result<i32, StatsStorageError> {
        Ok(self.session_manager.get_unique_users_count(date).await?)
    }

    pub async fn delete_unique_users(&self, before_date: Date) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .delete_unique_users(before_date)
            .await?)
    }

    pub async fn insert_active_session(
        &self,
        client_address: DestinationAddressBytes,
        session: ActiveSession,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .insert_active_session(
                client_address.as_base58_string(),
                session.start,
                session.typ.to_string().into(),
            )
            .await?)
    }

    pub async fn update_active_session_type(
        &self,
        client_address: DestinationAddressBytes,
        session_type: SessionType,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .update_active_session_type(
                client_address.as_base58_string(),
                session_type.to_string().into(),
            )
            .await?)
    }

    pub async fn get_active_session(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<ActiveSession>, StatsStorageError> {
        Ok(self
            .session_manager
            .get_active_session(client_address.as_base58_string())
            .await?
            .map(Into::into))
    }

    pub async fn get_all_active_sessions(&self) -> Result<Vec<ActiveSession>, StatsStorageError> {
        Ok(self
            .session_manager
            .get_all_active_sessions()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn get_started_sessions_count(
        &self,
        start_date: Date,
    ) -> Result<i32, StatsStorageError> {
        Ok(self
            .session_manager
            .get_started_sessions_count(start_date)
            .await?)
    }

    pub async fn get_active_users(&self) -> Result<Vec<String>, StatsStorageError> {
        Ok(self.session_manager.get_active_users().await?)
    }

    pub async fn delete_active_session(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StatsStorageError> {
        Ok(self
            .session_manager
            .delete_active_session(client_address.as_base58_string())
            .await?)
    }

    pub async fn cleanup_active_sessions(&self) -> Result<(), StatsStorageError> {
        Ok(self.session_manager.cleanup_active_sessions().await?)
    }
}
