// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use manager::StorageManager;
use sqlx::ConnectOptions;
use std::path::Path;

mod manager;
mod models;

#[derive(Clone)]
pub(crate) struct Storage {
    pub manager: StorageManager,
}

impl Storage {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, crate::error::Error> {
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let storage = Storage {
            manager: StorageManager { connection_pool },
        };

        Ok(storage)
    }
}
