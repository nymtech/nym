use std::str::FromStr;

use crate::read_env_var;
use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};

pub(crate) mod models;
pub(crate) mod queries;

pub(crate) const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init() -> Result<Self> {
        let connection_url = read_env_var(DATABASE_URL_ENV_VAR)?;
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
    pub async fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
