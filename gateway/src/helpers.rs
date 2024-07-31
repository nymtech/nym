// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::helpers::load_keypair;
use crate::GatewayError;
use nym_config::OptionalSet;
use nym_crypto::asymmetric::{encryption, identity};
use nym_pemstore::traits::PemStorableKey;
use nym_pemstore::KeyPairPath;
use nym_sphinx::addressing::clients::Recipient;
use nym_types::gateway::{
    GatewayIpPacketRouterDetails, GatewayNetworkRequesterDetails, GatewayNodeDetailsResponse,
};
use std::path::Path;
use tracing::info;

pub use crate::node::helpers::{load_ip_packet_router_config, load_network_requester_config};

fn display_maybe_path<P: AsRef<Path>>(path: Option<P>) -> String {
    path.as_ref()
        .map(|p| p.as_ref().display().to_string())
        .unwrap_or_default()
}

fn display_path<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().display().to_string()
}

#[derive(Default)]
pub struct OverrideNetworkRequesterConfig {
    pub fastmode: bool,
    pub no_cover: bool,
    pub medium_toggle: bool,

    pub open_proxy: Option<bool>,
}

#[derive(Default)]
pub struct OverrideIpPacketRouterConfig {
    // TODO
}

pub fn override_network_requester_config(
    mut cfg: nym_network_requester::Config,
    opts: Option<OverrideNetworkRequesterConfig>,
) -> nym_network_requester::Config {
    let Some(opts) = opts else { return cfg };

    // in the old code we had calls to `assert` thus panicking
    #[allow(clippy::expect_used)]
    cfg.base
        .try_apply_traffic_modes(
            cfg.network_requester.disable_poisson_rate,
            opts.medium_toggle,
            opts.fastmode,
            opts.no_cover,
        )
        .expect("failed to apply traffic modes");

    cfg.with_optional(
        nym_network_requester::Config::with_open_proxy,
        opts.open_proxy,
    )
}

// NOTE: make sure this is in sync with service-providers/ip-packet-router/src/cli/mod.rs::override_config
pub fn override_ip_packet_router_config(
    mut cfg: nym_ip_packet_router::Config,
    opts: Option<OverrideIpPacketRouterConfig>,
) -> nym_ip_packet_router::Config {
    let Some(_opts) = opts else { return cfg };

    // disable poisson rate in the BASE client if the IPR option is enabled
    if cfg.ip_packet_router.disable_poisson_rate {
        info!("Disabling poisson rate for ip packet router");
        cfg.set_no_poisson_process();
    }

    cfg
}

pub fn load_public_key<T, P, S>(path: P, name: S) -> Result<T, GatewayError>
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
pub fn load_identity_keys(config: &Config) -> Result<identity::KeyPair, GatewayError> {
    let identity_paths = KeyPairPath::new(
        config.storage_paths.keys.private_identity_key(),
        config.storage_paths.keys.public_identity_key(),
    );
    load_keypair(identity_paths, "gateway identity")
}

pub async fn node_details(config: &Config) -> Result<GatewayNodeDetailsResponse, GatewayError> {
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
