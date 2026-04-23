use anyhow::{Result, anyhow};
use sqlx::{Postgres, migrate::Migrator, postgres::PgConnectOptions};
use std::env;
use std::str::FromStr;
use tracing::info;

pub(crate) mod models;
pub(crate) mod queries {
    pub mod price;
    pub mod wasm;
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = sqlx::Pool<Postgres>;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init(connection_url: String) -> Result<Self> {
        let mut connect_options = PgConnectOptions::from_str(&connection_url)?;

        let ssl_cert_path = env::var("PG_CERT").ok();

        if let Some(ssl_cert) = ssl_cert_path {
            connect_options = connect_options
                .ssl_mode(sqlx::postgres::PgSslMode::Require)
                .ssl_root_cert(ssl_cert);
        }

        let pool = DbPool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|err| anyhow!("Failed to run migrations: {}", err))?;

        info!("✅ Successfully migrated the database");

        Ok(Storage { pool })
    }

    /// Cloning pool is cheap, it's the same underlying set of connections
    pub fn pool_owned(&self) -> DbPool {
        self.pool.clone()
    }
}
