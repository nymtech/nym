// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::NYMVISOR_DIR;
use crate::error::NymvisorError;
use crate::helpers::init_path;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::fs::{DirEntry, File};
use std::path::{Path, PathBuf};
use time::{format_description, OffsetDateTime};
use tracing::info;

fn generate_backup_filename() -> String {
    // safety: this expect is fine as we're using a constant formatter.
    #[allow(clippy::expect_used)]
    let format = format_description::parse(
        "[year]-[month]-[day]-[hour][minute][second][subsecond digits:3]",
    )
    .expect("our time formatter is malformed");
    #[allow(clippy::expect_used)]
    let now = OffsetDateTime::now_utc()
        .format(&format)
        .expect("our time formatter failed to format the current time");

    format!("backup-{now}-preupgrade.tar.gz")
}

pub(crate) struct BackupBuilder {
    tar_builder: tar::Builder<GzEncoder<File>>,
    backup_filepath: PathBuf,
}

impl BackupBuilder {
    pub(crate) fn new<P: AsRef<Path>>(backup_directory: P) -> Result<Self, NymvisorError> {
        let backup_directory = backup_directory.as_ref();
        let backup_filepath = backup_directory.join(generate_backup_filename());

        // create the backup directory itself (i.e. specific for this upgrade) if it doesn't yet exist
        init_path(backup_directory)?;

        // create the backup file
        let backup_file = fs::File::create(&backup_filepath).map_err(|source| {
            NymvisorError::BackupFileCreationFailure {
                path: backup_filepath.clone(),
                source,
            }
        })?;

        let enc = GzEncoder::new(backup_file, Compression::default());
        let tar_builder = tar::Builder::new(enc);
        Ok(BackupBuilder {
            tar_builder,
            backup_filepath,
        })
    }

    fn backup_subdir(&mut self, dir_entry: DirEntry) -> Result<(), NymvisorError> {
        let path = dir_entry.path();
        let filename = dir_entry.file_name();
        info!(
            "attempting to put {} into the backup tar file",
            path.display()
        );

        if dir_entry.file_name() == NYMVISOR_DIR {
            info!("skipping the /{NYMVISOR_DIR}...");
            return Ok(());
        }

        if path.is_dir() {
            self.tar_builder
                .append_dir_all(filename, &path)
                .map_err(|source| NymvisorError::BackupTarDirFailure {
                    path: self.backup_filepath.clone(),
                    data_source: path,
                    source,
                })
        } else {
            self.tar_builder
                .append_path_with_name(&path, filename)
                .map_err(|source| NymvisorError::BackupTarFileFailure {
                    path: self.backup_filepath.clone(),
                    data_source: path,
                    source,
                })
        }
    }

    fn finish(mut self) -> Result<(), NymvisorError> {
        self.tar_builder
            .finish()
            .map_err(|source| NymvisorError::BackupTarFinalizationFailure {
                path: self.backup_filepath,
                source,
            })
    }

    pub(crate) fn backup_daemon_home<P: AsRef<Path>>(
        mut self,
        daemon_home: P,
    ) -> Result<(), NymvisorError> {
        let home = daemon_home.as_ref();
        let home_entry =
            fs::read_dir(home).map_err(|source| NymvisorError::BackupTarDirFailure {
                path: self.backup_filepath.clone(),
                data_source: home.to_path_buf(),
                source,
            })?;

        for path in home_entry {
            let dir_entry = path.map_err(|source| NymvisorError::BackupTarDirFailure {
                path: self.backup_filepath.clone(),
                data_source: home.to_path_buf(),
                source,
            })?;
            self.backup_subdir(dir_entry)?;
        }
        self.finish()
    }
}
