use crate::read_env_var;
use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, SqlitePool};
pub(crate) const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) mod helpers;
pub(crate) mod models;
pub(crate) mod queries;

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init() -> Result<Self> {
        let connection_url = read_env_var(DATABASE_URL_ENV_VAR)?;

        let pool = sqlx::SqlitePool::connect(&connection_url)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;
        MIGRATOR.run(&pool).await?;

        Ok(Storage { pool })
    }

    pub async fn pool(&self) -> &DbPool {
        &self.pool
    }
}
