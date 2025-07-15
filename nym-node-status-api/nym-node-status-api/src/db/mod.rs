use anyhow::{anyhow, Result};
use std::{str::FromStr, time::Duration};

pub(crate) mod models;
pub(crate) mod queries;
pub(crate) mod query_wrapper;

// Re-export the query wrapper functions for easier access
pub(crate) use query_wrapper::query;
#[allow(unused_imports)]
pub(crate) use query_wrapper::{query_as, query_scalar};

#[cfg(feature = "sqlite")]
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteSynchronous},
    ConnectOptions, SqlitePool,
};

#[cfg(feature = "pg")]
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};

#[cfg(feature = "sqlite")]
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[cfg(feature = "pg")]
static MIGRATOR: Migrator = sqlx::migrate!("./migrations_pg");

#[cfg(feature = "sqlite")]
pub(crate) type DbPool = SqlitePool;

#[cfg(feature = "pg")]
pub(crate) type DbPool = PgPool;

#[cfg(feature = "sqlite")]
pub(crate) type DbConnection = sqlx::pool::PoolConnection<sqlx::Sqlite>;

#[cfg(feature = "pg")]
pub(crate) type DbConnection = sqlx::pool::PoolConnection<sqlx::Postgres>;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    #[cfg(feature = "sqlite")]
    pub async fn init(connection_url: String, busy_timeout: Duration) -> Result<Self> {
        let connect_options = SqliteConnectOptions::from_str(&connection_url)?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(busy_timeout)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .foreign_keys(true)
            .create_if_missing(true)
            .disable_statement_logging();

        let pool = sqlx::SqlitePool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR.run(&pool).await?;

        // aftering setting pragma, check whether it was set successfully
        Self::assert_busy_timeout(pool.clone(), busy_timeout.as_secs() as i64).await?;

        Ok(Storage { pool })
    }

    #[cfg(feature = "pg")]
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

        MIGRATOR.run(&pool).await?;

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }

    #[cfg(feature = "sqlite")]
    async fn assert_busy_timeout(pool: DbPool, expected_busy_timeout_s: i64) -> Result<()> {
        let mut conn = pool.acquire().await?;
        // Sqlite stores this value as miliseconds
        // https://www.sqlite.org/pragma.html#pragma_busy_timeout
        let busy_timeout_db = sqlx::query!("PRAGMA busy_timeout;")
            .fetch_one(conn.as_mut())
            .await?;

        let actual_busy_timeout_ms = busy_timeout_db.timeout.unwrap_or(0);
        tracing::info!("PRAGMA busy_timeout={}ms", actual_busy_timeout_ms);
        let expected_busy_timeout_ms = expected_busy_timeout_s * 1000;

        if expected_busy_timeout_ms != actual_busy_timeout_ms {
            anyhow::bail!(
                "PRAGMA busy_timeout expected: {}ms, actual: {}ms",
                expected_busy_timeout_ms,
                actual_busy_timeout_ms
            );
        }

        Ok(())
    }
}
