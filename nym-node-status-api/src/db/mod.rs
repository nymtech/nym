use anyhow::{anyhow, Result};
use sqlx::{migrate::Migrator, SqlitePool};

use crate::read_env_var;

pub(crate) const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub(crate) type DbPool = SqlitePool;

pub(crate) struct Storage {
    pool: DbPool,
}

impl Storage {
    pub async fn init() -> Result<Self> {
        let connection_url = abs_sqlite_url()?;

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

/// Problem: Sqlite DATABASE_URL env var accepts a relative path, but if the
/// binary was launched from a location where that relative path is invalid,
/// providing raw env var value wouldn't be enough.
///
/// Calculate absolute path for sqlite URL
fn abs_sqlite_url() -> anyhow::Result<String> {
    let crate_path = read_env_var("CARGO_MANIFEST_DIR")?;
    let sqlite_rel_path = read_env_var(DATABASE_URL_ENV_VAR)?.replace("sqlite://", "");
    let abs_path = format!("sqlite://{}/{}", crate_path, sqlite_rel_path);

    tracing::debug!("DB absolute path: {}", abs_path);

    Ok(abs_path)
}
