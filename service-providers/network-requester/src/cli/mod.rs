// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{CommandFactory, Parser, Subcommand};
use nym_bin_common::build_information::BinaryBuildInformation;
use nym_bin_common::completions::{fig_generate, ArgShell};

use crate::{
    config::{BaseConfig, Config},
    error::NetworkRequesterError,
};

mod init;
mod run;

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
        .with_base(BaseConfig::with_high_default_traffic_volume, args.fastmode)
        .with_base(BaseConfig::with_disabled_cover_traffic, args.no_cover)
        .with_optional_custom_env_ext(
            BaseConfig::with_custom_nym_apis,
            args.nym_apis,
            nym_network_defaults::var_names::NYM_API,
            config::parse_urls,
        )
        .with_optional_custom_env_ext(
            BaseConfig::with_custom_nyxd,
            args.nyxd_urls,
            nym_network_defaults::var_names::NYXD,
            config::parse_urls,
        )
        .with_optional_ext(
            BaseConfig::with_disabled_credentials,
            args.enabled_credentials_mode.map(|b| !b),
        )
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match &args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
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
