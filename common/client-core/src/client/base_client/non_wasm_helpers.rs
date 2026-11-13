// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::replies::reply_storage::{CombinedReplyStorage, ReplyStorageBackend, fs_backend},
    config,
    config::Config,
    error::ClientCoreError,
};
use nym_client_core_gateways_storage::OnDiskGatewaysDetails;
use nym_validator_client::{QueryHttpRpcNyxdClient, nyxd};
use std::{io, path::Path};
use time::OffsetDateTime;
use tracing::{error, info, trace};
use url::Url;

/// Maximum rename attempts when the database file is temporarily locked.
#[cfg(windows)]
const ARCHIVE_MAX_RENAME_ATTEMPTS: u8 = 15;

/// Delay between archive rename attempts.
#[cfg(windows)]
const ARCHIVE_RENAME_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

async fn setup_fresh_backend(
    db_path: &Path,
    db_key: Option<&nym_sqlcipher::DatabaseKey>,
    surb_config: &config::ReplySurbs,
) -> Result<fs_backend::Backend, ClientCoreError> {
    info!("Creating fresh surb database");
    let mut storage_backend = match fs_backend::Backend::init(db_path, db_key).await {
        Ok(backend) => backend,
        Err(err) => {
            error!(
                "setup_fresh_backend: Failed to setup persistent storage backend for our reply needs: {err}"
            );
            return Err(ClientCoreError::SurbStorageError {
                source: Box::new(err),
            });
        }
    };

    // while I kinda hate that we're going to be creating `CombinedReplyStorage` twice,
    // it will only be happening on the very first run and in practice won't incur huge
    // costs since the storage is going to be empty
    let mem_store = CombinedReplyStorage::new(
        surb_config.minimum_reply_surb_storage_threshold,
        surb_config.maximum_reply_surb_storage_threshold,
    );
    match storage_backend.init_fresh(&mem_store).await {
        Ok(()) => Ok(storage_backend),
        Err(err) => {
            storage_backend.shutdown().await;
            Err(ClientCoreError::SurbStorageError {
                source: Box::new(err),
            })
        }
    }
}

// fn setup_inactive_backend(surb_config: &config::ReplySurbs) -> fs_backend::Backend {
//     info!("creating inactive surb database");
//     fs_backend::Backend::new_inactive(
//         surb_config.minimum_reply_surb_storage_threshold,
//         surb_config.maximum_reply_surb_storage_threshold,
//     )
// }

async fn archive_corrupted_database<P: AsRef<Path>>(db_path: P) -> io::Result<()> {
    let db_path = db_path.as_ref();
    debug_assert!(db_path.exists());

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let suffix = format!("_{now}.corrupted");

    let new_extension =
        if let Some(existing_extension) = db_path.extension().and_then(|ext| ext.to_str()) {
            format!("{existing_extension}.{suffix}")
        } else {
            suffix
        };
    let renamed = db_path.with_extension(new_extension);

    #[cfg(windows)]
    let result = rename_with_retry(db_path, &renamed).await;
    #[cfg(not(windows))]
    let result = tokio::fs::rename(db_path, &renamed).await;

    result.inspect_err(|err| {
        error!(
            "Failed to rename corrupt database file {} to {}: {err}",
            db_path.display(),
            renamed.display()
        )
    })
}

/// Renames `from` to `to`, retrying while the file is locked.
///
/// While sqlx 0.9 should close database file upon `sqlite_pool.close().await`,
/// it's possible that some of code paths do not wait for pool to close
/// before continuing execution.
#[cfg(windows)]
async fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    for attempt in 1..ARCHIVE_MAX_RENAME_ATTEMPTS {
        match tokio::fs::rename(from, to).await {
            // ERROR_SHARING_VIOLATION = 32, ERROR_LOCK_VIOLATION = 33
            Err(err) if matches!(err.raw_os_error(), Some(32 | 33)) => {
                trace!(
                    "Database file is temporarily locked, retrying rename \
                     (attempt {attempt}/{ARCHIVE_MAX_RENAME_ATTEMPTS}): {err}"
                );
                tokio::time::sleep(ARCHIVE_RENAME_RETRY_DELAY).await;
            }
            result => return result,
        }
    }

    tokio::fs::rename(from, to).await
}

/// Sets up the reply SURB database: loads the existing one, or creates a fresh one.
///
/// With `db_key`, the database is encrypted with SQLCipher using the raw 256-bit key, and an
/// existing plaintext database is encrypted in place first.
pub async fn setup_fs_reply_surb_backend<P: AsRef<Path>>(
    db_path: P,
    db_key: Option<&nym_sqlcipher::DatabaseKey>,
    surb_config: &config::ReplySurbs,
) -> Result<fs_backend::Backend, ClientCoreError> {
    // if the database file doesnt exist, initialise fresh storage, otherwise attempt to load
    // the existing one
    let db_path = db_path.as_ref();
    if db_path.exists() {
        info!("Loading existing surb database");
        match fs_backend::Backend::try_load(db_path, db_key).await {
            Ok(backend) => return Ok(backend),
            Err(err) => handle_load_failure(db_path, err).await?,
        }
    }

    setup_fresh_backend(db_path, db_key, surb_config).await
}

/// Handles failing to load the existing surb database at `db_path`: archives it so a fresh one
/// can be created in its place, unless the failure is down to its encryption, e.g. a wrong key,
/// which a fresh database wouldn't fix and archiving would hide.
async fn handle_load_failure(
    db_path: &Path,
    err: fs_backend::StorageError,
) -> Result<(), ClientCoreError> {
    match err {
        // a fresh database wouldn't fix a wrong key, and archiving would hide the data
        fs_backend::StorageError::EncryptionError { .. } => {
            error!("Failed to open the encrypted surb database: {err}");
            Err(ClientCoreError::SurbStorageError {
                source: Box::new(err),
            })
        }
        _ => {
            error!(
                "setup_fs_reply_surb_backend: Failed to setup persistent storage backend for our reply needs: {err}. We're going to create a fresh database instead. This behaviour might change in the future"
            );
            archive_corrupted_database(db_path).await?;
            Ok(())
        }
    }
}

/// Sets up the gateway registrations database.
///
/// With `db_key`, the database is encrypted with SQLCipher using the raw 256-bit key, and an
/// existing plaintext database is encrypted in place first.
pub async fn setup_fs_gateways_storage<P: AsRef<Path>>(
    db_path: P,
    db_key: Option<&nym_sqlcipher::DatabaseKey>,
) -> Result<OnDiskGatewaysDetails, ClientCoreError> {
    trace!("setting up gateways details storage");
    OnDiskGatewaysDetails::init(db_path, db_key)
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub fn default_query_dkg_client_from_config(
    config: &Config,
) -> Result<QueryHttpRpcNyxdClient, ClientCoreError> {
    let nyxd_url = config
        .get_validator_endpoints()
        .pop()
        .ok_or(ClientCoreError::RpcClientMissingUrl)?;

    default_query_dkg_client(nyxd_url)
}

pub fn default_query_dkg_client(nyxd_url: Url) -> Result<QueryHttpRpcNyxdClient, ClientCoreError> {
    let details = nym_network_defaults::NymNetworkDetails::new_from_env();
    let client_config = nyxd::Config::try_from_nym_network_details(&details)
        .map_err(|source| ClientCoreError::InvalidNetworkDetails { source })?;
    // overwrite env configuration with config URLs

    QueryHttpRpcNyxdClient::connect(client_config, nyxd_url.as_str())
        .map_err(|source| ClientCoreError::RpcClientCreationFailure { source })
}
