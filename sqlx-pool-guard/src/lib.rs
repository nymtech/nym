// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(windows)]
#[path = "windows.rs"]
mod imp;

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[path = "apple.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod imp;

/// Max number of retry attempts
const CHECK_FILES_CLOSED_MAX_ATTEMPTS: u8 = 20;

/// Delay between file checks
const CHECK_FILES_CLOSED_RETRY_DELAY: Duration = Duration::from_millis(100);

/// `sqlx::SqlitePool` wrapper providing a workaround for the [known bug](https://github.com/launchbadge/sqlx/issues/3217).
/// In principle after requesting to close the sqlite pool, the wrapper monitors open file descriptor and polls periodically until all database files are closed.
#[derive(Debug, Clone)]
pub struct SqlitePoolGuard {
    /// Path to sqlite database file.
    database_path: PathBuf,

    /// Inner connection pool.
    connection_pool: sqlx::SqlitePool,
}

impl SqlitePoolGuard {
    /// Create new instance providing path to database and connection pool
    pub fn new(connection_pool: sqlx::SqlitePool) -> Self {
        let database_path = connection_pool
            .connect_options()
            .get_filename()
            .to_path_buf();

        Self {
            database_path,
            connection_pool,
        }
    }

    /// Returns database path
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Close udnerlying sqlite pool and wait for files to be closed before returning.
    pub async fn close(&self) {
        // Avoid waiting for db files once the pool is marked closed to ensure that we don't wait on some other sqlite pool to close the database.
        if !self.connection_pool.is_closed() {
            tracing::info!("Closing sqlite pool: {}", self.database_path.display());
            self.close_pool_inner().await.ok();
        }
    }

    async fn close_pool_inner(&self) -> std::io::Result<()> {
        self.connection_pool.close().await;

        self.wait_for_db_files_close().await.inspect_err(|e| {
            tracing::error!("Failed to wait for file to close: {e}");
        })
    }

    /// Returns all database files, including shm and wal files.
    fn all_database_files(&self) -> Vec<PathBuf> {
        let mut database_files = vec![];
        let canonical_path = self
            .database_path
            .canonicalize()
            .inspect_err(|e| {
                tracing::error!(
                    "Failed to canonicalize path: {}. Cause: {e}",
                    self.database_path.display()
                );
            })
            .unwrap_or(self.database_path.clone());

        if let Some(ext) = canonical_path.extension() {
            for added_ext in ["-shm", "-wal"] {
                let mut new_ext = ext.to_owned();
                new_ext.push(added_ext);
                database_files.push(canonical_path.with_extension(new_ext));
            }
        }
        database_files.push(canonical_path);
        database_files
    }

    /// Wait for database files to be closed before returning.
    async fn wait_for_db_files_close(&self) -> std::io::Result<()> {
        let database_files = self.all_database_files();
        let paths: Vec<&Path> = database_files.iter().map(PathBuf::as_path).collect();

        for _ in 0..CHECK_FILES_CLOSED_MAX_ATTEMPTS {
            match imp::check_files_closed(&paths)
                .await
                .inspect_err(|e| tracing::error!("imp::check_files_closed() failure: {e}"))
            {
                Ok(false) | Err(_) => tokio::time::sleep(CHECK_FILES_CLOSED_RETRY_DELAY).await,
                Ok(true) => return Ok(()),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "timed out waiting for sqlite files to be closed",
        ))
    }
}

impl Deref for SqlitePoolGuard {
    type Target = sqlx::SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.connection_pool
    }
}

impl DerefMut for SqlitePoolGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection_pool
    }
}
#[cfg(test)]
mod tests {
    use sqlx::{
        ConnectOptions, Executor,
        sqlite::{SqliteAutoVacuum, SqliteSynchronous},
    };

    use super::*;

    #[tokio::test]
    async fn test_wait_close() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .init();

        let temp_dir = tempfile::tempdir().unwrap();
        let database_path = temp_dir.path().join("storage.db");

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path.clone())
            .create_if_missing(true)
            .disable_statement_logging();
        let connection_pool = sqlx::SqlitePool::connect_with(opts).await.unwrap();

        connection_pool
            .execute("create table test (col int)")
            .await
            .unwrap();

        let guard = SqlitePoolGuard::new(connection_pool);
        assert!(
            guard
                .wait_for_db_files_close()
                .await
                .err()
                .is_some_and(|e| e.kind() == io::ErrorKind::TimedOut)
        );

        assert!(guard.close_pool_inner().await.is_ok());
        tokio::fs::remove_file(database_path).await.unwrap();
    }
}
