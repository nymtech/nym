// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::{config::Config, NymClient},
    commands::{override_config, OverrideConfig},
};

use clap::Args;
use config::NymConfig;
use log::*;
use version_checker::is_minor_version_compatible;

#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnet-client we want to run.
    #[clap(long)]
    id: String,

    /// Custom path to the nym-mixnet-client configuration file
    #[clap(long)]
    config: Option<String>,

    /// Address of the socks5 provider to send messages to.
    #[clap(long)]
    provider: Option<String>,

    /// Id of the gateway we want to connect to. If overridden, it is user's responsibility to
    /// ensure prior registration happened
    #[clap(long)]
    gateway: Option<String>,

    /// Comma separated list of rest endpoints of the validators
    #[clap(long)]
    validators: Option<String>,

    /// Port for the socket to listen on
    #[clap(short, long)]
    port: Option<u16>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement. If this value is set, --eth-endpoint and
    /// --eth-private-key don't need to be set.
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long, conflicts_with_all = &["eth-endpoint", "eth-private-key"])]
    enabled_credentials_mode: bool,

    /// URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20
    /// tokens. If you don't want to set this value, use --enabled-credentials-mode instead
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    eth_endpoint: Option<String>,

    /// Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't
    /// want to set this value, use --enabled-credentials-mode instead
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    eth_private_key: Option<String>,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            validators: run_config.validators,
            port: run_config.port,
            fastmode: false,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            enabled_credentials_mode: run_config.enabled_credentials_mode,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_private_key: run_config.eth_private_key,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_endpoint: run_config.eth_endpoint,
        }
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_base().get_version();
    if binary_version == config_version {
        true
    } else {
        warn!(
            "The mixnode binary has different version than what is specified in config file! {} and {}",
            binary_version, config_version
        );
        if is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    }
}

pub(crate) async fn execute(args: &Run) {
    let id = &args.id;

    let mut config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    let override_config_fields = OverrideConfig::from(args.clone());
    config = override_config(config, override_config_fields);

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    NymClient::new(config).run_forever().await;
}
