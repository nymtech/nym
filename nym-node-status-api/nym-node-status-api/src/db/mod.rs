use anyhow::{anyhow, Result};
use std::{str::FromStr, time::Duration};

pub(crate) mod models;
pub(crate) mod queries;

#[cfg(test)]
mod tests;

use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations_pg");

pub(crate) type DbPool = PgPool;

pub(crate) type DbConnection = sqlx::pool::PoolConnection<sqlx::Postgres>;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init(connection_url: String, _busy_timeout: Duration) -> Result<Self> {
        use std::env;
        let mut connect_options =
            PgConnectOptions::from_str(&connection_url)?.disable_statement_logging();

        let ssl_cert_path = env::var("PG_CERT").ok();

        if let Some(ssl_cert) = ssl_cert_path {
            connect_options = connect_options
                .ssl_mode(sqlx::postgres::PgSslMode::Require)
                .ssl_root_cert(ssl_cert);
        }
        let pool = sqlx::PgPool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        if env::var("SKIP_MIGRATIONS").unwrap_or_default() != "true" {
            MIGRATOR.run(&pool).await?;
        } else {
            tracing::warn!("Skipping migrations");
        }

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
