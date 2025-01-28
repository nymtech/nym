// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use error::ClientStatsReportStorageError;
use log::{debug, error};
use nym_statistics_common::report::ClientStatsReport;
use sqlx::{
    sqlite::{SqliteAutoVacuum, SqliteSynchronous},
    ConnectOptions,
};
use std::path::Path;

pub mod error;
//pub mod models;
mod client_stats_report;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct ClientStatsStorage {
    client_stats_report_manager: client_stats_report::ClientStatsReportManager,
}

impl ClientStatsStorage {
    /// Initialises `ClientStatsStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    pub async fn init<P: AsRef<Path> + Send>(
        database_path: P,
    ) -> Result<Self, ClientStatsReportStorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        // TODO: we can inject here more stuff based on our gateway global config
        // struct. Maybe different pool size or timeout intervals?
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

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to perform migration on the SQLx database: {err}");
            return Err(err.into());
        }

        // the cloning here are cheap as connection pool is stored behind an Arc
        Ok(ClientStatsStorage {
            client_stats_report_manager: client_stats_report::ClientStatsReportManager::new(
                connection_pool,
            ),
        })
    }

    pub(crate) async fn store_report(
        &mut self,
        report: ClientStatsReport,
    ) -> Result<(), ClientStatsReportStorageError> {
        Ok(self
            .client_stats_report_manager
            .store_report(report)
            .await?)
    }
}
