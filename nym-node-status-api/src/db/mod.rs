use std::str::FromStr;

use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};

pub(crate) mod models;
pub(crate) mod queries;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init(connection_url: String) -> Result<Self> {
        let connect_options = {
            let connect_options = SqliteConnectOptions::from_str(&connection_url)?;
            let mut connect_options = connect_options.create_if_missing(true);
            let connect_options = connect_options.disable_statement_logging();
            (*connect_options).clone()
        };

        let pool = sqlx::SqlitePool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR.run(&pool).await?;

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub async fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
