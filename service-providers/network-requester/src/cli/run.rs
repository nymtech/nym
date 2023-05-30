// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_upgrade_v1_1_13_config;
use crate::{
    cli::{override_config, OverrideConfig},
    config::Config,
    error::NetworkRequesterError,
};
use clap::Args;
use nym_bin_common::version_checker;
use nym_config::NymConfig;
use nym_sphinx::addressing::clients::Recipient;

const ENABLE_STATISTICS: &str = "enable-statistics";

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnet-client we want to run.
    #[clap(long)]
    id: String,

    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[clap(long)]
    open_proxy: bool,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enable_statistics: bool,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[clap(long)]
    statistics_recipient: Option<String>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[clap(long)]
    enabled_credentials_mode: Option<bool>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hide = true)]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[clap(long, hide = true)]
    no_cover: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: None,
            fastmode: run_config.fastmode,
            no_cover: run_config.no_cover,
            nyxd_urls: None,
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
        log::warn!(
            "The native-client binary has different version than what is specified \
            in config file! {} and {}",
            binary_version,
            config_version
        );
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            log::info!(
                "but they are still semver compatible. \
                However, consider running the `upgrade` command"
            );
            true
        } else {
            log::error!(
                "and they are semver incompatible! - \
                please run the `upgrade` command before attempting `run` again"
            );
            false
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), NetworkRequesterError> {
    if args.open_proxy {
        println!(
            "\n\nYOU HAVE STARTED IN 'OPEN PROXY' MODE. ANYONE WITH YOUR CLIENT ADDRESS \
                CAN MAKE REQUESTS FROM YOUR MACHINE. PLEASE QUIT IF YOU DON'T UNDERSTAND WHAT \
                YOU'RE DOING.\n\n"
        );
    }

    if args.enable_statistics {
        println!(
            "\n\nTHE NETWORK REQUESTER STATISTICS ARE ENABLED. IT WILL COLLECT AND SEND \
                ANONYMIZED STATISTICS TO A CENTRAL SERVER. PLEASE QUIT IF YOU DON'T WANT \
                THIS TO HAPPEN AND START WITHOUT THE {ENABLE_STATISTICS} FLAG .\n\n"
        );
    }

    let id = &args.id;

    // in case we're using old config, try to upgrade it
    // (if we're using the current version, it's a no-op)
    try_upgrade_v1_1_13_config(id)?;

    let mut config = match Config::load_from_file(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!(
                "Failed to load config for {}. \
                Are you sure you have run `init` before? (Error was: {err})",
                id
            );
            return Err(NetworkRequesterError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(NetworkRequesterError::ConfigValidationFailure);
    }

    let override_config_fields = OverrideConfig::from(args.clone());
    config = override_config(config, override_config_fields);

    if config.get_base_mut().set_empty_fields_to_defaults() {
        log::warn!(
            "Some of the core config options were left unset. \
            The default values are going to get used instead."
        );
    }

    if !version_check(&config) {
        log::error!("Failed the local version check");
        return Err(NetworkRequesterError::FailedLocalVersionCheck);
    }

    // TODO: consider incorporating statistics_recipient, open_proxuy and enable_statistics in
    // `Config`.

    let stats_provider_addr = args
        .statistics_recipient
        .as_ref()
        .map(Recipient::try_from_base58_string)
        .transpose()
        .unwrap_or(None);

    log::info!("Starting socks5 service provider");
    let server = crate::core::NRServiceProviderBuilder::new(
        config,
        args.open_proxy,
        args.enable_statistics,
        stats_provider_addr,
    )
    .await;
    server.run_service_provider().await
}
