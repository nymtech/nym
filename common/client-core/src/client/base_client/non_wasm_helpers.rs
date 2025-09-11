// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::replies::reply_storage::{fs_backend, CombinedReplyStorage, ReplyStorageBackend},
    config,
    config::Config,
    error::ClientCoreError,
};
use nym_bandwidth_controller::BandwidthController;
use nym_client_core_gateways_storage::OnDiskGatewaysDetails;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_validator_client::{nyxd, QueryHttpRpcNyxdClient};
use std::{io, path::Path};
use time::OffsetDateTime;
use tracing::{error, info, trace};
use url::Url;

async fn setup_fresh_backend<P: AsRef<Path>>(
    db_path: P,
    surb_config: &config::ReplySurbs,
) -> Result<fs_backend::Backend, ClientCoreError> {
    info!("Creating fresh surb database");
    let mut storage_backend = match fs_backend::Backend::init(db_path).await {
        Ok(backend) => backend,
        Err(err) => {
            error!("setup_fresh_backend: Failed to setup persistent storage backend for our reply needs: {err}");
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

    tokio::fs::rename(db_path, &renamed).await.inspect_err(|_| {
        error!(
            "Failed to rename corrupt database file: {} to {}",
            db_path.display(),
            renamed.display()
        );
    })
}

pub async fn setup_fs_reply_surb_backend<P: AsRef<Path>>(
    db_path: P,
    surb_config: &config::ReplySurbs,
) -> Result<fs_backend::Backend, ClientCoreError> {
    // if the database file doesnt exist, initialise fresh storage, otherwise attempt to load
    // the existing one
    let db_path = db_path.as_ref();
    if db_path.exists() {
        info!("Loading existing surb database");
        match fs_backend::Backend::try_load(db_path).await {
            Ok(backend) => Ok(backend),
            Err(err) => {
                error!("setup_fs_reply_surb_backend: Failed to setup persistent storage backend for our reply needs: {err}. We're going to create a fresh database instead. This behaviour might change in the future");
                archive_corrupted_database(db_path).await?;
                setup_fresh_backend(db_path, surb_config).await
            }
        }
    } else {
        setup_fresh_backend(db_path, surb_config).await
    }
}

pub async fn setup_fs_gateways_storage<P: AsRef<Path>>(
    db_path: P,
) -> Result<OnDiskGatewaysDetails, ClientCoreError> {
    trace!("setting up gateways details storage");
    OnDiskGatewaysDetails::init(db_path)
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub fn create_bandwidth_controller_with_urls<St: CredentialStorage>(
    nyxd_url: Url,
    storage: St,
) -> Result<BandwidthController<QueryHttpRpcNyxdClient, St>, ClientCoreError> {
    let client = default_query_dkg_client(nyxd_url)?;

    Ok(BandwidthController::new(storage, client))
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
