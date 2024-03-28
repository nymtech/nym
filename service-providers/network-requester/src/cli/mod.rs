// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::try_upgrade_config_by_id;
use crate::{
    config::{BaseClientConfig, Config},
    error::NetworkRequesterError,
};
use clap::{CommandFactory, Parser, Subcommand};
use log::error;
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;
use nym_client_core::cli_helpers::client_import_credential::CommonClientImportCredentialArgs;
use nym_client_core::cli_helpers::CliClient;
use nym_config::OptionalSet;
use std::sync::OnceLock;

mod add_gateway;
mod build_info;
mod import_credential;
mod init;
mod list_gateways;
mod run;
mod sign;
mod switch_gateway;

pub(crate) struct CliNetworkRequesterClient;

impl CliClient for CliNetworkRequesterClient {
    const NAME: &'static str = "network requester";
    type Error = NetworkRequesterError;
    type Config = Config;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error> {
        try_upgrade_config_by_id(id).await
    }

    async fn try_load_current_config(id: &str) -> Result<Self::Config, Self::Error> {
        try_load_current_config(id).await
    }
}

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[command(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[arg(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[arg(long)]
    pub(crate) no_banner: bool,

    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize a network-requester. Do this first!
    Init(init::Init),

    /// Run the network requester with the provided configuration and optionally override
    /// parameters.
    Run(run::Run),

    /// Sign to prove ownership of this network requester
    Sign(sign::Sign),

    /// Import a pre-generated credential
    ImportCredential(CommonClientImportCredentialArgs),

    /// List all registered with gateways
    ListGateways(list_gateways::Args),

    /// Add new gateway to this client
    AddGateway(add_gateway::Args),

    /// Change the currently active gateway. Note that you must have already registered with the new gateway!
    SwitchGateway(switch_gateway::Args),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    nym_apis: Option<Vec<url::Url>>,
    fastmode: bool,
    no_cover: bool,
    medium_toggle: bool,
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
    enable_exit_policy: Option<bool>,

    open_proxy: Option<bool>,
    enable_statistics: Option<bool>,
    statistics_recipient: Option<String>,
}

// NOTE: make sure this is in sync with `gateway/src/commands/helpers.rs::override_network_requester_config`
pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
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
    if config.network_requester.disable_poisson_rate {
        config.set_no_poisson_process();
    }

    // those should be enforced by `clap` when parsing the arguments
    if args.medium_toggle {
        assert!(!args.fastmode);
        assert!(!args.no_cover);

        config.set_medium_toggle();
    }

    config
        .with_base(
            BaseClientConfig::with_high_default_traffic_volume,
            args.fastmode,
        )
        .with_base(BaseClientConfig::with_disabled_cover_traffic, args.no_cover)
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nym_apis,
            args.nym_apis,
            nym_network_defaults::var_names::NYM_API,
            nym_config::parse_urls,
        )
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nyxd,
            args.nyxd_urls,
            nym_network_defaults::var_names::NYXD,
            nym_config::parse_urls,
        )
        .with_optional_base(
            BaseClientConfig::with_disabled_credentials,
            args.enabled_credentials_mode.map(|b| !b),
        )
        .with_optional(Config::with_open_proxy, args.open_proxy)
        .with_optional(
            Config::with_old_allow_list,
            args.enable_exit_policy.map(|e| !e),
        )
        .with_optional(Config::with_enabled_statistics, args.enable_statistics)
        .with_optional(Config::with_statistics_recipient, args.statistics_recipient)
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(&m).await?,
        Commands::Sign(m) => sign::execute(&m).await?,
        Commands::ImportCredential(m) => import_credential::execute(m).await?,
        Commands::ListGateways(args) => list_gateways::execute(args).await?,
        Commands::AddGateway(args) => add_gateway::execute(args).await?,
        Commands::SwitchGateway(args) => switch_gateway::execute(args).await?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

async fn try_load_current_config(id: &str) -> Result<Config, NetworkRequesterError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(NetworkRequesterError::ConfigValidationFailure)
        } else {
            Ok(cfg)
        };
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config_by_id(id).await?;

    let config = match Config::read_from_default_path(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})");
            return Err(NetworkRequesterError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(NetworkRequesterError::ConfigValidationFailure);
    }

    Ok(config)
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.base.client.version;
    if binary_version == config_version {
        true
    } else {
        log::warn!(
            "The native-client binary has different version than what is specified \
            in config file! {binary_version} and {config_version}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
