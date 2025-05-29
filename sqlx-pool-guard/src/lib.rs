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

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod imp;

/// Max number of retry attempts
const CHECK_FILES_CLOSED_MAX_ATTEMPTS: u8 = 10;

/// Delay between file checks
const CHECK_FILES_CLOSED_RETRY_DELAY: Duration = Duration::from_millis(100);

pub struct SqlitePoolGuard {
    database_path: PathBuf,
    connection_pool: sqlx::SqlitePool,
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

impl SqlitePoolGuard {
    pub fn new(database_path: PathBuf, connection_pool: sqlx::SqlitePool) -> Self {
        Self {
            database_path,
            connection_pool,
        }
    }

    /// Close udnerlying sqlite pool and wait for files to be closed before returning.
    pub async fn close_pool(&self) {
        _ = self.close_pool_inner();
    }

    async fn close_pool_inner(&self) -> std::io::Result<()> {
        self.connection_pool.close().await;

        if let Err(e) = self.wait_for_db_files_close().await {
            log::error!("Failed to wait for file to close: {e}");
        }

        Ok(())
    }

    /// Returns all database files, including shm and wal files.
    fn all_database_files(&self) -> Vec<PathBuf> {
        let mut database_files = vec![];
        let canonical_path = self
            .database_path
            .canonicalize()
            .inspect_err(|e| {
                log::error!(
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
                .inspect_err(|e| log::error!("imp::check_files_closed() failure: {e}"))
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

#[cfg(test)]
mod tests {
    use sqlx::{
        ConnectOptions, Executor,
        sqlite::{SqliteAutoVacuum, SqliteSynchronous},
    };

    use super::*;

    #[tokio::test]
    async fn test_wait_close() {
        let temp_dir = tempfile::tempdir().unwrap();
        let database_path = temp_dir.path().join("storage.sqlite");

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

        let guard = SqlitePoolGuard::new(database_path, connection_pool);
        assert!(
            guard
                .wait_for_db_files_close()
                .await
                .err()
                .is_some_and(|e| e.kind() == io::ErrorKind::TimedOut)
        );

        assert!(guard.close_pool_inner().await.is_ok());
    }
}
