// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::{
    self, fs_backend, CombinedReplyStorage, ReplyStorageBackend,
};
use crate::config::DebugConfig;
use crate::error::ClientCoreError;
use log::{error, info};
use std::path::Path;
use std::{fs, io};
use time::OffsetDateTime;

async fn setup_fresh_backend<P: AsRef<Path>>(
    db_path: P,
    debug_config: &DebugConfig,
) -> Result<fs_backend::Backend, ClientCoreError> {
    info!("creating fresh surb database");
    let mut storage_backend = match fs_backend::Backend::init(db_path).await {
        Ok(backend) => backend,
        Err(err) => {
            error!("failed to setup persistent storage backend for our reply needs: {err}");
            return Err(ClientCoreError::SurbStorageError {
                source: Box::new(err),
            });
        }
    };

    // while I kinda hate that we're going to be creating `CombinedReplyStorage` twice,
    // it will only be happening on the very first run and in practice won't incur huge
    // costs since the storage is going to be empty
    let mem_store = CombinedReplyStorage::new(
        debug_config.minimum_reply_surb_storage_threshold,
        debug_config.maximum_reply_surb_storage_threshold,
    );
    storage_backend
        .init_fresh(&mem_store)
        .await
        .map_err(|err| ClientCoreError::SurbStorageError {
            source: Box::new(err),
        })?;

    Ok(storage_backend)
}

fn archive_corrupted_database<P: AsRef<Path>>(db_path: P) -> io::Result<()> {
    let db_path = db_path.as_ref();
    debug_assert!(db_path.exists());

    let now = OffsetDateTime::now_utc().unix_timestamp();

    let suffix = format!("_{now}.corrupted");

    let new_extension =
        if let Some(existing_extension) = db_path.extension().and_then(|ext| ext.to_str()) {
            format!("{existing_extension}.{}", suffix)
        } else {
            suffix
        };

    let mut renamed = db_path.to_owned();
    renamed.set_extension(new_extension);

    fs::rename(db_path, renamed)
}

pub async fn setup_fs_reply_surb_backend<P: AsRef<Path>>(
    db_path: P,
    debug_config: &DebugConfig,
) -> Result<fs_backend::Backend, ClientCoreError> {
    // if the database file doesnt exist, initialise fresh storage, otherwise attempt to load the existing one
    let db_path = db_path.as_ref();
    if db_path.exists() {
        info!("loading existing surb database");
        match fs_backend::Backend::try_load(db_path).await {
            Ok(backend) => Ok(backend),
            Err(err) => {
                error!("failed to setup persistent storage backend for our reply needs: {err}. We're going to create a fresh database instead. This behaviour might change in the future");

                archive_corrupted_database(db_path)?;
                setup_fresh_backend(db_path, debug_config).await
            }
        }
    } else {
        setup_fresh_backend(db_path, debug_config).await
    }
}

pub fn setup_empty_reply_surb_backend(debug_config: &DebugConfig) -> reply_storage::Empty {
    reply_storage::Empty {
        min_surb_threshold: debug_config.minimum_reply_surb_storage_threshold,
        max_surb_threshold: debug_config.maximum_reply_surb_storage_threshold,
    }
}
