// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::upgrade_helpers;
use log::{error, info};
use nym_config::{save_formatted_config_to_file, OptionalSet};
use nym_crypto::asymmetric::identity;
use nym_gateway::config::default_config_filepath;
use nym_gateway::config::persistence::paths::{
    default_ip_packet_router_data_dir, default_network_requester_data_dir,
};
use nym_gateway::config::Config;
use nym_gateway::error::GatewayError;
use nym_gateway::helpers::{
    override_ip_packet_router_config, override_network_requester_config,
    OverrideIpPacketRouterConfig, OverrideNetworkRequesterConfig,
};
use nym_network_defaults::mainnet;
use nym_network_defaults::var_names::NYXD;
use nym_network_defaults::var_names::{BECH32_PREFIX, NYM_API, STATISTICS_SERVICE_DOMAIN_ADDRESS};

use nym_network_requester::{
    generate_new_client_keys, set_active_gateway, setup_fs_gateways_storage, setup_gateway,
    GatewaySetup, OnDiskKeys,
};
use nym_types::gateway::{GatewayIpPacketRouterDetails, GatewayNetworkRequesterDetails};
use nym_validator_client::nyxd::AccountId;
use rand::rngs::OsRng;
use std::net::IpAddr;
use std::path::PathBuf;

// Configuration that can be overridden.
#[derive(Default)]
pub(crate) struct OverrideConfig {
    pub(crate) listening_address: Option<IpAddr>,
    pub(crate) public_ips: Option<Vec<IpAddr>>,
    pub(crate) hostname: Option<String>,

    pub(crate) mix_port: Option<u16>,
    pub(crate) clients_port: Option<u16>,
    pub(crate) datastore: Option<PathBuf>,
    pub(crate) enabled_statistics: Option<bool>,
    pub(crate) statistics_service_url: Option<url::Url>,
    pub(crate) nym_apis: Option<Vec<url::Url>>,
    pub(crate) mnemonic: Option<bip39::Mnemonic>,
    pub(crate) nyxd_urls: Option<Vec<url::Url>>,
    pub(crate) only_coconut_credentials: Option<bool>,
    pub(crate) with_network_requester: Option<bool>,
    pub(crate) with_ip_packet_router: Option<bool>,
}

impl OverrideConfig {
    pub(crate) fn do_override(self, mut config: Config) -> Result<Config, GatewayError> {
        config = config
            .with_optional(Config::with_hostname, self.hostname)
            .with_optional(Config::with_public_ips, self.public_ips)
            .with_optional(Config::with_listening_address, self.listening_address)
            .with_optional(Config::with_mix_port, self.mix_port)
            .with_optional(Config::with_clients_port, self.clients_port)
            .with_optional_custom_env(
                Config::with_custom_nym_apis,
                self.nym_apis,
                NYM_API,
                nym_config::parse_urls,
            )
            .with_optional(Config::with_enabled_statistics, self.enabled_statistics)
            .with_optional_env(
                Config::with_custom_statistics_service_url,
                self.statistics_service_url,
                STATISTICS_SERVICE_DOMAIN_ADDRESS,
            )
            .with_optional(Config::with_custom_persistent_store, self.datastore)
            .with_optional(Config::with_cosmos_mnemonic, self.mnemonic)
            .with_optional_custom_env(
                Config::with_custom_validator_nyxd,
                self.nyxd_urls,
                NYXD,
                nym_config::parse_urls,
            )
            .with_optional(
                Config::with_only_coconut_credentials,
                self.only_coconut_credentials,
            )
            .with_optional(
                Config::with_enabled_network_requester,
                self.with_network_requester,
            )
            .with_optional(
                Config::with_enabled_ip_packet_router,
                self.with_ip_packet_router,
            );

        if config.network_requester.enabled
            && config.storage_paths.network_requester_config.is_none()
        {
            config = config.with_default_network_requester_config_path();
        }

        if config.ip_packet_router.enabled && config.storage_paths.ip_packet_router_config.is_none()
        {
            config = config.with_default_ip_packet_router_config_path();
        }

        Ok(config)
    }
}

pub(crate) fn try_override_config<O: Into<OverrideConfig>>(
    config: Config,
    override_args: O,
) -> Result<Config, GatewayError> {
    override_args.into().do_override(config)
}

/// Ensures that a given bech32 address is valid
pub(crate) fn ensure_correct_bech32_prefix(address: &AccountId) -> Result<(), GatewayError> {
    let expected_prefix =
        std::env::var(BECH32_PREFIX).unwrap_or(mainnet::BECH32_PREFIX.to_string());
    let actual_prefix = address.prefix();

    if expected_prefix != actual_prefix {
        return Err(GatewayError::InvalidBech32AccountPrefix {
            account: address.to_owned(),
            expected_prefix,
            actual_prefix: actual_prefix.to_owned(),
        });
    }

    Ok(())
}

pub(crate) fn try_load_current_config(id: &str) -> Result<Config, GatewayError> {
    upgrade_helpers::try_upgrade_config(id)?;

    Config::read_from_default_path(id).map_err(|err| {
        error!(
            "Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})",
        );
        GatewayError::ConfigLoadFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        }
    })
}

