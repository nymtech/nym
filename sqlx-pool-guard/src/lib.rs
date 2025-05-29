// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(target_os = "macos")]
use proc_pidinfo::{
    ProcFDInfo, ProcFDType, VnodeFdInfoWithPath, proc_pidfdinfo_self, proc_pidinfo_list_self,
};

const SQL_CLOSE_MAX_ATTEMPTS: u8 = 10;
const SQL_CLOSE_RETRY_DELAY: Duration = Duration::from_millis(100);

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

    /// Close udnerlying sqlite pool
    pub async fn close_pool(&self) {
        _ = self.close_pool_inner();
    }

    async fn close_pool_inner(&self) -> std::io::Result<()> {
        self.connection_pool.close().await;

        if let Err(e) = self.wait_io_close().await {
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

    /// Wait for I/O close to the database files
    ///
    /// - macOS: uses `proc_pidinfo` (`sys/proc_info.h`)
    ///   See: http://blog.palominolabs.com/2012/06/19/getting-the-files-being-used-by-a-process-on-mac-os-x/
    ///
    /// - Linux, Android: uses `/proc/self/fd/` to list open file descriptors
    ///   See: https://stackoverflow.com/a/59797198/351305
    ///
    /// - Windows: attempts to open files to detect whether they are still open.
    async fn wait_io_close(&self) -> std::io::Result<()> {
        let database_files = self.all_database_files();
        let paths: Vec<&Path> = database_files.iter().map(PathBuf::as_path).collect();

        for _ in 0..SQL_CLOSE_MAX_ATTEMPTS {
            match Self::check_io_close(&paths)
                .await
                .inspect_err(|e| log::error!("check_io_close() failure: {e}"))
            {
                Ok(false) | Err(_) => tokio::time::sleep(SQL_CLOSE_RETRY_DELAY).await,
                Ok(true) => return Ok(()),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "timed out waiting for sqlite files to be closed",
        ))
    }

    /// Check if no more open file descriptors exist for the given files.
    #[cfg(target_os = "macos")]
    async fn check_io_close(file_paths: &[&Path]) -> io::Result<bool> {
        let fd_list = proc_pidinfo_list_self::<ProcFDInfo>()?;

        for fd in fd_list
            .iter()
            .filter(|s| s.fd_type() == Ok(ProcFDType::VNODE))
        {
            let Some(vnode) = proc_pidfdinfo_self::<VnodeFdInfoWithPath>(fd.proc_fd)
                .inspect_err(|e| {
                    log::warn!("proc_pidfdinfo_self::<VnodeFdInfoWithPath>() failure: {e}");
                })
                .ok()
                .flatten()
            else {
                continue;
            };

            if let Ok(true) = vnode
                .path()
                .map(|vnode_path| file_paths.contains(&vnode_path))
                .inspect_err(|e| {
                    log::warn!("vnode.path() failure: {e:?}");
                })
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if no more open file descriptors exist for the given files.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    async fn check_io_close(file_paths: &[&Path]) -> io::Result<bool> {
        let mut dir = tokio::fs::read_dir("/proc/self/fd/").await?;

        while let Ok(Some(entry)) = dir.next_entry().await {
            if entry
                .file_type()
                .await
                .inspect_err(|e| log::warn!("entry.file_type() failure: {e}"))
                .is_ok_and(|entry_type| entry_type.is_symlink())
            {
                match tokio::fs::read_link(entry.path()).await {
                    Ok(resolved_path) => {
                        if file_paths.contains(&resolved_path.as_ref()) {
                            return Ok(false);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read symlink: {e}");
                    }
                }
            }
        }

        Ok(true)
    }

    #[cfg(windows)]
    async fn check_io_close(file_paths: &[&Path]) -> io::Result<bool> {
        // Error code returned when file is still in use.
        const FILE_IN_USE_ERR: i32 = 32;

        for file_path in file_paths {
            if let Err(e) = tokio::fs::OpenOptions::new()
                .read(true)
                .open(file_path)
                .await
            {
                if e.raw_os_error() == Some(FILE_IN_USE_ERR) {
                    return Ok(false);
                } else if e.kind() != io::ErrorKind::NotFound {
                    log::error!("Failed to open file: {}", file_path.display());
                }
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{
        ConnectOptions,
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

        let guard = SqlitePoolGuard::new(database_path, connection_pool);
        assert!(
            guard
                .wait_io_close()
                .await
                .err()
                .is_some_and(|e| e.kind() == io::ErrorKind::TimedOut)
        );

        assert!(guard.close_pool_inner().await.is_ok());
    }
}
