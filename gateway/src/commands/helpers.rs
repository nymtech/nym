// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::upgrade_helpers;
use crate::config::default_config_filepath;
use crate::config::persistence::paths::{
    default_ip_packet_router_data_dir, default_network_requester_data_dir,
};
use crate::config::Config;
use crate::error::GatewayError;
use log::{error, info};
use nym_bin_common::version_checker;
use nym_config::{save_formatted_config_to_file, OptionalSet};
use nym_crypto::asymmetric::identity;
use nym_network_defaults::mainnet;
use nym_network_defaults::var_names::NYXD;
use nym_network_defaults::var_names::{BECH32_PREFIX, NYM_API, STATISTICS_SERVICE_DOMAIN_ADDRESS};
use nym_network_requester::config::BaseClientConfig;
use nym_network_requester::{
    setup_gateway, GatewaySelectionSpecification, GatewaySetup, OnDiskGatewayDetails, OnDiskKeys,
};
use nym_types::gateway::{GatewayIpPacketRouterDetails, GatewayNetworkRequesterDetails};
use nym_validator_client::nyxd::AccountId;
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
            Ok(config.with_default_network_requester_config_path())
        } else if config.ip_packet_router.enabled
            && config.storage_paths.ip_packet_router_config.is_none()
        {
            Ok(config.with_default_ip_packet_router_config_path())
        } else {
            Ok(config)
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct OverrideNetworkRequesterConfig {
    pub(crate) fastmode: bool,
    pub(crate) no_cover: bool,
    pub(crate) medium_toggle: bool,

    pub(crate) open_proxy: Option<bool>,
    pub(crate) enable_exit_policy: Option<bool>,

    pub(crate) enable_statistics: Option<bool>,
    pub(crate) statistics_recipient: Option<String>,
}

#[derive(Default, Debug)]
pub(crate) struct OverrideIpPacketRouterConfig {
    // TODO
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

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
pub(crate) fn ensure_config_version_compatibility(cfg: &Config) -> Result<(), GatewayError> {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.gateway.version;

    if binary_version == config_version {
        Ok(())
    } else if version_checker::is_minor_version_compatible(binary_version, config_version) {
        log::warn!(
            "The gateway binary has different version than what is specified in config file! {binary_version} and {config_version}. \
             But, they are still semver compatible. However, consider running the `upgrade` command.");
        Ok(())
    } else {
        log::error!(
            "The gateway binary has different version than what is specified in config file! {binary_version} and {config_version}. \
             And they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
        Err(GatewayError::LocalVersionCheckFailure {
            binary_version: binary_version.to_owned(),
            config_version: config_version.to_owned(),
        })
    }
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

// NOTE: make sure this is in sync with service-providers/network-requester/src/cli/mod.rs::override_config
pub(crate) fn override_network_requester_config(
    mut cfg: nym_network_requester::Config,
    opts: Option<OverrideNetworkRequesterConfig>,
) -> nym_network_requester::Config {
    let Some(opts) = opts else { return cfg };

    // as of 12.09.23 the below is true (not sure how this comment will rot in the future)
    // medium_toggle:
    // - sets secondary packet size to 16kb
    // - disables poisson distribution of the main traffic stream
    // - sets the cover traffic stream to 1 packet / 5s (on average)
    // - disables per hop delay
    //
    // fastmode (to be renamed to `fast-poisson`):
    // - sets average per hop delay to 10ms
    // - sets the cover traffic stream to 1 packet / 2000s (on average); for all intents and purposes it disables the stream
    // - sets the poisson distribution of the main traffic stream to 4ms, i.e. 250 packets / s on average
    //
    // no_cover:
    // - disables poisson distribution of the main traffic stream
    // - disables the secondary cover traffic stream

    // disable poisson rate in the BASE client if the NR option is enabled
    if cfg.network_requester.disable_poisson_rate {
        cfg.set_no_poisson_process();
    }

    // those should be enforced by `clap` when parsing the arguments
    if opts.medium_toggle {
        assert!(!opts.fastmode);
        assert!(!opts.no_cover);

        cfg.set_medium_toggle();
    }

    cfg.with_base(
        BaseClientConfig::with_high_default_traffic_volume,
        opts.fastmode,
    )
    .with_base(BaseClientConfig::with_disabled_cover_traffic, opts.no_cover)
    .with_optional(
        nym_network_requester::Config::with_open_proxy,
        opts.open_proxy,
    )
    .with_optional(
        nym_network_requester::Config::with_old_allow_list,
        opts.enable_exit_policy.map(|e| !e),
    )
    .with_optional(
        nym_network_requester::Config::with_enabled_statistics,
        opts.enable_statistics,
    )
    .with_optional(
        nym_network_requester::Config::with_statistics_recipient,
        opts.statistics_recipient,
    )
}

// NOTE: make sure this is in sync with service-providers/ip-packet-router/src/cli/mod.rs::override_config
pub(crate) fn override_ip_packet_router_config(
    mut cfg: nym_ip_packet_router::Config,
    opts: Option<OverrideIpPacketRouterConfig>,
) -> nym_ip_packet_router::Config {
    let Some(_opts) = opts else { return cfg };

    // disable poisson rate in the BASE client if the IPR option is enabled
    if cfg.ip_packet_router.disable_poisson_rate {
        log::info!("Disabling poisson rate for ip packet router");
        cfg.set_no_poisson_process();
    }

    cfg
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
        OnDiskGatewayDetails::new(&nr_cfg.storage_paths.common_paths.gateway_details);

    // gateway setup here is way simpler as we're 'connecting' to ourselves
    let init_res = setup_gateway(
        GatewaySetup::New {
            specification: GatewaySelectionSpecification::Custom {
                gateway_identity: identity.to_base58_string(),
                additional_data: Default::default(),
            },
            available_gateways: vec![],
            overwrite_data: false,
        },
        &key_store,
        &details_store,
    )
    .await?;

    let address = init_res.client_address()?;

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
        exit_policy: !nr_cfg.network_requester.use_deprecated_allow_list,
        open_proxy: nr_cfg.network_requester.open_proxy,
        enabled_statistics: nr_cfg.network_requester.enabled_statistics,
        address: address.to_string(),
        config_path: nr_cfg_path.display().to_string(),
        allow_list_path: nr_cfg
            .storage_paths
            .allowed_list_location
            .display()
            .to_string(),
        unknown_list_path: nr_cfg
            .storage_paths
            .unknown_list_location
            .display()
            .to_string(),
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
        OnDiskGatewayDetails::new(&ip_cfg.storage_paths.common_paths.gateway_details);

    // gateway setup here is way simpler as we're 'connecting' to ourselves
    let init_res = setup_gateway(
        GatewaySetup::New {
            specification: GatewaySelectionSpecification::Custom {
                gateway_identity: identity.to_base58_string(),
                additional_data: Default::default(),
            },
            available_gateways: vec![],
            overwrite_data: false,
        },
        &key_store,
        &details_store,
    )
    .await?;

    let address = init_res.client_address()?;

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
