use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};
use std::str::FromStr;

pub(crate) mod models;
pub(crate) mod queries;

pub(crate) const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = PgPool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init() -> Result<Self> {
        let connection_url = std::env::var(DATABASE_URL_ENV_VAR).map_err(anyhow::Error::from)?;
        let connect_options =
            PgConnectOptions::from_str(&connection_url)?.disable_statement_logging();

        let pool = DbPool::connect_with(connect_options)
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
