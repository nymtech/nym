// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::GatewayError;
use crate::node::storage::PersistentStorage;
use nym_crypto::asymmetric::{encryption, identity};
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use nym_pemstore::KeyPairPath;
use nym_sphinx::addressing::clients::Recipient;
use nym_types::gateway::{
    GatewayIpPacketRouterDetails, GatewayNetworkRequesterDetails, GatewayNodeDetailsResponse,
};
use std::path::Path;

fn display_maybe_path<P: AsRef<Path>>(path: Option<P>) -> String {
    path.as_ref()
        .map(|p| p.as_ref().display().to_string())
        .unwrap_or_default()
}

fn display_path<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().display().to_string()
}

pub(crate) async fn node_details(
    config: &Config,
) -> Result<GatewayNodeDetailsResponse, GatewayError> {
    let gateway_identity_public_key: identity::PublicKey = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;

    let gateway_sphinx_public_key: encryption::PublicKey = load_public_key(
        &config.storage_paths.keys.public_sphinx_key_file,
        "gateway sphinx",
    )?;

    let network_requester =
        if let Some(nr_cfg_path) = &config.storage_paths.network_requester_config {
            let cfg = load_network_requester_config(&config.gateway.id, nr_cfg_path).await?;

            let nr_identity_public_key: identity::PublicKey = load_public_key(
                &cfg.storage_paths.common_paths.keys.public_identity_key_file,
                "network requester identity",
            )?;

            let nr_encryption_key: encryption::PublicKey = load_public_key(
                &cfg.storage_paths
                    .common_paths
                    .keys
                    .public_encryption_key_file,
                "network requester encryption",
            )?;

            let address = Recipient::new(
                nr_identity_public_key,
                nr_encryption_key,
                gateway_identity_public_key,
            );

            Some(GatewayNetworkRequesterDetails {
                enabled: config.network_requester.enabled,
                identity_key: nr_identity_public_key.to_base58_string(),
                encryption_key: nr_encryption_key.to_base58_string(),
                open_proxy: cfg.network_requester.open_proxy,
                enabled_statistics: cfg.network_requester.enabled_statistics,
                address: address.to_string(),
                config_path: display_path(nr_cfg_path),
            })
        } else {
            None
        };

    let ip_packet_router = if let Some(nr_cfg_path) = &config.storage_paths.ip_packet_router_config
    {
        let cfg = load_ip_packet_router_config(&config.gateway.id, nr_cfg_path).await?;

        let nr_identity_public_key: identity::PublicKey = load_public_key(
            &cfg.storage_paths.common_paths.keys.public_identity_key_file,
            "ip packet router identity",
        )?;

        let nr_encryption_key: encryption::PublicKey = load_public_key(
            &cfg.storage_paths
                .common_paths
                .keys
                .public_encryption_key_file,
            "ip packet router encryption",
        )?;

        let address = Recipient::new(
            nr_identity_public_key,
            nr_encryption_key,
            gateway_identity_public_key,
        );

        Some(GatewayIpPacketRouterDetails {
            enabled: config.ip_packet_router.enabled,
            identity_key: nr_identity_public_key.to_base58_string(),
            encryption_key: nr_encryption_key.to_base58_string(),
            address: address.to_string(),
            config_path: display_path(nr_cfg_path),
        })
    } else {
        None
    };

    Ok(GatewayNodeDetailsResponse {
        identity_key: gateway_identity_public_key.to_base58_string(),
        sphinx_key: gateway_sphinx_public_key.to_base58_string(),
        bind_address: config.gateway.listening_address.to_string(),
        mix_port: config.gateway.mix_port,
        clients_port: config.gateway.clients_port,
        config_path: display_maybe_path(config.save_path.as_ref()),
        data_store: display_path(&config.storage_paths.clients_storage),
        network_requester,
        ip_packet_router,
    })
}

pub(crate) async fn load_network_requester_config<P: AsRef<Path>>(
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

pub(crate) async fn load_ip_packet_router_config<P: AsRef<Path>>(
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

fn read_network_requester_config<P: AsRef<Path>>(
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

fn read_ip_packet_router_config<P: AsRef<Path>>(
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

pub(crate) fn load_keypair<T: PemStorableKeyPair>(
    paths: KeyPairPath,
    name: impl Into<String>,
) -> Result<T, GatewayError> {
    nym_pemstore::load_keypair(&paths).map_err(|err| GatewayError::KeyPairLoadFailure {
        keys: name.into(),
        paths,
        err,
    })
}

pub(crate) fn load_public_key<T, P, S>(path: P, name: S) -> Result<T, GatewayError>
where
    T: PemStorableKey,
    P: AsRef<Path>,
    S: Into<String>,
{
    nym_pemstore::load_key(path.as_ref()).map_err(|err| GatewayError::PublicKeyLoadFailure {
        key: name.into(),
        path: path.as_ref().to_path_buf(),
        err,
    })
}

/// Loads identity keys stored on disk
pub(crate) fn load_identity_keys(config: &Config) -> Result<identity::KeyPair, GatewayError> {
    let identity_paths = KeyPairPath::new(
        config.storage_paths.keys.private_identity_key(),
        config.storage_paths.keys.public_identity_key(),
    );
    load_keypair(identity_paths, "gateway identity")
}

/// Loads Sphinx keys stored on disk
pub(crate) fn load_sphinx_keys(config: &Config) -> Result<encryption::KeyPair, GatewayError> {
    let sphinx_paths = KeyPairPath::new(
        config.storage_paths.keys.private_encryption_key(),
        config.storage_paths.keys.public_encryption_key(),
    );
    load_keypair(sphinx_paths, "gateway sphinx")
}
