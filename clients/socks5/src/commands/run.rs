// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::{config::Config, NymClient},
    commands::{override_config, OverrideConfig},
    error::Socks5ClientError,
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

    /// Specifies whether this client is going to use an anonymous sender tag for communication with the service provider.
    /// While this is going to hide its actual address information, it will make the actual communication
    /// slower and consume nearly double the bandwidth as it will require sending reply SURBs.
    ///
    /// Note that some service providers might not support this.
    #[clap(long)]
    use_anonymous_sender_tag: bool,

    /// Address of the socks5 provider to send messages to.
    #[clap(long)]
    provider: Option<String>,

    /// Id of the gateway we want to connect to. If overridden, it is user's responsibility to
    /// ensure prior registration happened
    #[clap(long)]
    gateway: Option<String>,

    /// Comma separated list of rest endpoints of the nymd validators
    #[clap(long)]
    nymd_validators: Option<String>,

    /// Comma separated list of rest endpoints of the API validators
    #[clap(long)]
    api_validators: Option<String>,

    /// Port for the socket to listen on
    #[clap(short, long)]
    port: Option<u16>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[cfg(feature = "coconut")]
    #[clap(long)]
    enabled_credentials_mode: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nymd_validators: run_config.nymd_validators,
            api_validators: run_config.api_validators,
            port: run_config.port,
            use_anonymous_sender_tag: run_config.use_anonymous_sender_tag,
            fastmode: false,

            #[cfg(feature = "coconut")]
            enabled_credentials_mode: run_config.enabled_credentials_mode,
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

pub(crate) async fn execute(args: &Run) -> Result<(), Socks5ClientError> {
    let id = &args.id;

    let mut config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return Err(Socks5ClientError::FailedToLoadConfig(id.to_string()));
        }
    };

    let override_config_fields = OverrideConfig::from(args.clone());
    config = override_config(config, override_config_fields);

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(Socks5ClientError::FailedLocalVersionCheck);
    }

    NymClient::new(config).run_forever().await
}
