// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::manager::StorageManager;
use anyhow::Context;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use std::path::Path;
use std::time::Duration;
use tracing::log::{LevelFilter, debug, error};

mod manager;
mod models;

#[derive(Clone)]
pub(crate) struct NetworkMonitorStorage {
    pub(crate) storage_manager: StorageManager,
}

impl NetworkMonitorStorage {
    pub(crate) async fn init<P: AsRef<Path>>(database_path: P) -> anyhow::Result<Self> {
        debug!(
            "attempting to connect to database {}",
            database_path.as_ref().display()
        );

        let connect_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(50));

        let connection_pool = sqlx::SqlitePool::connect_with(connect_opts)
            .await
            .context("Failed to connect to SQLx database")?;

        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .context("Failed to run database migrations")?;

        Ok(Self {
            storage_manager: StorageManager { connection_pool },
        })
    }
}
