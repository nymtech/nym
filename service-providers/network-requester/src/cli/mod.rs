// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::{
    config::{BaseClientConfig, Config},
    error::NetworkRequesterError,
};
use clap::{CommandFactory, Parser, Subcommand};
use log::info;
use nym_bin_common::build_information::BinaryBuildInformation;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;

mod init;
mod run;
mod sign;

lazy_static::lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
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
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
}

pub(crate) fn override_config(config: Config, args: OverrideConfig) -> Config {
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
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match &args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::Sign(m) => sign::execute(m).await?,
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

fn try_upgrade_v1_1_13_config(id: &str) -> std::io::Result<()> {
    todo!()
    // // explicitly load it as v1.1.13 (which is incompatible with the current, i.e. 1.1.14+)
    // let Ok(old_config) = OldConfigV1_1_13::load_from_file(id) else {
    //     // if we failed to load it, there might have been nothing to upgrade
    //     // or maybe it was an even older file. in either way. just ignore it and carry on with our day
    //     return Ok(());
    // };
    // info!("It seems the client is using <= v1.1.13 config template.");
    // info!("It is going to get updated to the current specification.");
    //
    // let updated: Config = old_config.into();
    // updated.save_to_file(None)
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
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
