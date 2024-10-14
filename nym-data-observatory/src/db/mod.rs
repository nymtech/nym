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
    pub async fn init(connection_url: Option<String>) -> Result<Self> {
        let connection_url = connection_url.unwrap();
        let connect_options = {
            let mut connect_options = PgConnectOptions::from_str(&connection_url)?;
            let connect_options = connect_options.disable_statement_logging();
            (*connect_options).clone()
        };

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
