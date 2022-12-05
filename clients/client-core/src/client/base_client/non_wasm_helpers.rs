// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::{
    fs_backend, CombinedReplyStorage, ReplyStorageBackend,
};
use crate::config::DebugConfig;
use crate::error::ClientCoreError;
use log::{error, info};
use std::path::Path;

pub async fn setup_fs_reply_surb_backend<P: AsRef<Path>>(
    db_path: P,
    debug_config: &DebugConfig,
) -> Result<fs_backend::Backend, ClientCoreError<fs_backend::Backend>> {
    // if the database file doesnt exist, initialise fresh storage, otherwise attempt to load the existing one
    let db_path = db_path.as_ref();
    if db_path.exists() {
        info!("loading existing surb database");
        match fs_backend::Backend::try_load(db_path).await {
            Ok(backend) => Ok(backend),
            Err(err) => {
                error!("failed to setup persistent storage backend for our reply needs: {err}");
                Err(ClientCoreError::SurbStorageError { source: err })
            }
        }
    } else {
        info!("creating fresh surb database");
        let mut storage_backend = match fs_backend::Backend::init(db_path).await {
            Ok(backend) => backend,
            Err(err) => {
                error!("failed to setup persistent storage backend for our reply needs: {err}");
                return Err(ClientCoreError::SurbStorageError { source: err });
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
            .map_err(|err| ClientCoreError::SurbStorageError { source: err })?;

        Ok(storage_backend)
    }
}
