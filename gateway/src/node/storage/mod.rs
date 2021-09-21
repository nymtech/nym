// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod error;
pub(crate) mod inboxes;
mod ledger;
mod manager;
mod models;
mod shared_keys;

use crate::node::storage::error::StorageError;
use crate::node::storage::manager::StorageManager;
pub(crate) use ledger::ClientLedger;
use log::{debug, error};
use sqlx::ConnectOptions;
use std::path::Path;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct GatewayStorage {
    manager: StorageManager,
}

impl GatewayStorage {
    /// Initialises `GatewayStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    pub(crate) async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        // TODO: we can inject here more stuff based on our gateway global config
        // struct. Maybe different pool size or timeout intervals?
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {}", err);
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to perform migration on the SQLx database: {}", err);
            return Err(err.into());
        }

        Ok(GatewayStorage {
            manager: StorageManager { connection_pool },
        })
    }
}