fn make_nr_id(gateway_id: &str) -> String {
    format!("{gateway_id}-network-requester")
}

fn make_ip_id(gateway_id: &str) -> String {
    format!("{gateway_id}-ip-packet-router")
}

pub(crate) async fn initialise_local_network_requester(
    gateway_config: &Config,
    opts: OverrideNetworkRequesterConfig,
    identity: identity::PublicKey,
) -> Result<GatewayNetworkRequesterDetails, GatewayError> {
    info!("initialising network requester...");
    let Some(nr_cfg_path) = gateway_config.storage_paths.network_requester_config() else {
        return Err(GatewayError::UnspecifiedNetworkRequesterConfig);
    };

    let id = &gateway_config.gateway.id;
    let nr_id = make_nr_id(id);
    let nr_data_dir = default_network_requester_data_dir(id);
    let mut nr_cfg = nym_network_requester::Config::new(&nr_id).with_data_directory(nr_data_dir);
    nr_cfg = override_network_requester_config(nr_cfg, Some(opts));

    let key_store = OnDiskKeys::new(nr_cfg.storage_paths.common_paths.keys.clone());
    let details_store =
        setup_fs_gateways_storage(&nr_cfg.storage_paths.common_paths.gateway_registrations).await?;

    // if this is a first time client with this particular id is initialised, generated long-term keys
    if !nr_cfg_path.exists() {
        let mut rng = OsRng;
        generate_new_client_keys(&mut rng, &key_store).await?;
    }

    // gateway setup here is way simpler as we're 'connecting' to ourselves
    let init_res = setup_gateway(
        GatewaySetup::new_inbuilt(identity),
        &key_store,
        &details_store,
    )
    .await?;
    set_active_gateway(&details_store, &init_res.gateway_id().to_base58_string()).await?;

    let address = init_res.client_address();

    if let Err(err) = save_formatted_config_to_file(&nr_cfg, nr_cfg_path) {
        log::error!("Failed to save the network requester config file: {err}");
        return Err(GatewayError::ConfigSaveFailure {
            id: nr_id,
            path: nr_cfg_path.to_path_buf(),
            source: err,
        });
    } else {
        eprintln!(
            "Saved network requester configuration file to {}",
            nr_cfg_path.display()
        )
    }

    Ok(GatewayNetworkRequesterDetails {
        enabled: gateway_config.network_requester.enabled,
        identity_key: address.identity().to_string(),
        encryption_key: address.encryption_key().to_string(),
        open_proxy: nr_cfg.network_requester.open_proxy,
        enabled_statistics: nr_cfg.network_requester.enabled_statistics,
        address: address.to_string(),
        config_path: nr_cfg_path.display().to_string(),
    })
}

pub(crate) async fn initialise_local_ip_packet_router(
    gateway_config: &Config,
    opts: OverrideIpPacketRouterConfig,
    identity: identity::PublicKey,
) -> Result<GatewayIpPacketRouterDetails, GatewayError> {
    info!("initialising ip packet router...");
    let Some(ip_cfg_path) = gateway_config.storage_paths.ip_packet_router_config() else {
        return Err(GatewayError::UnspecifiedIpPacketRouterConfig);
    };

    let id = &gateway_config.gateway.id;
    let ip_id = make_ip_id(id);
    let ip_data_dir = default_ip_packet_router_data_dir(id);
    let mut ip_cfg = nym_ip_packet_router::Config::new(&ip_id).with_data_directory(ip_data_dir);
    ip_cfg = override_ip_packet_router_config(ip_cfg, Some(opts));

    let key_store = OnDiskKeys::new(ip_cfg.storage_paths.common_paths.keys.clone());
    let details_store =
        setup_fs_gateways_storage(&ip_cfg.storage_paths.common_paths.gateway_registrations).await?;

    // if this is a first time client with this particular id is initialised, generated long-term keys
    if !ip_cfg_path.exists() {
        let mut rng = OsRng;
        generate_new_client_keys(&mut rng, &key_store).await?;
    }

    // gateway setup here is way simpler as we're 'connecting' to ourselves
    let init_res = setup_gateway(
        GatewaySetup::new_inbuilt(identity),
        &key_store,
        &details_store,
    )
    .await?;
    set_active_gateway(&details_store, &init_res.gateway_id().to_base58_string()).await?;

    let address = init_res.client_address();

    if let Err(err) = save_formatted_config_to_file(&ip_cfg, ip_cfg_path) {
        log::error!("Failed to save the ip packet router config file: {err}");
        return Err(GatewayError::ConfigSaveFailure {
            id: ip_id,
            path: ip_cfg_path.to_path_buf(),
            source: err,
        });
    } else {
        eprintln!(
            "Saved ip packet router configuration file to {}",
            ip_cfg_path.display()
        )
    }

    Ok(GatewayIpPacketRouterDetails {
        enabled: gateway_config.ip_packet_router.enabled,
        identity_key: address.identity().to_string(),
        encryption_key: address.encryption_key().to_string(),
        address: address.to_string(),
        config_path: ip_cfg_path.display().to_string(),
    })
}
