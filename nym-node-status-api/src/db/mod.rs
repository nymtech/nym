use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};
use std::str::FromStr;

pub(crate) mod models;
pub(crate) mod queries;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init(connection_url: String) -> Result<Self> {
        let connect_options = SqliteConnectOptions::from_str(&connection_url)?
            .create_if_missing(true)
            .disable_statement_logging();

        let pool = sqlx::SqlitePool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR.run(&pool).await?;

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
