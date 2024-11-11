use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub(crate) mod models;
pub(crate) mod queries {
    pub mod payments;
    pub mod price;
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init(connection_url: String) -> Result<Self> {
        let connect_options =
            SqliteConnectOptions::from_str(&connection_url)?.create_if_missing(true);

        let pool = DbPool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|err| anyhow!("Failed to run migrations: {}", err))?;

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub async fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
