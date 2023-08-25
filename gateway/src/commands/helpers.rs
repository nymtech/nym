// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::upgrade_helpers;
use crate::config::default_config_filepath;
use crate::config::Config;
use crate::error::GatewayError;
use log::error;
use nym_bin_common::version_checker;
use nym_config::OptionalSet;
use nym_network_defaults::var_names::NYXD;
use nym_network_defaults::var_names::{BECH32_PREFIX, NYM_API, STATISTICS_SERVICE_DOMAIN_ADDRESS};
use nym_validator_client::nyxd::AccountId;
use std::net::IpAddr;
use std::path::PathBuf;

// Configuration that can be overridden.
#[derive(Default)]
pub(crate) struct OverrideConfig {
    pub(crate) host: Option<IpAddr>,
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
}

impl OverrideConfig {
    pub(crate) fn do_override(self, mut config: Config) -> Result<Config, GatewayError> {
        config = config
            .with_optional(Config::with_listening_address, self.host)
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
            );

        if config.network_requester.enabled
            && config.storage_paths.network_requester_config.is_none()
        {
            Ok(config.with_default_network_requester_config_path())
        } else {
            Ok(config)
        }
    }
}

/// Ensures that a given bech32 address is valid
pub(crate) fn ensure_correct_bech32_prefix(address: &AccountId) -> Result<(), GatewayError> {
    let expected_prefix = std::env::var(BECH32_PREFIX).expect("bech32 prefix not set");
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
    upgrade_helpers::try_upgrade_v1_1_20_config(id)?;

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

pub(crate) fn initialise_local_network_requester() -> Result<(), GatewayError> {
    println!("here we're initialising network requester");
    Ok(())
}
