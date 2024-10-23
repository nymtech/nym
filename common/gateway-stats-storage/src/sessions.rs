// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use time::{Date, OffsetDateTime};

use crate::models::{StoredActiveSession, StoredFinishedSession};

pub(crate) type Result<T> = std::result::Result<T, sqlx::Error>;

#[derive(Clone)]
pub(crate) struct SessionManager {
    connection_pool: sqlx::SqlitePool,
}

impl SessionManager {
    /// Creates new instance of the `SessionsManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        SessionManager { connection_pool }
    }

    pub(crate) async fn insert_finished_session(
        &self,
        date: Date,
        duration_ms: i64,
        typ: String,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO sessions_finished (day, duration_ms, typ) VALUES (?, ?, ?)",
            date,
            duration_ms,
            typ
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_finished_sessions(
        &self,
        date: Date,
    ) -> Result<Vec<StoredFinishedSession>> {
        sqlx::query_as("SELECT duration_ms, typ FROM sessions_finished WHERE day = ?")
            .bind(date)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn delete_finished_sessions(&self, before_date: Date) -> Result<()> {
        sqlx::query!("DELETE FROM sessions_finished WHERE day <= ? ", before_date)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_unique_user(
        &self,
        date: Date,
        client_address_b58: String,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO sessions_unique_users (day, client_address) VALUES (?, ?)",
            date,
            client_address_b58,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_unique_users(&self, date: Date) -> Result<i32> {
        Ok(sqlx::query!(
            "SELECT COUNT(*) as count FROM sessions_unique_users WHERE day = ?",
            date
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count)
    }

    pub(crate) async fn delete_unique_users(&self, before_date: Date) -> Result<()> {
        sqlx::query!("DELETE FROM sessions_finished WHERE day <= ? ", before_date)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_active_session(
        &self,
        client_address_b58: String,
        start_time: OffsetDateTime,
        typ: String,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO sessions_active (client_address, start_time, typ) VALUES (?, ?, ?)",
            client_address_b58,
            start_time,
            typ
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_active_session_type(
        &self,
        client_address_b58: String,
        typ: String,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE sessions_active SET typ = ? WHERE client_address = ?",
            typ,
            client_address_b58,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_active_session(
        &self,
        client_address_b58: String,
    ) -> Result<Option<StoredActiveSession>> {
        sqlx::query_as("SELECT start_time, typ FROM sessions_active WHERE client_address = ?")
            .bind(client_address_b58)
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn get_active_users(&self) -> Result<Vec<String>> {
        Ok(sqlx::query!("SELECT client_address from sessions_active")
            .fetch_all(&self.connection_pool)
            .await?
            .into_iter()
            .map(|record| record.client_address)
            .collect())
    }

    pub(crate) async fn delete_active_session(&self, client_address_b58: String) -> Result<()> {
        sqlx::query!(
            "DELETE FROM sessions_active WHERE client_address = ?",
            client_address_b58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn cleanup_active_sessions(&self) -> Result<()> {
        sqlx::query!("DELETE FROM sessions_active")
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }
}
