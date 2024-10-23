// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::GatewayError;

use nym_crypto::asymmetric::encryption;
use nym_gateway_stats_storage::PersistentStatsStorage;
use nym_gateway_storage::PersistentStorage;
use nym_pemstore::traits::PemStorableKeyPair;
use nym_pemstore::KeyPairPath;

use std::path::Path;

pub async fn load_network_requester_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_network_requester::Config, GatewayError> {
    let path = path.as_ref();
    if let Ok(cfg) = read_network_requester_config(id, path) {
        return Ok(cfg);
    }

    nym_network_requester::config::helpers::try_upgrade_config(path).await?;
    read_network_requester_config(id, path)
}

pub async fn load_ip_packet_router_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_ip_packet_router::Config, GatewayError> {
    let path = path.as_ref();
    if let Ok(cfg) = read_ip_packet_router_config(id, path) {
        return Ok(cfg);
    }

    nym_ip_packet_router::config::helpers::try_upgrade_config(path).await?;
    read_ip_packet_router_config(id, path)
}

pub fn read_network_requester_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_network_requester::Config, GatewayError> {
    let path = path.as_ref();
    nym_network_requester::Config::read_from_toml_file(path).map_err(|err| {
        GatewayError::NetworkRequesterConfigLoadFailure {
            id: id.to_string(),
            path: path.to_path_buf(),
            source: err,
        }
    })
}

pub fn read_ip_packet_router_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_ip_packet_router::Config, GatewayError> {
    let path = path.as_ref();
    nym_ip_packet_router::Config::read_from_toml_file(path).map_err(|err| {
        GatewayError::IpPacketRouterConfigLoadFailure {
            id: id.to_string(),
            path: path.to_path_buf(),
            source: err,
        }
    })
}

pub(crate) async fn initialise_main_storage(
    config: &Config,
) -> Result<PersistentStorage, GatewayError> {
    let path = &config.storage_paths.clients_storage;
    let retrieval_limit = config.debug.message_retrieval_limit;

    Ok(PersistentStorage::init(path, retrieval_limit).await?)
}

pub(crate) async fn initialise_stats_storage(
    config: &Config,
) -> Result<PersistentStatsStorage, GatewayError> {
    let path = &config.storage_paths.stats_storage;

    Ok(PersistentStatsStorage::init(path).await?)
}

pub fn load_keypair<T: PemStorableKeyPair>(
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<T, GatewayError> {
    nym_pemstore::load_keypair(&paths).map_err(|err| GatewayError::KeyPairLoadFailure {
        keys: name.into(),
        paths,
        err,
    })
}

/// Loads Sphinx keys stored on disk
pub(crate) fn load_sphinx_keys(config: &Config) -> Result<encryption::KeyPair, GatewayError> {
    let sphinx_paths = KeyPairPath::new(
        config.storage_paths.keys.private_encryption_key(),
        config.storage_paths.keys.public_encryption_key(),
    );
    load_keypair(sphinx_paths, "gateway sphinx")
}
